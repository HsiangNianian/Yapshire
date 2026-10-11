//! Local exploration/building worlds. This never mutates a Club room or editor pack.
pub(crate) mod storage;
use crate::{
    Session,
    game::{self, Actor, Art},
    i18n::{Message, tr},
    settings::Settings,
    ui::{Menu, Page},
};
use bevy::{
    prelude::*,
    ui::RelativeCursorPosition,
    window::{ClosingWindow, WindowCloseRequested},
};
use bevy_ecs_tilemap::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use yapshire_shared::construction::{Catalog, Collision, Layer};
use yapshire_shared::terrain::{Block, CHUNK, HEIGHT, TILE, Terrain, WIDTH};

pub(crate) const MAP_ID: &str = "yapshire:wilderness";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Create,
    Open(usize),
    Select(usize),
    Palette,
    Group(i32),
    Piece(usize),
    Layer(Layer),
    Recall,
    Save,
    Leave,
}

pub(crate) struct Run {
    pub saved: storage::Saved,
    pub selected: usize,
    pub catalog: Catalog,
    pub selected_piece: usize,
    pub using_catalog: bool,
    pub palette_open: bool,
    pub palette_group: usize,
    pub layer: Layer,
    cooldown: f32,
    autosave: f32,
    save_failed: bool,
    chunks: BTreeMap<(i32, i32), Entity>,
    dirty: BTreeSet<(i32, i32)>,
    pub target: Option<(IVec2, bool)>,
}

#[derive(Resource, Default)]
pub(crate) struct Wilds {
    pub active: Option<Run>,
    pub entries: Vec<storage::Entry>,
    pub pending: Option<Action>,
    pub notice: Message,
    pub dirty: bool,
    browsing: bool,
}

#[derive(Component)]
pub(crate) struct ChunkRoot;
#[derive(Component)]
pub(crate) struct Target;

impl Wilds {
    fn refresh(&mut self) {
        match storage::directory().and_then(|d| storage::list(&d)) {
            Ok(entries) => self.entries = entries,
            Err(e) => self.notice = tr("wilds.load_failed").arg("error", e.to_string()),
        }
        self.dirty = true;
    }
}

