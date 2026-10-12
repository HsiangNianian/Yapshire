use crate::{
    Session,
    game::{self, Actor, Art, OuterCamera, WorldCamera},
    i18n::{Localized, tr},
    maps::{MapKind, Maps},
    ui::{Chat, Menu, Page},
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::Anchor,
};
use yapshire_shared::pixels::{CHUNK, Grid, RADIUS, REACH, Tool};

#[derive(Component)]
pub(crate) struct Chunk {
    pub map: MapKind,
    pub index: usize,
}
#[derive(Resource, Default)]
pub(crate) struct Brush {
    pub open: bool,
    pub tool: Tool,
}
#[derive(Component)]
pub(crate) struct Panel;
#[derive(Component)]
pub(crate) struct Choice(Tool);
#[derive(Component)]
pub(crate) struct Cursor;

pub(crate) fn spawn(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    parent: Entity,
    map: MapKind,
    grid: &Grid,
) {
    for index in 0..grid.chunks() {
        let [x, y] = grid.chunk_origin(index);
        let image = images.add(Image::new(
            Extent3d {
                width: CHUNK,
                height: CHUNK,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            grid.rgba(index),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        commands.spawn((
            Chunk { map, index },
            Sprite::from_image(image),
            Anchor::TOP_LEFT,
            Transform::from_xyz(x as f32, grid.collision.origin_y() - y as f32, -1.0),
            ChildOf(parent),
        ));
    }
}

pub(crate) fn upload(
    mut maps: ResMut<Maps>,
    mut images: ResMut<Assets<Image>>,
    chunks: Query<(&Chunk, &Sprite)>,
) {
    for (chunk, sprite) in &chunks {
        if let Some(grid) = maps.pixels.get(&maps.info(chunk.map).id) {
            if grid.dirty(chunk.index) {
                if let Some(image) = images.get_mut(&sprite.image) {
                    image.data = Some(grid.rgba(chunk.index));
                }
            }
        }
    }
    for grid in maps.pixels.values_mut() {
        grid.clear_dirty();
    }
}

pub(crate) fn controls(
    mut commands: Commands,
    mut brush: ResMut<Brush>,
    context: (
        Res<Maps>,
        Res<Session>,
        Res<Menu>,
        Res<Chat>,
        Res<crate::settings::Settings>,
        Res<crate::fishing::Fishing>,
    ),
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    art: Res<Art>,
    window: Single<&Window>,
    world: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    outer: Single<&Camera, (With<OuterCamera>, Without<WorldCamera>)>,
    actors: Query<&Actor>,
    buttons: Query<(&Choice, &Interaction), Changed<Interaction>>,
    interactions: Query<&Interaction>,
    mut view: ParamSet<(
        Query<Entity, Or<(With<Panel>, With<Cursor>)>>,
        Query<(&mut Transform, &mut Sprite, &mut Visibility), With<Cursor>>,
    )>,
    mut images: ResMut<Assets<Image>>,
    mut local: Local<(bool, Tool, f32)>,
) {
    let (maps, session, menu, chat, settings, fishing) = context;
    let mine = actors.iter().find(|a| Some(a.player.id) == session.you);
    let allowed = menu.page == Page::Playing
        && session.connected
        && !chat.open
        && !settings.blocks_input()
        && !fishing.modal()
        && mine.is_some_and(|a| maps.pixels.contains_key(&a.player.map));
    if menu.page != Page::Playing {
        brush.open = false;
    }
    if allowed
        && keys.just_pressed(KeyCode::KeyV)
        && !keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ])
    {
        brush.open = !brush.open;
    }
    if allowed && brush.open {
        for (key, tool) in [
            (KeyCode::Digit1, Tool::Dig),
            (KeyCode::Digit2, Tool::Sand),
            (KeyCode::Digit3, Tool::Water),
        ] {
            if keys.just_pressed(key) {
                brush.tool = tool;
            }
        }
        for (choice, interaction) in &buttons {
            if *interaction == Interaction::Pressed {
                brush.tool = choice.0;
            }
        }
    }
    let shown = allowed && brush.open;
    if (local.0, local.1) != (shown, brush.tool) {
        for entity in &view.p0() {
            commands.entity(entity).despawn();
        }
        local.0 = shown;
        local.1 = brush.tool;
        if shown {
            let root = commands
                .spawn((
                    Panel,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(24),
                        bottom: px(148),
                        padding: UiRect::all(px(10)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(8),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.04, 0.09, 0.08, 0.95)),
                    GlobalZIndex(5),
                    Interaction::None,
                ))
                .id();
            let row = commands
                .spawn((
                    Node {
                        column_gap: px(6),
                        ..default()
                    },
                    ChildOf(root),
                ))
                .id();
            for (tool, key, icon) in [
                (Tool::Dig, "pixels.dig", crate::icons::Icon::Eraser),
                (Tool::Sand, "pixels.sand", crate::icons::Icon::Brush),
                (Tool::Water, "pixels.water", crate::icons::Icon::Fill),
            ] {
                let button = commands
                    .spawn((
                        Button,
                        Choice(tool),
                        Node {
                            width: px(120),
                            height: px(48),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            column_gap: px(8),
                            ..default()
                        },
                        BackgroundColor(if tool == brush.tool {
                            Color::srgb_u8(91, 112, 76)
                        } else {
                            Color::srgb_u8(41, 63, 57)
                        }),
                        ChildOf(row),
                    ))
                    .id();
                commands.spawn((
                    icon.image(&art),
                    Node {
                        width: px(24),
                        height: px(24),
                        ..default()
                    },
                    ChildOf(button),
                ));
                let label = tr(key);
                commands.spawn((
                    Text::new(label.to_string()),
                    Localized(label),
                    game::font(&art, 18.0),
                    TextColor(Color::srgb_u8(237, 226, 184)),
                    ChildOf(button),
                ));
            }
            let label = tr("pixels.help");
            commands.spawn((
                Text::new(label.to_string()),
                Localized(label),
                game::font(&art, 16.0),
                TextColor(Color::srgb_u8(200, 207, 171)),
                ChildOf(root),
            ));
            let size = (RADIUS * 2 + 1) as u32;
            let data = (0..size * size)
                .flat_map(|i| {
                    let dx = i as i32 % size as i32 - RADIUS;
                    let dy = i as i32 / size as i32 - RADIUS;
                    [
                        255,
                        255,
                        255,
                        if (RADIUS * RADIUS - 8..=RADIUS * RADIUS).contains(&(dx * dx + dy * dy)) {
                            255
                        } else {
                            0
                        },
                    ]
                })
                .collect();
            let image = images.add(Image::new(
                Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            ));
            commands.spawn((
                Cursor,
                Sprite::from_image(image),
                Transform::from_xyz(0.0, 0.0, 100.0),
                Visibility::Hidden,
            ));
        }
    }
    if !shown {
        return;
    }
    let mut target = None;
    if let Some(position) = window.cursor_position() {
        if let Some(viewport) = outer.physical_viewport_rect() {
            let physical = position * window.scale_factor();
            if viewport.as_rect().contains(physical) {
                let canvas = (physical - viewport.min.as_vec2()) / viewport.size().as_vec2()
                    * game::canvas_size(settings.pixel_scale()).as_vec2();
                target = world.0.viewport_to_world_2d(world.1, canvas).ok();
            }
        }
    }
    let actor = mine.unwrap();
    let map = maps.by_id(&actor.player.map).unwrap();
    let over_ui = interactions.iter().any(|i| *i != Interaction::None);
    let point =
        target.map(|v| IVec2::new(v.x.floor() as i32, (map.origin_y() - v.y).floor() as i32));
    let reachable = point.is_some_and(|p| {
        p.x >= 0
            && p.y >= 0
            && p.x < map.width as i32 * 16
            && p.y < map.height as i32 * 16
            && (p.as_vec2() - Vec2::new(actor.position.x, map.origin_y() - actor.position.y - 12.0))
                .length_squared()
                <= REACH * REACH
    });
    for (mut transform, mut sprite, mut visibility) in &mut view.p1() {
        *visibility = if point.is_some() && !over_ui {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(p) = point {
            transform.translation.x = p.x as f32 + 0.5;
            transform.translation.y = map.origin_y() - p.y as f32 - 0.5;
        }
        sprite.color = if reachable {
            Color::srgb_u8(248, 218, 140)
        } else {
            Color::srgb_u8(214, 108, 87)
        };
    }
    if reachable
        && !over_ui
        && mouse.pressed(MouseButton::Left)
        && time.elapsed_secs() - local.2 >= 0.11
        && (window.focused
            || (cfg!(debug_assertions) && std::env::var_os("YAPSHIRE_SMOKE").is_some()))
    {
        let point = point.unwrap();
        if let Some(link) = &session.link {
            if link
                .send
                .try_send(yapshire_shared::protocol::ClientMessage::Paint {
                    map: actor.player.map.clone(),
                    x: point.x as u32,
                    y: point.y as u32,
                    tool: brush.tool,
                })
                .is_ok()
            {
                local.2 = time.elapsed_secs();
            }
        }
    }
}
