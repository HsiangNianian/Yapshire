use super::*;

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("yapshire-editor-{}", rand::random::<u64>())))
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn maps() -> Maps {
    Maps::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap()
}

#[test]
fn a_drag_is_one_undo_and_a_new_stroke_clears_redo() {
    let mut doc = Document::new(maps().get(MapKind::Town).clone());
    let original = doc.map.clone();
    for cell in line(UVec2::new(1, 0), UVec2::new(24, 8)) {
        doc.paint(4, cell, 20);
    }
    doc.paint(4, UVec2::new(1, 0), 21);
    doc.finish();
    let painted = doc.map.clone();
    assert_ne!(painted, original);
    assert!(doc.dirty());
    assert_eq!(doc.undo.len(), 1);
    doc.undo();
    assert_eq!(doc.map, original);
    assert!(!doc.dirty());
    doc.redo();
    assert_eq!(doc.map, painted);
    doc.undo();
    doc.paint(4, UVec2::new(0, 0), 25);
    doc.finish();
    doc.redo();
    assert_eq!(doc.map.layers[4].data[0], 25);
    assert_eq!(doc.map.layers[4].data[1], original.layers[4].data[1]);
    assert!(!doc.can_redo());
}

#[test]
fn fill_respects_connected_regions_and_does_not_wrap_rows_or_touch_other_layers() {
    let mut doc = Document::new(maps().get(MapKind::Shop).clone());
    let other = doc.map.layers[2].clone();
    let layer = &mut doc.map.layers[4].data;
    layer.fill(10);
    layer[0] = 0;
    layer[30] = 0;
    layer[29] = 0;
    layer[31] = 0;
    doc.fill(4, UVec2::ZERO, 15);
    assert_eq!(doc.map.layers[4].data[0], 15);
    assert_eq!(doc.map.layers[4].data[30], 15);
    assert_eq!(doc.map.layers[4].data[31], 15);
    assert_eq!(doc.map.layers[4].data[29], 0);
    assert_eq!(doc.map.layers[2], other);
    doc.undo();
    assert_eq!(doc.map.layers[4].data[0], 0);
}

#[test]
fn save_roundtrip_preserves_tiled_metadata_flips_hidden_layers_and_backup() {
    let workspace = Workspace::new();
    let maps = maps();
    let mut json = serde_json::to_value(&maps.get(MapKind::Town)).unwrap();
    json["properties"] = serde_json::json!([{"name":"author","type":"string","value":"Rowan"}]);
    json["layers"][4]["custom_layer_field"] = serde_json::json!({"keep":true});
    let mut doc = Document::new(serde_json::from_value(json).unwrap());
    let gid = 0xe000_0007;
    doc.paint(4, UVec2::new(3, 0), gid);
    doc.map.layers[1].visible = false;
    doc.save(&workspace.0, MapKind::Town, &maps).unwrap();
    let path = workspace.0.join("maps/town.tmj");
    let first = std::fs::read(&path).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(value["properties"][0]["value"], "Rowan");
    assert_eq!(value["layers"][4]["custom_layer_field"]["keep"], true);
    assert_eq!(value["layers"][4]["data"][3], gid);
    assert_eq!(value["layers"][1]["visible"], false);
    assert!(value["layers"][4]["id"].is_number());
    assert!(!doc.dirty());
    doc.undo();
    assert!(doc.dirty(), "undo after save must be an unsaved change");
    doc.save(&workspace.0, MapKind::Town, &maps).unwrap();
    assert_eq!(
        std::fs::read(path.with_extension("tmj.bak")).unwrap(),
        first
    );
    let mut reopened = Document::new(maps.get(MapKind::Town).clone());
    reopened.reload(&workspace.0, MapKind::Town, &maps).unwrap();
    assert_eq!(reopened.map, doc.map);
}