pub(crate) fn update(
    mut commands: Commands,
    mut wilds: ResMut<Wilds>,
    mut menu: ResMut<Menu>,
    mut session: ResMut<Session>,
    resources: (Res<Art>, Res<crate::maps::Maps>),
    settings: Res<Settings>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<(Entity, &Window)>,
    outer: Single<&Camera, With<game::OuterCamera>>,
    camera: Single<&Transform, With<game::WorldCamera>>,
    panels: Query<&RelativeCursorPosition, With<crate::wilds_ui::Panel>>,
    mut actors: Query<(Entity, &mut Actor)>,
    mut close: MessageReader<WindowCloseRequested>,
    targets: Query<Entity, With<Target>>,
) {
    let (art, maps) = resources;
    if menu.page == Page::Worlds && !wilds.browsing {
        wilds.notice.clear();
        wilds.refresh();
    }
    wilds.browsing = menu.page == Page::Worlds;
    let closing = close.read().next().is_some() && wilds.active.is_some();
    let enabled = !settings.blocks_input()
        && (window.1.focused
            || (cfg!(debug_assertions) && std::env::var_os("YAPSHIRE_SMOKE").is_some()));
    let mut action = wilds.pending.take();
    if closing {
        action = Some(Action::Leave);
    } else if enabled && wilds.active.is_some() {
        if keys.just_pressed(KeyCode::Escape) {
            action = Some(if wilds.active.as_ref().is_some_and(|r| r.palette_open) {
                Action::Palette
            } else {
                Action::Leave
            });
        } else if keys.just_pressed(KeyCode::KeyB) {
            action = Some(Action::Palette);
        } else if keys.just_pressed(KeyCode::KeyE) {
            let run = wilds.active.as_ref().unwrap();
            action = Some(Action::Layer(Layer::ALL[(run.layer.index() + 1) % 3]));
        } else if keys.just_pressed(KeyCode::KeyH) {
            action = Some(Action::Recall);
        } else if keys.just_pressed(KeyCode::F5) {
            action = Some(Action::Save);
        }
        for (i, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
            .into_iter()
            .enumerate()
        {
            if keys.just_pressed(key) {
                action = Some(Action::Select(i));
            }
        }
    }
    if matches!(action, Some(Action::Create | Action::Open(_))) && menu.page == Page::Worlds {
        if action == Some(Action::Create) {
            let invalid = if crate::network::clean(&menu.world_name, 32).is_empty() {
                Some("wilds.name_required")
            } else if !menu.world_seed.trim().is_empty()
                && menu.world_seed.trim().parse::<u32>().is_err()
            {
                Some("wilds.seed_invalid")
            } else {
                None
            };
            if let Some(key) = invalid {
                wilds.notice = tr(key);
                wilds.dirty = true;
                return;
            }
        }
        let catalog = Catalog::new(&maps.world.content);
        let result = (|| {
            if action == Some(Action::Create) {
                let seed = if menu.world_seed.trim().is_empty() {
                    rand::random()
                } else {
                    menu.world_seed
                        .trim()
                        .parse::<u32>()
                        .expect("Validated seed")
                };
                let name = crate::network::clean(&menu.world_name, 32);
                let terrain = Terrain::new(name, seed);
                terrain.validate_catalog(&catalog)?;
                storage::Saved::create(&storage::directory()?, terrain)
            } else if let Some(Action::Open(i)) = action {
                let entry = wilds
                    .entries
                    .get(i)
                    .ok_or_else(|| std::io::Error::other("World not found"))?;
                storage::Saved::load(&entry.path)
            } else {
                unreachable!()
            }
        })();
        let result = result.and_then(|saved| {
            saved.terrain.validate_catalog(&catalog)?;
            Ok(saved)
        });
        match result {
            Ok(mut saved) => {
                if !saved
                    .terrain
                    .construction_clear(saved.terrain.position, &catalog)
                {
                    saved.terrain.position = saved.terrain.construction_spawn(&catalog);
                }
                for (entity, _) in &actors {
                    commands.entity(entity).despawn();
                }
                *session = Session::default();
                session.you = Some(0);
                let p = saved.terrain.position;
                game::spawn_actor(
                    &mut commands,
                    &art,
                    crate::network::Player {
                        id: 0,
                        name: menu.name.clone(),
                        map: MAP_ID.into(),
                        x: p[0],
                        y: p[1],
                        moving: false,
                        facing: false,
                        indoors: false,
                        fishing: false,
                    },
                );
                wilds.active = Some(Run {
                    saved,
                    selected: 0,
                    selected_piece: catalog.index("yapshire:cedar").unwrap_or(0),
                    palette_group: catalog
                        .groups
                        .iter()
                        .position(|g| g == "buildings/modules.tsj")
                        .unwrap_or(0),
                    catalog,
                    using_catalog: false,
                    palette_open: false,
                    layer: Layer::Solid,
                    cooldown: 0.25,
                    autosave: 0.0,
                    save_failed: false,
                    chunks: BTreeMap::new(),
                    dirty: BTreeSet::new(),
                    target: None,
                });
                wilds.notice = tr("wilds.saved");
                wilds.dirty = true;
                menu.go(Page::Exploring);
            }
            Err(e) => {
                wilds.notice = tr("wilds.load_failed").arg("error", e.to_string());
                wilds.dirty = true;
            }
        }
        return;
    }
    let Some(run) = wilds.active.as_mut() else {
        return;
    };
    let mut hud_dirty = false;
    let mut edited = false;
    match action {
        Some(Action::Select(slot)) if slot < 3 => {
            run.selected = slot;
            run.using_catalog = false;
            run.layer = Layer::Solid;
            hud_dirty = true;
        }
        Some(Action::Palette) => {
            run.palette_open = !run.palette_open;
            hud_dirty = true;
        }
        Some(Action::Group(delta)) => {
            run.palette_group = (run.palette_group as i32 + delta)
                .rem_euclid(run.catalog.groups.len() as i32)
                as usize;
            hud_dirty = true;
        }
        Some(Action::Piece(index)) if index < run.catalog.pieces.len() => {
            run.selected_piece = index;
            run.using_catalog = true;
            run.layer = run.catalog.pieces[index].layer;
            hud_dirty = true;
        }
        Some(Action::Layer(layer)) => {
            if !run.using_catalog && layer != Layer::Solid {
                let id = ["yapshire:earth", "yapshire:rock", "yapshire:sand"][run.selected];
                run.selected_piece = run.catalog.index(id).expect("Validated terrain catalog");
                run.using_catalog = true;
            }
            run.layer = layer;
            hud_dirty = true;
        }
        _ => {}
    }
    let dt = time.delta_secs().min(0.05);
    run.cooldown = (run.cooldown - dt).max(0.0);
    if let Some((_, mut actor)) = actors.iter_mut().find(|(_, a)| a.player.map == MAP_ID) {
        if action == Some(Action::Recall) {
            actor.teleport(Vec2::from_array(
                run.saved.terrain.construction_spawn(&run.catalog),
            ));
        }
        let direction = if enabled {
            (keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) as i8
                - keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) as i8) as f32
        } else {
            0.0
        };
        let p = actor.position;
        let mut surfaces = Vec::new();
        for x in ((p.x - 24.0) / TILE).floor() as i32..=((p.x + 24.0) / TILE).floor() as i32 {
            for y in ((p.y - 32.0 - (-actor.velocity_y).max(0.0) * dt) / TILE).floor() as i32
                ..=((p.y + 48.0) / TILE).floor() as i32
            {
                let collision = run.saved.terrain.collision(x, y, &run.catalog);
                if collision != Collision::None {
                    surfaces.push((
                        Rect::new(
                            x as f32 * TILE,
                            y as f32 * TILE,
                            (x + 1) as f32 * TILE,
                            (y + 1) as f32 * TILE,
                        ),
                        collision == Collision::Solid,
                    ));
                }
            }
        }
        let Actor {
            position,
            velocity_y,
            ..
        } = &mut *actor;
        game::step_on_surfaces(
            position,
            velocity_y,
            direction,
            keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            enabled && keys.just_pressed(KeyCode::Space),
            dt,
            &surfaces,
            WIDTH as f32 * TILE,
            0.0,
            Vec2::from_array(run.saved.terrain.construction_spawn(&run.catalog)),
        );
        actor.player.moving = direction != 0.0;
        if direction != 0.0 {
            actor.player.facing = direction < 0.0;
        }
        actor.player.x = actor.position.x;
        actor.player.y = actor.position.y;
        run.saved.terrain.position = actor.position.to_array();
        run.target = None;
        if enabled && !panels.iter().any(|p| p.cursor_over()) {
            if let (Some(cursor), Some(viewport)) = (
                window.1.physical_cursor_position(),
                outer.physical_viewport_rect(),
            ) {
                if viewport.as_rect().contains(cursor) {
                    let point = cursor_cell(
                        cursor,
                        viewport,
                        game::canvas_size(settings.pixel_scale()).as_vec2(),
                        camera.translation.truncate(),
                    );
                    let reachable = run.saved.terrain.construction_reachable(
                        point.x,
                        point.y,
                        actor.position.to_array(),
                        &run.catalog,
                    );
                    run.target = Some((point, reachable));
                    if mouse.just_pressed(MouseButton::Middle) && reachable {
                        if let Some(index) = run.catalog.index(run.saved.terrain.cell(
                            run.layer,
                            point.x,
                            point.y,
                            &run.catalog,
                        )) {
                            run.selected_piece = index;
                            run.using_catalog = true;
                            hud_dirty = true;
                        }
                    }
                    if run.cooldown == 0.0 && reachable && action.is_none() {
                        let terrain = &mut run.saved.terrain;
                        let changed = if mouse.pressed(MouseButton::Left) {
                            terrain.dig_tile(
                                run.layer,
                                point.x,
                                point.y,
                                actor.position.to_array(),
                                &run.catalog,
                            )
                        } else if mouse.pressed(MouseButton::Right) {
                            if run.using_catalog {
                                terrain.place_tile(
                                    run.layer,
                                    point.x,
                                    point.y,
                                    &run.catalog.pieces[run.selected_piece].id,
                                    actor.position.to_array(),
                                    &run.catalog,
                                )
                            } else if run.layer == Layer::Solid
                                && terrain
                                    .cell(Layer::Solid, point.x, point.y, &run.catalog)
                                    .is_empty()
                            {
                                terrain.place(
                                    point.x,
                                    point.y,
                                    Block::MATERIALS[run.selected],
                                    actor.position.to_array(),
                                    &run.catalog,
                                )
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        if changed {
                            for offset in
                                [IVec2::ZERO, IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y]
                            {
                                let cell = point + offset;
                                run.dirty
                                    .insert((cell.x.div_euclid(CHUNK), cell.y.div_euclid(CHUNK)));
                            }
                            run.cooldown = 0.18;
                            hud_dirty = true;
                            edited = true;
                        }
                    }
                }
            }
        }
    }
    run.autosave += time.delta_secs();
    let saving = matches!(action, Some(Action::Save | Action::Leave)) || run.autosave >= 5.0;
    if saving {
        run.autosave = 0.0;
        match run.saved.save() {
            Ok(()) => {
                run.save_failed = false;
                wilds.notice = tr("wilds.saved");
                if action == Some(Action::Leave) {
                    let run = wilds.active.take().unwrap();
                    for entity in run.chunks.into_values().chain(targets.iter()) {
                        commands.entity(entity).despawn();
                    }
                    for (entity, _) in &actors {
                        commands.entity(entity).despawn();
                    }
                    *session = Session::default();
                    menu.go(Page::Worlds);
                    wilds.browsing = false;
                    if closing {
                        commands.entity(window.0).insert(ClosingWindow);
                    }
                }
            }
            Err(e) => {
                run.save_failed = true;
                wilds.notice = tr("wilds.save_failed").arg("error", e.to_string());
            }
        }
        hud_dirty = true;
    } else if edited && !run.save_failed {
        wilds.notice = tr("wilds.pending_save");
    }
    wilds.dirty |= hud_dirty;
}

pub(crate) fn cursor_cell(cursor: Vec2, viewport: URect, view: Vec2, camera: Vec2) -> IVec2 {
    let relative = (cursor - viewport.min.as_vec2()) / viewport.size().as_vec2();
    ((camera + Vec2::new(relative.x - 0.5, 0.5 - relative.y) * view) / TILE)
        .floor()
        .as_ivec2()
}

pub(crate) fn draw(
    mut commands: Commands,
    maps: Res<crate::maps::Maps>,
    mut wilds: ResMut<Wilds>,
    assets: Res<AssetServer>,
    settings: Res<Settings>,
    camera: Single<&Transform, With<game::WorldCamera>>,
    targets: Query<Entity, With<Target>>,
    mut last_target: Local<Option<(IVec2, bool)>>,
) {
    let Some(run) = wilds.active.as_mut() else {
        *last_target = None;
        return;
    };
    let view = game::canvas_size(settings.pixel_scale()).as_vec2();
    let centre = camera.translation.truncate();
    let min = ((centre - view / 2.0) / TILE / CHUNK as f32)
        .floor()
        .as_ivec2()
        .max(IVec2::ZERO);
    let max = ((centre + view / 2.0) / TILE / CHUNK as f32)
        .floor()
        .as_ivec2()
        .min(IVec2::new(WIDTH / CHUNK - 1, HEIGHT / CHUNK - 1));
    let wanted: BTreeSet<_> = (min.x..=max.x)
        .flat_map(|x| (min.y..=max.y).map(move |y| (x, y)))
        .collect();
    let discard: Vec<_> = run
        .chunks
        .keys()
        .filter(|key| !wanted.contains(key) || run.dirty.contains(key))
        .copied()
        .collect();
    for key in discard {
        commands.entity(run.chunks.remove(&key).unwrap()).despawn();
    }
    for &(cx, cy) in &wanted {
        if run.chunks.contains_key(&(cx, cy)) {
            continue;
        }
        let root = commands
            .spawn((ChunkRoot, Transform::default(), Visibility::Inherited))
            .id();
        // A chunk contains independent tilemaps per depth and pack atlas.
        // No object is rendered as an indivisible sprite or flattened composition.
        let mut batches: BTreeMap<
            (usize, &str),
            Vec<(TilePos, &yapshire_shared::construction::Piece, Color)>,
        > = BTreeMap::new();
        for x in 0..CHUNK {
            for y in 0..CHUNK {
                let (wx, wy) = (cx * CHUNK + x, cy * CHUNK + y);
                let pos = TilePos {
                    x: x as u32,
                    y: y as u32,
                };
                if wy < run.saved.terrain.surface(wx) {
                    let piece = run
                        .catalog
                        .get("yapshire:rock")
                        .expect("Validated world catalog");
                    batches.entry((0, &piece.source)).or_default().push((
                        pos,
                        piece,
                        Color::srgb_u8(75, 78, 79),
                    ));
                }
                for depth in Layer::ALL {
                    let id = run.saved.terrain.visual_cell(depth, wx, wy, &run.catalog);
                    if let Some(piece) = run.catalog.get(id) {
                        batches
                            .entry((depth.index() + 1, &piece.source))
                            .or_default()
                            .push((pos, piece, Color::WHITE));
                    }
                }
            }
        }
        for ((depth, _), cells) in batches {
            let layer = commands.spawn((ChildOf(root),)).id();
            let size = TilemapSize {
                x: CHUNK as u32,
                y: CHUNK as u32,
            };
            let texture = assets.load(maps.image_path(&cells[0].1.image));
            let mut storage = TileStorage::empty(size);
            for (pos, piece, color) in cells {
                let mut entity = commands.spawn((
                    TileBundle {
                        position: pos,
                        tilemap_id: TilemapId(layer),
                        texture_index: TileTextureIndex(piece.index),
                        color: TileColor(color),
                        ..default()
                    },
                    ChildOf(layer),
                ));
                if let Some(first) = piece.animation.first() {
                    entity.insert(AnimatedTile {
                        start: first.tileid,
                        end: first.tileid + piece.animation.len() as u32,
                        speed: 1000.0 / (first.duration as f32 * piece.animation.len() as f32),
                    });
                }
                storage.set(&pos, entity.id());
            }
            let tile_size = TilemapTileSize { x: TILE, y: TILE };
            commands.entity(layer).insert(TilemapBundle {
                grid_size: tile_size.into(),
                map_type: TilemapType::Square,
                size,
                storage,
                texture: TilemapTexture::Single(texture),
                tile_size,
                transform: Transform::from_xyz(
                    (cx * CHUNK) as f32 * TILE + 8.0,
                    (cy * CHUNK) as f32 * TILE + 8.0,
                    [-24.0, -18.0, -10.0, -8.0][depth],
                ),
                ..default()
            });
        }
        run.chunks.insert((cx, cy), root);
    }
    run.dirty.clear();
    if run.target != *last_target {
        for entity in &targets {
            commands.entity(entity).despawn();
        }
        if let Some((cell, reachable)) = run.target {
            let color = if reachable {
                crate::ui::GOLD
            } else {
                Color::srgb_u8(202, 100, 86)
            };
            for (offset, size) in [
                (Vec2::new(0.0, 7.5), Vec2::new(16.0, 1.0)),
                (Vec2::new(0.0, -7.5), Vec2::new(16.0, 1.0)),
                (Vec2::new(7.5, 0.0), Vec2::new(1.0, 16.0)),
                (Vec2::new(-7.5, 0.0), Vec2::new(1.0, 16.0)),
            ] {
                let point = (cell.as_vec2() + 0.5) * TILE + offset;
                commands.spawn((
                    Target,
                    Sprite::from_color(color, size),
                    Transform::from_xyz(point.x, point.y, 12.0),
                ));
            }
        }
        *last_target = run.target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pointer_targets_the_same_world_cell_at_every_scene_and_ui_scale() {
        for scene in 2..=4 {
            for ui in [75, 100, 125, 150] {
                let view = game::canvas_size(scene).as_vec2();
                let viewport = URect::from_corners(UVec2::new(40, 20), UVec2::new(1480, 828));
                let camera = Vec2::new(16400.0, 2600.0);
                let target = Vec2::new(16424.0, 2568.0);
                let relative = (target - camera) / view;
                let cursor = viewport.min.as_vec2()
                    + Vec2::new(relative.x + 0.5, 0.5 - relative.y) * viewport.size().as_vec2();
                assert_eq!(
                    cursor_cell(cursor, viewport, view, camera),
                    (target / TILE).floor().as_ivec2(),
                    "scene={scene} ui={ui}"
                );
            }
        }
    }
}
