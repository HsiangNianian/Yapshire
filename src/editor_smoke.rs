//! Native acceptance through the actual editor controls; input stays inside this app.
use crate::{
    editor::{self, Action, Editor, Tool},
    editor_ui::Readout,
    game,
    maps::{MapKind, Maps},
    ui::{Menu, Page},
};
use bevy::{
    input::{ButtonState, mouse::MouseButtonInput},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use bevy_ecs_tilemap::prelude::{TilePos, TileTextureIndex};

#[derive(Resource, Default)]
pub(crate) struct Check {
    menu_shot: bool,
    stage: u8,
    since: f32,
    before: [u32; 4],
    tiles: usize,
}

pub(crate) fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut check: ResMut<Check>,
    mut menu: ResMut<Menu>,
    editor: Res<Editor>,
    maps: Res<Maps>,
    mut window: Single<(Entity, &mut Window)>,
    camera: Single<&Camera, With<game::OuterCamera>>,
    mut buttons: Query<(&Action, &mut Interaction)>,
    mut home: Query<(&crate::ui::Action, &mut Interaction), Without<Action>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    tiles: Query<(&TilePos, &TileTextureIndex)>,
    readouts: Query<(&Readout, &Text)>,
    mut mouse_events: MessageWriter<MouseButtonInput>,
    mut close: MessageWriter<bevy::window::WindowCloseRequested>,
) {
    let mode = std::env::var("YAPSHIRE_SMOKE").unwrap_or_default();
    if !mode.starts_with("editor") {
        return;
    }
    assert!(
        std::env::var_os("YAPSHIRE_MAP_DIR").is_some(),
        "Editor smoke requires an isolated YAPSHIRE_MAP_DIR"
    );
    let now = time.elapsed_secs();
    assert!(
        now < 100.0,
        "Editor smoke timed out at {}: {}",
        check.stage,
        editor.status
    );
    keys.release_all();
    let capture = |commands: &mut Commands, tag: &str| {
        std::fs::create_dir_all("artifacts").unwrap();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("artifacts/{mode}-{tag}.png")));
    };
    let mut click = |mouse: &mut ButtonInput<MouseButton>, action| {
        let (_, mut interaction) = buttons
            .iter_mut()
            .find(|(a, _)| **a == action)
            .expect("Missing editor button");
        *interaction = Interaction::Pressed;
        mouse.release(MouseButton::Left);
        mouse.press(MouseButton::Left);
    };
    let point_ui = |window: &mut Window, ui: Vec2| {
        let viewport = camera.physical_viewport_rect().unwrap();
        let physical =
            viewport.min.as_vec2() + ui * (viewport.width() as f32 / game::WINDOW_SIZE.x as f32);
        window.set_physical_cursor_position(Some(physical.as_dvec2()));
    };
    let point = |window: &mut Window, cell: UVec2| {
        point_ui(
            window,
            editor::CANVAS
                + ((cell - editor.offset).as_vec2() + Vec2::splat(0.5)) * editor.cell_size(),
        );
    };
    if now - check.since < if check.stage == 0 { 3.0 } else { 0.8 } {
        return;
    }
    window.1.set_physical_cursor_position(None);
    match check.stage {
        0 => {
            if !check.menu_shot {
                capture(&mut commands, "menu");
                check.menu_shot = true;
                check.since = now;
                return;
            }
            if mode == "editor-reload" {
                assert_eq!(maps.get(MapKind::Town).layers[4].data[5 * 90 + 50], 93);
            }
            for (action, mut interaction) in &mut home {
                if matches!(action, crate::ui::Action::Go(Page::Editor)) {
                    *interaction = Interaction::Pressed;
                }
            }
        }
        1 => {
            assert!(menu.page == Page::Editor);
            capture(&mut commands, "town");
            if mode == "editor-reload" {
                check.stage = 30;
                check.since = now;
                return;
            }
            check
                .before
                .copy_from_slice(&editor.docs[0].map.layers[4].data[5 * 90 + 50..5 * 90 + 54]);
            click(&mut mouse, Action::Tile(93));
        }
        2 => {
            assert_eq!(editor.tile, 93);
            point(&mut window.1, UVec2::new(50, 5));
            mouse.release(MouseButton::Left);
            mouse.press(MouseButton::Left);
        }
        3 => point(&mut window.1, UVec2::new(53, 5)),
        4 => {
            mouse.release(MouseButton::Left);
            assert_eq!(
                &editor.docs[0].map.layers[4].data[5 * 90 + 50..5 * 90 + 54],
                &[93; 4]
            );
            assert_eq!(
                &maps.get(MapKind::Town).layers[4].data[5 * 90 + 50..5 * 90 + 54],
                &check.before
            );
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyZ);
        }
        5 => {
            assert_eq!(
                &editor.docs[0].map.layers[4].data[5 * 90 + 50..5 * 90 + 54],
                &check.before
            );
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyZ);
        }
        6 => {
            assert_eq!(
                &editor.docs[0].map.layers[4].data[5 * 90 + 50..5 * 90 + 54],
                &[93; 4]
            );
            click(&mut mouse, Action::Save);
        }
        7 => {
            assert!(!editor.dirty(), "{}", editor.status);
            assert_eq!(maps.get(MapKind::Town).layers[4].data[5 * 90 + 50], 93);
            assert!(
                tiles.iter().any(|(pos, tile)| pos.x == 50
                    && pos.y == 11
                    && tile.0 == maps.tile(93).unwrap().2.id),
                "Saved tile must appear in the runtime tilemap"
            );
            check.tiles = tiles.iter().count();
            click(&mut mouse, Action::Save);
        }
        8 => {
            assert_eq!(
                tiles.iter().count(),
                check.tiles,
                "Repeated saves must not leak tile entities"
            );
            click(&mut mouse, Action::Map(MapKind::Shop));
        }
        9 => {
            assert!(editor.kind == MapKind::Shop);
            mouse.release(MouseButton::Left);
            point(&mut window.1, UVec2::new(16, 11));
            mouse.press(MouseButton::Right);
            capture(&mut commands, "shop");
        }
        10 => {
            mouse.release(MouseButton::Right);
            assert_eq!(editor.docs[1].map.layers[4].data[11 * 30 + 16], 0);
            assert!(editor.dirty());
            click(&mut mouse, Action::Done);
            capture(&mut commands, "unsaved");
        }
        11 => {
            assert!(editor.dialog.is_some());
            click(&mut mouse, Action::Cancel);
        }
        12 => {
            assert!(editor.dialog.is_none() && editor.dirty());
            keys.press(KeyCode::Escape);
            mouse.release(MouseButton::Left);
        }
        13 => click(&mut mouse, Action::Discard),
        14 => {
            assert!(menu.page == Page::Home);
            assert!(!editor.dirty());
            assert_ne!(maps.get(MapKind::Shop).layers[4].data[11 * 30 + 16], 0);
            menu.go(Page::Editor);
        }
        15 => {
            click(&mut mouse, Action::Zoom(1));
            keys.press(KeyCode::F11);
        }
        16 => {
            mouse.release(MouseButton::Left);
            point(&mut window.1, UVec2::new(3, 3));
            mouse.press(MouseButton::Left);
            capture(&mut commands, "fullscreen");
        }
        17 => {
            mouse.release(MouseButton::Left);
            assert_eq!(
                editor.docs[1].map.layers[4].data[3 * 30 + 3],
                93,
                "Fullscreen pointer must paint the selected cell"
            );
            keys.press(KeyCode::Escape);
        }
        18 => click(&mut mouse, Action::Discard),
        19 => {
            assert!(menu.page == Page::Home);
            keys.press(KeyCode::F11);
        }
        20 => {
            capture(&mut commands, "returned");
            assert_eq!(tiles.iter().count(), check.tiles);
        }
        21 => {
            mouse.release(MouseButton::Left);
            menu.go(Page::Editor);
        }
        22 => {
            // Hit the actual icon through the window's cursor, including its child image.
            point_ui(&mut window.1, Vec2::new(428.0, 110.0));
        }
        23 => {
            assert!(
                readouts
                    .iter()
                    .any(|(kind, text)| matches!(kind, Readout::Tooltip)
                        && text.starts_with("Eyedropper [I or Alt+click]")),
                "Hovering the icon must reveal its full name and shortcut"
            );
            point_ui(&mut window.1, Vec2::new(428.0, 110.0));
            capture(&mut commands, "tooltip");
            // The focus system runs in PreUpdate. Send a window input event so
            // its hit test and the editor consume the same just-pressed frame.
            mouse_events.write(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                window: window.0,
            });
        }
        24 => {
            assert!(
                editor.tool == Tool::Pick,
                "Icon image must pass clicks to its button"
            );
            mouse.release(MouseButton::Left);
            click(&mut mouse, Action::Visibility(4));
        }
        25 => {
            assert!(!editor.doc().map.layers[4].visible);
            mouse.release(MouseButton::Left);
            capture(&mut commands, "hidden-layer");
        }
        26 => click(&mut mouse, Action::Undo),
        27 => {
            assert!(editor.doc().map.layers[4].visible);
            click(&mut mouse, Action::Done);
        }
        _ => {
            info!(
                "EDITOR SMOKE PASS: menu, painting, drag, undo/redo, atomic save, runtime tiles, shop, unsaved guard, fullscreen input, icon hover/click, layer visibility and return to play"
            );
            close.write(bevy::window::WindowCloseRequested { window: window.0 });
        }
    }
    check.stage += 1;
    check.since = now;
}