#[test]
fn conflicting_external_changes_invalid_data_and_write_failures_preserve_the_draft() {
    let workspace = Workspace::new();
    let maps = maps();
    let mut doc = Document::new(maps.get(MapKind::Town).clone());
    doc.save(&workspace.0, MapKind::Town, &maps).unwrap();
    let path = workspace.0.join("maps/town.tmj");
    doc.paint(4, UVec2::ZERO, 20);
    let draft = doc.map.clone();
    std::fs::write(&path, b"{external invalid file").unwrap();
    assert!(doc.save(&workspace.0, MapKind::Town, &maps).is_err());
    assert!(doc.reload(&workspace.0, MapKind::Town, &maps).is_err());
    assert_eq!(doc.map, draft);
    assert!(doc.dirty());
    assert_eq!(std::fs::read(&path).unwrap(), b"{external invalid file");
    let unwritable = workspace.0.join("file-not-a-folder");
    std::fs::write(&unwritable, b"keep").unwrap();
    assert!(doc.save(&unwritable, MapKind::Town, &maps).is_err());
    assert_eq!(doc.map, draft);
    doc.map.layers[4].data[0] = 99999;
    assert!(doc.save(&workspace.0, MapKind::Town, &maps).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{external invalid file");
}

#[test]
fn two_map_drafts_survive_switches_and_save_and_leave_updates_runtime_and_restart() {
    let workspace = Workspace::new();
    let mut runtime = maps();
    let shipped = runtime.get(MapKind::Town).clone();
    let mut editor = Editor::from_directory(&mut runtime, Ok(workspace.0.clone()));
    editor.doc_mut().paint(4, UVec2::ZERO, 20);
    editor.act(Action::Map(MapKind::Shop), &mut runtime);
    editor.doc_mut().paint(4, UVec2::ZERO, 21);
    assert_eq!(
        runtime.get(MapKind::Town),
        &shipped,
        "drafts must not mutate the runtime"
    );
    assert!(editor.act(Action::Done, &mut runtime).is_none());
    assert!(matches!(editor.dialog, Some(Dialog::Leave(false))));
    editor.act(Action::Cancel, &mut runtime);
    assert!(editor.dirty());
    editor.act(Action::Done, &mut runtime);
    assert_eq!(editor.act(Action::SaveLeave, &mut runtime), Some(false));
    assert!(!editor.dirty());
    assert_eq!(runtime.get(MapKind::Town).layers[4].data[0], 20);
    assert_eq!(runtime.get(MapKind::Shop).layers[4].data[0], 21);
    assert!(workspace.0.join("terrain/ground.tsj").is_file());
    assert!(workspace.0.join("objects/harbor.png").is_file());
    let mut restarted = maps();
    let mut editor = Editor::from_directory(&mut restarted, Ok(workspace.0.clone()));
    assert_eq!(restarted.get(MapKind::Town), runtime.get(MapKind::Town));
    assert_eq!(restarted.get(MapKind::Shop), runtime.get(MapKind::Shop));
    editor.doc_mut().paint(4, UVec2::ZERO, 22);
    editor.act(Action::Quit, &mut restarted);
    assert_eq!(editor.act(Action::Discard, &mut restarted), Some(true));
    assert!(!editor.dirty());
    assert_eq!(editor.doc().map.layers[4].data[0], 20);
}

#[test]
fn saving_one_map_exports_a_complete_pack_without_saving_other_drafts() {
    let workspace = Workspace::new();
    let mut runtime = maps();
    let original_shop = runtime.get(MapKind::Shop).clone();
    let mut editor = Editor::from_directory(&mut runtime, Ok(workspace.0.clone()));
    editor.docs[MapKind::Shop.0].paint(4, UVec2::ZERO, 21);
    editor.doc_mut().paint(4, UVec2::ZERO, 20);
    editor.save(MapKind::Town, &mut runtime).unwrap();
    let exported = yapshire_shared::World::load(&workspace.0).unwrap();
    assert_eq!(exported.maps["yapshire:town"].layers[4].data[0], 20);
    assert_eq!(exported.maps["yapshire:tackle_shop"], original_shop);
    assert!(editor.docs[MapKind::Shop.0].dirty());
    assert_eq!(editor.docs[MapKind::Shop.0].map.layers[4].data[0], 21);
    editor.save(MapKind::Shop, &mut runtime).unwrap();
    assert!(!editor.dirty());
    let exported = yapshire_shared::World::load(&workspace.0).unwrap();
    assert_eq!(exported.maps["yapshire:tackle_shop"].layers[4].data[0], 21);
}

#[test]
fn broken_local_map_falls_back_and_explicit_repair_keeps_corrupt_bytes_as_backup() {
    let workspace = Workspace::new();
    std::fs::create_dir_all(&workspace.0).unwrap();
    std::fs::write(workspace.0.join("town.tmj"), b"broken").unwrap();
    let mut runtime = maps();
    let original = runtime.get(MapKind::Town).clone();
    let mut editor = Editor::from_directory(&mut runtime, Ok(workspace.0.clone()));
    assert_eq!(runtime.get(MapKind::Town), &original);
    assert!(editor.status.to_string().contains("unavailable"));
    editor.doc_mut().paint(4, UVec2::ZERO, 20);
    editor.save(MapKind::Town, &mut runtime).unwrap();
    assert_eq!(
        std::fs::read(workspace.0.join("town.tmj.bak")).unwrap(),
        b"broken"
    );
}

#[test]
fn pointer_mapping_handles_hidpi_letterboxing_zoom_and_map_edges() {
    let viewport = URect::from_corners(UVec2::new(560, 90), UVec2::new(3440, 1710));
    let ui = CANVAS + Vec2::new(32.0 * 3.0 + 1.0, 32.0 * 4.0 + 1.0);
    let point = ui_point(viewport.min.as_vec2() + ui * 2.0, viewport).unwrap();
    assert!(point.abs_diff_eq(ui, 0.001));
    assert!(ui_point(Vec2::ZERO, viewport).is_none());
    let four_x = URect::from_corners(UVec2::new(20, 13), UVec2::new(2540, 1427));
    assert!(
        ui_point(four_x.min.as_vec2() + ui * 1.75, four_x)
            .unwrap()
            .abs_diff_eq(ui, 0.001)
    );
    let mut runtime = maps();
    let workspace = Workspace::new();
    let mut editor = Editor::from_directory(&mut runtime, Ok(workspace.0.clone()));
    assert_eq!(editor.cell_at(point), Some(UVec2::new(51, 4)));
    assert!(editor.cell_at(CANVAS + CANVAS_SIZE).is_none());
    editor.act(Action::Map(MapKind::Shop), &mut runtime);
    editor.act(Action::Zoom(-1), &mut runtime);
    assert!(editor.cell_at(CANVAS + Vec2::new(490.0, 0.0)).is_none());
    assert!(editor.cell_at(CANVAS + Vec2::new(0.0, 280.0)).is_none());
    editor.act(Action::Zoom(2), &mut runtime);
    editor.act(Action::Pan(IVec2::splat(10000)), &mut runtime);
    assert_eq!(editor.offset, UVec2::new(10, 6));
}

#[test]
fn history_is_bounded_and_interpolated_strokes_do_not_skip_cells() {
    let points = line(UVec2::new(4, 3), UVec2::new(25, 12));
    assert_eq!(points.first(), Some(&UVec2::new(4, 3)));
    assert_eq!(points.last(), Some(&UVec2::new(25, 12)));
    for pair in points.windows(2) {
        assert!(
            (pair[1].as_ivec2() - pair[0].as_ivec2())
                .abs()
                .max_element()
                <= 1
        );
    }
    let mut doc = Document::new(maps().get(MapKind::Shop).clone());
    for tile in 1..=150 {
        doc.paint(4, UVec2::ZERO, tile);
        doc.finish();
    }
    assert_eq!(doc.undo.len(), HISTORY_LIMIT);
    for _ in 0..150 {
        doc.undo();
    }
    assert_eq!(doc.map.layers[4].data[0], 50);
}

#[test]
fn closing_a_dirty_editor_keeps_the_window_until_discard_then_uses_two_frame_close() {
    let workspace = Workspace::new();
    let mut maps = maps();
    let mut editor = Editor::from_directory(&mut maps, Ok(workspace.0.clone()));
    editor.doc_mut().paint(4, UVec2::ZERO, 20);
    let mut menu = Menu::default();
    menu.go(Page::Editor);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(editor)
        .insert_resource(maps)
        .insert_resource(menu)
        .init_resource::<crate::settings::Settings>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_message::<MouseWheel>()
        .add_message::<WindowCloseRequested>()
        .add_systems(Update, update);
    let window = app.world_mut().spawn(Window::default()).id();
    app.world_mut()
        .spawn((Camera::default(), game::OuterCamera));
    app.world_mut()
        .write_message(WindowCloseRequested { window });
    app.update();
    assert!(matches!(
        app.world().resource::<Editor>().dialog,
        Some(Dialog::Leave(true))
    ));
    assert!(app.world().get::<ClosingWindow>(window).is_none());
    assert!(app.world().get::<Window>(window).is_some());
    app.world_mut()
        .spawn((Action::Discard, Interaction::Pressed));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    assert!(app.world().get::<ClosingWindow>(window).is_some());
    assert!(
        app.world().get::<Window>(window).is_some(),
        "renderer gets a frame before window removal"
    );
    app.update();
    assert!(app.world().get::<Window>(window).is_none());
    assert!(!app.world().resource::<Editor>().dirty());
}

#[test]
fn terrain_neighbors_and_whole_object_stamps_are_undoable() {
    let maps = maps();
    let mut doc = Document::new(maps.get(MapKind::Town).clone());
    let original = doc.map.clone();
    let gid = |mask| {
        maps.world
            .content
            .gid(&format!("yapshire:quay/{mask}"))
            .unwrap()
    };
    doc.paint(4, UVec2::new(4, 0), gid(0));
    doc.paint(4, UVec2::new(5, 0), gid(0));
    doc.connect_terrain(4, &maps, MapKind::Town);
    doc.finish();
    assert_eq!(&doc.map.layers[4].data[4..6], &[gid(2), gid(8)]);
    doc.paint(4, UVec2::new(4, 0), 0);
    doc.connect_terrain(4, &maps, MapKind::Town);
    doc.finish();
    assert_eq!(doc.map.layers[4].data[5], gid(0));
    doc.undo();
    assert_eq!(&doc.map.layers[4].data[4..6], &[gid(2), gid(8)]);
    doc.undo();
    assert_eq!(doc.map, original);
    let prefab = &maps.world.content.manifest.prefabs["yapshire:shop"];
    doc.stamp(3, UVec2::new(1, 1), prefab, &maps);
    doc.connect_terrain(3, &maps, MapKind::Town);
    doc.finish();
    assert_ne!(doc.map, original);
    assert_eq!(doc.undo.len(), 1);
    doc.undo();
    assert_eq!(doc.map, original);
    doc.stamp(3, UVec2::new(89, 16), prefab, &maps);
    assert_eq!(
        doc.map, original,
        "an object that does not fit must not be partially painted"
    );
    doc.fill(4, UVec2::ZERO, gid(0));
    doc.connect_terrain(4, &maps, MapKind::Town);
    doc.finish();
    assert_eq!(doc.undo.len(), 1, "fill and terrain joins are one edit");
    assert_eq!(doc.map.layers[4].data[0], gid(6));
    doc.undo();
    assert_eq!(doc.map, original);
}

#[test]
fn importing_legacy_maps_never_rewrites_the_original_and_save_keeps_a_backup() {
    let workspace = Workspace::new();
    std::fs::create_dir_all(&workspace.0).unwrap();
    let legacy = workspace.0.join("town.tmj");
    std::fs::write(&legacy, yapshire_shared::TOWN).unwrap();
    let mut maps = maps();
    let mut editor = Editor::from_directory(&mut maps, Ok(workspace.0.clone()));
    assert_eq!(
        std::fs::read(&legacy).unwrap(),
        yapshire_shared::TOWN.as_bytes()
    );
    assert!(!workspace.0.join("maps/town.tmj").exists());
    editor.save(MapKind::Town, &mut maps).unwrap();
    assert_eq!(
        std::fs::read(&legacy).unwrap(),
        yapshire_shared::TOWN.as_bytes()
    );
    assert_eq!(
        std::fs::read(legacy.with_extension("tmj.bak")).unwrap(),
        yapshire_shared::TOWN.as_bytes()
    );
    assert!(workspace.0.join("maps/town.tmj").exists());
    assert!(workspace.0.join("pack.json").exists());
}
