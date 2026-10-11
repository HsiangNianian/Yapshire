use crate::{
    Session,
    network::{ClientMessage, Player},
    ui::{Chat, Menu, Page},
};
use bevy::{
    camera::{RenderTarget, Viewport, visibility::RenderLayers},
    prelude::*,
    render::{
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::screenshot::{Screenshot, save_to_disk},
    },
    sprite::Anchor,
    text::FontSmoothing,
};

pub const WIDTH: f32 = 720.0;
pub const HEIGHT: f32 = 405.0;
pub const WINDOW_SIZE: UVec2 = UVec2::new(1440, 810);
const CAMERA_Y: f32 = HEIGHT / 2.0 - 64.0;
const CHARACTER_SIZE: UVec2 = UVec2::new(20, 32);

pub(crate) fn canvas_size(scale: u32) -> UVec2 {
    WINDOW_SIZE / scale.clamp(2, 4)
}

#[derive(Resource)]
pub struct Art {
    pub font: Handle<Font>,
    pub people: Handle<Image>,
    pub atlas: Handle<TextureAtlasLayout>,
    pub shadow: Handle<Image>,
    pub items: Handle<Image>,
    pub items_atlas: Handle<TextureAtlasLayout>,
    pub panel: Handle<Image>,
    pub slot: Handle<Image>,
    pub water: Handle<Image>,
    pub editor_icons: Handle<Image>,
}

#[derive(Component)]
pub struct Actor {
    pub player: Player,
    pub position: Vec2,
    velocity_y: f32,
    phase: f32,
    send_time: f32,
    last_sent: (Vec2, bool, bool, String, bool),
}

#[derive(Component)]
pub(crate) struct Shadow;

impl Actor {
    pub(crate) fn teleport(&mut self, position: Vec2) {
        self.position = position;
        self.player.x = position.x;
        self.player.y = position.y;
        self.velocity_y = 0.0;
    }
}

#[derive(Component)]
pub(crate) struct Outside;

#[derive(Component)]
pub struct Bubble {
    pub id: u32,
    timer: Timer,
    height: f32,
}
#[derive(Component)]
pub(crate) struct WorldCamera;
#[derive(Component)]
pub(crate) struct OuterCamera;
#[derive(Component)]
pub(crate) struct Backdrop {
    base: Vec2,
    parallax: f32,
}
impl Backdrop {
    fn x(&self, camera: f32, width: f32, view_width: f32) -> f32 {
        let margin = ((width - view_width) / 2.0).max(0.0);
        (self.base.x + (camera - WIDTH / 2.0) * self.parallax)
            .clamp(camera - margin, camera + margin)
            .round()
    }
}
#[derive(Component)]
pub(crate) struct Cloud {
    x: f32,
    y: f32,
    speed: f32,
}
#[derive(Component)]
pub(crate) struct Mote {
    origin: Vec2,
    phase: f32,
}

pub fn font(art: &Art, size: f32) -> TextFont {
    TextFont {
        font: art.font.clone(),
        font_size: size,
        font_smoothing: FontSmoothing::None,
        ..default()
    }
}

pub fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    settings: Res<crate::settings::Settings>,
) {
    let art = Art {
        font: assets.load("fonts/fusion-pixel.ttf"),
        people: assets.load("people.png"),
        atlas: layouts.add(TextureAtlasLayout::from_grid(
            CHARACTER_SIZE,
            6,
            4,
            None,
            None,
        )),
        shadow: assets.load("shadow.png"),
        items: assets.load("fishing/items.png"),
        items_atlas: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(32),
            4,
            4,
            None,
            None,
        )),
        panel: assets.load("fishing/frame.png"),
        slot: assets.load("fishing/slot.png"),
        water: assets.load("fishing/water.png"),
        editor_icons: assets.load("ui/editor-icons.png"),
    };
    let view = canvas_size(settings.pixel_scale());
    let size = Extent3d {
        width: view.x,
        height: view.y,
        depth_or_array_layers: 1,
    };
    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("pixel canvas"),
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    canvas.resize(size);
    let canvas = images.add(canvas);
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        RenderTarget::Image(canvas.clone().into()),
        Msaa::Off,
        WorldCamera,
        Transform::from_xyz(WIDTH / 2.0, view.y as f32 / 2.0 - 64.0, 0.0),
    ));
    commands.spawn((Sprite::from_image(canvas), RenderLayers::layer(1)));
    commands.spawn((
        Camera2d,
        Msaa::Off,
        OuterCamera,
        RenderLayers::layer(1),
        IsDefaultUiCamera,
    ));
    commands.spawn((
        Sprite::from_image(assets.load("sky.png")),
        Outside,
        Transform::from_xyz(WIDTH / 2.0, CAMERA_Y, -50.0),
        Backdrop {
            base: Vec2::new(WIDTH / 2.0, CAMERA_Y),
            parallax: 1.0,
        },
    ));
    commands.spawn((
        Sprite {
            image: assets.load("hills.png"),
            custom_size: Some(Vec2::new(1620.0, 540.0)),
            ..default()
        },
        Outside,
        Transform::from_xyz(810.0, 124.0, -46.0),
        Backdrop {
            base: Vec2::new(810.0, 124.0),
            parallax: 0.44,
        },
    ));
    for (x, y, speed) in [
        (77.0, 154.0, 1.5),
        (325.0, 176.0, 0.8),
        (591.0, 144.0, 1.2),
        (884.0, 163.0, 1.0),
        (1190.0, 175.0, 1.3),
    ] {
        commands.spawn((
            Sprite::from_image(assets.load("cloud.png")),
            Outside,
            Transform::from_xyz(x, y, -45.0),
            Cloud { x, y, speed },
        ));
    }
    for i in 0..22 {
        let origin = Vec2::new(30.0 + i as f32 * 64.0, (i * 17 % 170) as f32);
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(192, 165, 96), Vec2::new(2.0, 1.0)),
            Transform::from_xyz(origin.x, origin.y, 9.0),
            Mote {
                origin,
                phase: i as f32 * 2.3,
            },
            Outside,
        ));
    }
    commands.insert_resource(art);
}

pub fn spawn_actor(commands: &mut Commands, art: &Art, player: Player) {
    let position = Vec2::new(player.x, player.y);
    let row = player.id as usize % 4;
    let name = crate::network::clean(&player.name, 12);
    let id = player.id;
    let entity = commands
        .spawn((
            Sprite::from_atlas_image(
                art.people.clone(),
                TextureAtlas {
                    layout: art.atlas.clone(),
                    index: row * 6,
                },
            ),
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(position.x.round(), position.y.round(), 5.0),
            Actor {
                player,
                position,
                velocity_y: 0.0,
                phase: 0.0,
                send_time: 0.0,
                last_sent: (position, false, false, String::new(), false),
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Shadow,
                Sprite::from_image(art.shadow.clone()),
                Transform::from_xyz(0.0, 1.0, -0.2),
            ));
            parent.spawn((
                Text2d::new(name),
                font(art, 9.0),
                TextColor(crate::ui::CREAM),
                TextBackgroundColor(Color::srgba_u8(26, 41, 38, 210)),
                Transform::from_xyz(0.0, 38.0, 0.5),
            ));
        })
        .id();
    crate::coast::rod(commands, entity, id, art);
}

fn overlaps_x(x: f32, left: f32, right: f32) -> bool {
    x + 6.0 > left + 0.01 && x - 6.0 < right - 0.01
}

fn step(
    position: &mut Vec2,
    velocity_y: &mut f32,
    direction: f32,
    run: bool,
    jump: bool,
    dt: f32,
    map: &crate::maps::Map,
    content: &yapshire_shared::content::Content,
    path: &str,
) {
    const HALF: f32 = 6.0;
    const HEIGHT: f32 = 26.0;
    let old = *position;
    let mut surfaces = Vec::new();
    let min = old - Vec2::new(24.0, 32.0 + (-*velocity_y).max(0.0) * dt);
    let max = old + Vec2::new(24.0, 48.0);
    for row in (((map.origin_y() - max.y) / 16.0).floor() as i32).max(0)
        ..=((map.origin_y() - min.y) / 16.0).floor() as i32
    {
        for col in ((min.x / 16.0).floor() as i32).max(0)..=(max.x / 16.0).floor() as i32 {
            let collision = map.collision(content, path, col, row);
            if collision != "none" {
                let top = map.origin_y() - row as f32 * 16.0;
                surfaces.push((
                    Rect::new(col as f32 * 16.0, top - 16.0, (col + 1) as f32 * 16.0, top),
                    collision == "solid",
                ));
            }
        }
    }
    for o in map.objects().filter(|o| o.kind == "solid") {
        surfaces.push((
            Rect::new(
                o.x,
                map.origin_y() - o.y - o.height,
                o.x + o.width,
                map.origin_y() - o.y,
            ),
            true,
        ));
    }
    let grounded = surfaces
        .iter()
        .any(|(r, _)| overlaps_x(old.x, r.min.x, r.max.x) && (old.y - r.max.y).abs() < 0.1);
    if jump && grounded {
        *velocity_y = 180.0;
    }
    position.x = (old.x + direction * if run { 105.0 } else { 62.0 } * dt)
        .clamp(12.0, map.width as f32 * 16.0 - 12.0);
    for (r, solid) in &surfaces {
        if *solid
            && old.y < r.max.y - 0.01
            && old.y + HEIGHT > r.min.y + 0.01
            && overlaps_x(position.x, r.min.x, r.max.x)
        {
            position.x = if direction > 0.0 {
                r.min.x - HALF
            } else if direction < 0.0 {
                r.max.x + HALF
            } else {
                old.x
            };
        }
    }
    *velocity_y -= 460.0 * dt;
    position.y = old.y + *velocity_y * dt;
    for (r, solid) in &surfaces {
        if !overlaps_x(position.x, r.min.x, r.max.x) {
            continue;
        }
        if *velocity_y <= 0.0 && old.y >= r.max.y - 0.01 && position.y <= r.max.y {
            position.y = r.max.y;
            *velocity_y = 0.0;
        } else if *solid
            && *velocity_y > 0.0
            && old.y + HEIGHT <= r.min.y + 0.01
            && position.y + HEIGHT >= r.min.y
        {
            position.y = r.min.y - HEIGHT;
            *velocity_y = 0.0;
        }
    }
    if position.y < -64.0 {
        if let Some(spawn) = map
            .objects()
            .find(|o| o.kind == "spawn")
            .and_then(|o| map.spawn(&o.name))
        {
            *position = Vec2::from_array(spawn);
            *velocity_y = 0.0;
        }
    }
}

pub fn walk(
    maps: Res<crate::maps::Maps>,
    settings: Res<crate::settings::Settings>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    menu: Res<Menu>,
    chat: Res<Chat>,
    session: Res<Session>,
    fishing: Res<crate::fishing::Fishing>,
    window: Single<&Window>,
    mut actors: Query<&mut Actor>,
) {
    let dt = time.delta_secs().min(0.05);
    for mut actor in &mut actors {
        let Some(map) = maps.by_id(&actor.player.map) else {
            continue;
        };
        let path = &maps
            .world
            .content
            .manifest
            .maps
            .iter()
            .find(|m| m.id == actor.player.map)
            .unwrap()
            .path;
        if Some(actor.player.id) != session.you {
            let target = Vec2::new(
                actor.player.x.clamp(12.0, map.width as f32 * 16.0 - 12.0),
                actor.player.y.clamp(-64.0, map.origin_y() + 96.0),
            );
            actor.position = actor.position.lerp(target, 1.0 - (-18.0 * dt).exp());
            continue;
        }
        let enabled = menu.page == Page::Playing
            && !settings.blocks_input()
            && !chat.open
            && !fishing.modal()
            && session.connected
            && (window.focused
                || (cfg!(debug_assertions) && std::env::var_os("YAPSHIRE_SMOKE").is_some()));
        let direction = if enabled {
            (keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) as i8
                - keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) as i8) as f32
        } else {
            0.0
        };
        let run = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let jump = enabled && keys.just_pressed(KeyCode::Space);
        let Actor {
            position,
            velocity_y,
            ..
        } = &mut *actor;
        step(
            position,
            velocity_y,
            direction,
            run,
            jump,
            dt,
            map,
            &maps.world.content,
            path,
        );
        actor.player.x = actor.position.x;
        actor.player.y = actor.position.y;
        actor.player.moving = direction != 0.0;
        if direction != 0.0 {
            actor.player.facing = direction < 0.0;
        }
        actor.send_time += dt;
        let now = (
            actor.position,
            actor.player.moving,
            actor.player.facing,
            actor.player.map.clone(),
            actor.player.fishing,
        );
        if actor.send_time >= 0.05 && now != actor.last_sent {
            if let Some(link) = &session.link {
                let sent = link.send.try_send(ClientMessage::Move {
                    map: actor.player.map.clone(),
                    x: actor.player.x,
                    y: actor.player.y,
                    moving: actor.player.moving,
                    facing: actor.player.facing,
                    indoors: actor.player.indoors,
                    fishing: actor.player.fishing,
                });
                if sent.is_ok() {
                    actor.last_sent = now;
                    actor.send_time = 0.0;
                }
            }
        }
    }
}

pub fn animate(
    time: Res<Time>,
    maps: Res<crate::maps::Maps>,
    mut actors: Query<(&mut Actor, &mut Sprite, &mut Transform, &Children)>,
    mut clouds: Query<(&Cloud, &mut Transform), Without<Actor>>,
    mut motes: Query<(&Mote, &mut Transform, &mut Sprite), (Without<Actor>, Without<Cloud>)>,
    mut shadows: Query<
        (&mut Transform, &mut Visibility),
        (With<Shadow>, Without<Actor>, Without<Cloud>, Without<Mote>),
    >,
) {
    let t = time.elapsed_secs();
    for (mut actor, mut sprite, mut transform, children) in &mut actors {
        actor.phase += time.delta_secs() * if actor.player.moving { 9.0 } else { 1.3 };
        let frame = if actor.player.y > 1.0 {
            3
        } else if actor.player.moving {
            2 + actor.phase as usize % 4
        } else {
            actor.phase as usize % 2
        };
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = actor.player.id as usize % 4 * 6 + frame;
        }
        sprite.flip_x = actor.player.facing;
        transform.translation.x = actor.position.x.round();
        transform.translation.y = actor.position.y.round();
        let ground = maps
            .world
            .content
            .manifest
            .maps
            .iter()
            .find(|info| info.id == actor.player.map)
            .and_then(|info| {
                shadow_ground_y(
                    actor.position,
                    maps.by_id(&info.id)?,
                    &maps.world.content,
                    &info.path,
                )
            });
        for child in children.iter() {
            if let Ok((mut shadow, mut visibility)) = shadows.get_mut(child) {
                if let Some(y) = ground {
                    // Keep parenting for horizontal movement and cleanup; cancel the jump.
                    shadow.translation.y = y.round() + 1.0 - transform.translation.y;
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
    for (cloud, mut transform) in &mut clouds {
        transform.translation.x = (cloud.x + t * cloud.speed).rem_euclid(1700.0).round() - 80.0;
        transform.translation.y = cloud.y;
    }
    for (mote, mut transform, mut sprite) in &mut motes {
        transform.translation.x = (mote.origin.x + (t * 0.35 + mote.phase).sin() * 14.0).round();
        transform.translation.y = (mote.origin.y - t * 3.0).rem_euclid(185.0).round();
        sprite
            .color
            .set_alpha(0.18 + (t * 0.8 + mote.phase).sin().max(0.0) * 0.4);
    }
}

fn shadow_ground_y(
    position: Vec2,
    map: &crate::maps::Map,
    content: &yapshire_shared::content::Content,
    path: &str,
) -> Option<f32> {
    // Scan candidate columns, then use the same strict footprint test as step().
    let left = position.x - 6.0;
    let right = position.x + 6.0;
    let first_col = ((left / 16.0).floor() as i32).max(0);
    let last_col = ((right / 16.0).floor() as i32).min(map.width as i32 - 1);
    let first_row = ((map.origin_y() - position.y - 0.1) / 16.0).ceil().max(0.0) as i32;
    let mut ground = None;
    for row in first_row..map.height as i32 {
        if (first_col..=last_col).any(|col| {
            overlaps_x(position.x, col as f32 * 16.0, (col + 1) as f32 * 16.0)
                && map.collision(content, path, col, row) != "none"
        }) {
            ground = Some(map.origin_y() - row as f32 * 16.0);
            break;
        }
    }
    for object in map.objects().filter(|o| o.kind == "solid") {
        let top = map.origin_y() - object.y;
        if overlaps_x(position.x, object.x, object.x + object.width) && top <= position.y + 0.1 {
            ground = Some(ground.map_or(top, |y| y.max(top)));
        }
    }
    ground
}

pub fn follow_camera(
    maps: Res<crate::maps::Maps>,
    settings: Res<crate::settings::Settings>,
    time: Res<Time>,
    session: Res<Session>,
    actors: Query<&Actor>,
    mut camera: Single<&mut Transform, (With<WorldCamera>, Without<Backdrop>)>,
    mut backdrops: Query<(&Backdrop, &Sprite, &mut Transform), Without<WorldCamera>>,
    mut previous_map: Local<String>,
) {
    let mine = actors.iter().find(|a| Some(a.player.id) == session.you);
    let map_id = mine.map_or(maps.world.entry().0, |a| a.player.map.as_str());
    let map = maps
        .by_id(map_id)
        .unwrap_or(maps.get(crate::maps::MapKind(0)));
    let view = canvas_size(settings.pixel_scale()).as_vec2();
    let camera_y = view.y / 2.0 - 64.0;
    let width = map.width as f32 * 16.0;
    let target = if width <= view.x {
        width / 2.0
    } else {
        mine.map_or(WIDTH / 2.0, |a| a.position.x)
            .clamp(view.x / 2.0, width - view.x / 2.0)
    };
    if *previous_map != map_id {
        camera.translation.x = target;
        *previous_map = map_id.to_owned();
    }
    camera.translation.y = mine
        .map_or(camera_y, |a| (a.position.y + camera_y).max(camera_y))
        .min((map.origin_y() - view.y / 2.0).max(camera_y));
    let x = camera.translation.x
        + (target - camera.translation.x) * (1.0 - (-6.0 * time.delta_secs()).exp());
    let x = if (target - x).abs() < 1.0 {
        target.round()
    } else {
        x
    };
    camera.translation.x = if width <= view.x {
        width / 2.0
    } else {
        x.clamp(view.x / 2.0, width - view.x / 2.0)
    };
    for (backdrop, sprite, mut transform) in &mut backdrops {
        let width = sprite.custom_size.map_or(WIDTH, |size| size.x);
        transform.translation.x = backdrop.x(camera.translation.x, width, view.x);
        transform.translation.y = backdrop.base.y;
    }
}

fn bubble_text(text: &str) -> String {
    let mut out = String::new();
    let (mut width, mut lines) = (0, 1);
    for ch in text.chars() {
        let w = if ch.is_ascii() { 1 } else { 2 };
        if width + w > 22 {
            if lines == 3 {
                out.push('…');
                break;
            }
            out.push('\n');
            width = 0;
            lines += 1;
        }
        out.push(ch);
        width += w;
    }
    out
}

pub fn spawn_bubble(commands: &mut Commands, art: &Art, id: u32, position: Vec2, text: &str) {
    let text = bubble_text(text);
    let height = text.lines().count() as f32 * 11.0 + 10.0;
    commands
        .spawn((
            Sprite::from_color(crate::ui::PANEL, Vec2::new(112.0, height)),
            Transform::from_xyz(position.x, position.y + 44.0 + height / 2.0, 20.0),
            Bubble {
                id,
                height,
                timer: Timer::from_seconds(8.0, TimerMode::Once),
            },
        ))
        .with_children(|p| {
            p.spawn((
                Text2d::new(text),
                font(art, 9.0),
                TextColor(crate::ui::CREAM),
                TextLayout::new_with_justify(Justify::Center),
                Transform::from_xyz(0.0, 1.0, 1.0),
            ));
            p.spawn((
                Sprite::from_color(crate::ui::PANEL, Vec2::new(4.0, 3.0)),
                Transform::from_xyz(0.0, -height / 2.0 - 2.0, 0.0),
            ));
        });
}

pub fn bubbles(
    mut commands: Commands,
    time: Res<Time>,
    actors: Query<&Actor>,
    fishing: Res<crate::fishing::Fishing>,
    mut bubbles: Query<(Entity, &mut Bubble, &mut Transform, &mut Visibility)>,
) {
    for (entity, mut bubble, mut transform, mut visibility) in &mut bubbles {
        if bubble.timer.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(actor) = actors.iter().find(|a| a.player.id == bubble.id) {
            *visibility = if actor.player.map == fishing.map {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            transform.translation.x = actor.position.x.round();
            transform.translation.y = actor.position.y.round() + 44.0 + bubble.height / 2.0;
        }
    }
}

pub fn fit_window(
    window: Single<&Window>,
    mut camera: Single<(&mut Camera, &mut Projection), With<OuterCamera>>,
    target: Single<&RenderTarget, With<WorldCamera>>,
    mut images: ResMut<Assets<Image>>,
    settings: Res<crate::settings::Settings>,
    mut ui_scale: ResMut<UiScale>,
) {
    let available = window.physical_size();
    if available.min_element() == 0 {
        return;
    }
    let view = canvas_size(settings.pixel_scale());
    if let RenderTarget::Image(target) = &*target {
        if images
            .get(&target.handle)
            .expect("Pixel canvas exists")
            .size()
            != view
        {
            images.get_mut(&target.handle).unwrap().resize(Extent3d {
                width: view.x,
                height: view.y,
                depth_or_array_layers: 1,
            });
        }
    }
    let fit = (available.as_vec2() / view.as_vec2()).min_element();
    let scale = fit.floor().max(1.0).min(fit);
    let size = (view.as_vec2() * scale).as_uvec2();
    // The UI and pixel canvas share one centered viewport, including on HiDPI displays.
    camera.0.viewport = Some(Viewport {
        physical_position: (available - size) / 2,
        physical_size: size,
        ..default()
    });
    if let Projection::Orthographic(p) = &mut *camera.1 {
        p.scale = window.scale_factor() / scale;
    }
    ui_scale.0 = size.x as f32 / WINDOW_SIZE.x as f32 / window.scale_factor();
}

pub fn capture(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut window: Single<&mut Window>,
) {
    if keys.just_pressed(KeyCode::F11) {
        use bevy::window::{MonitorSelection, WindowMode};
        window.mode = if window.mode == WindowMode::Windowed {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        } else {
            window
                .resolution
                .set(WINDOW_SIZE.x as f32, WINDOW_SIZE.y as f32);
            WindowMode::Windowed
        };
    }
    if keys.just_pressed(KeyCode::F12) {
        if let Err(e) = std::fs::create_dir_all("artifacts") {
            error!("Cannot save screenshot: {e}");
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("artifacts/yapshire-{stamp}.png")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadows_match_physics_at_tile_and_object_edges() {
        for object_surface in [false, true] {
            let mut world = yapshire_shared::World::bundled();
            let path = world.content.manifest.maps[0].path.clone();
            let gid = world.content.gid("yapshire:deck/0_0").unwrap();
            let map = world.maps.get_mut("yapshire:town").unwrap();
            let width = map.width as usize;
            for layer in &mut map.layers {
                layer.data.fill(0);
                layer.objects.retain(|object| object.kind != "solid");
            }
            for col in 0..width {
                map.layers[2].data[13 * width + col] = gid;
            }
            if object_surface {
                let object = serde_json::from_value(serde_json::json!({
                    "id": 999, "name": "Platform", "type": "solid",
                    "x": 224.0, "y": map.origin_y() - 32.0,
                    "width": 16.0, "height": 16.0
                }))
                .unwrap();
                map.layers
                    .iter_mut()
                    .find(|layer| layer.kind == "objectgroup")
                    .unwrap()
                    .objects
                    .push(object);
            } else {
                map.layers[2].data[11 * width + 14] = gid;
            }
            for (x, supported) in [
                (218.0, false),
                (218.01, false),
                (218.02, true),
                (245.98, true),
                (245.99, false),
                (246.0, false),
            ] {
                let mut position = Vec2::new(x, 32.0);
                assert_eq!(
                    shadow_ground_y(position, map, &world.content, &path),
                    Some(if supported { 32.0 } else { 0.0 }),
                    "x={x}, object_surface={object_surface}"
                );
                let mut velocity = 0.0;
                step(
                    &mut position,
                    &mut velocity,
                    0.0,
                    false,
                    true,
                    1.0 / 60.0,
                    map,
                    &world.content,
                    &path,
                );
                assert_eq!(velocity > 0.0, supported, "Only supported feet can jump");
            }
        }
    }

    #[test]
    fn jumping_shadows_stay_on_the_ground_and_platforms() {
        for (map_id, ground) in [("yapshire:town", 0.0), ("yapshire:tackle_shop", 32.0)] {
            let mut maps = crate::maps::Maps::load(std::path::Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets"
            )))
            .unwrap();
            if ground > 0.0 {
                let platform = maps.world.content.gid("yapshire:deck/0_0").unwrap();
                let map = maps.world.maps.get_mut(map_id).unwrap();
                for col in 14..=18 {
                    map.layers[2].data[11 * map.width as usize + col] = platform;
                }
            }
            let world = maps.world.clone();
            let map = &world.maps[map_id];
            let path = &world
                .content
                .manifest
                .maps
                .iter()
                .find(|m| m.id == map_id)
                .unwrap()
                .path;
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, TransformPlugin))
                .insert_resource(maps)
                .add_systems(Update, animate);
            let art = Art {
                font: default(),
                people: default(),
                atlas: default(),
                shadow: default(),
                items: default(),
                items_atlas: default(),
                panel: default(),
                slot: default(),
                water: default(),
                editor_icons: default(),
            };
            let player = Player {
                id: 1,
                name: "Shadow test".into(),
                map: map_id.into(),
                x: 244.0,
                y: ground,
                moving: false,
                facing: false,
                indoors: false,
                fishing: false,
            };
            let mut queue = bevy::ecs::world::CommandQueue::default();
            spawn_actor(&mut Commands::new(&mut queue, app.world()), &art, player);
            queue.apply(app.world_mut());
            let actor = app
                .world_mut()
                .query_filtered::<Entity, With<Actor>>()
                .single(app.world())
                .unwrap();
            let shadow = app
                .world_mut()
                .query_filtered::<Entity, With<Shadow>>()
                .single(app.world())
                .unwrap();
            let mut peak = ground;
            for frame in 0..60 {
                {
                    let mut actor = app.world_mut().get_mut::<Actor>(actor).unwrap();
                    let Actor {
                        position,
                        velocity_y,
                        ..
                    } = &mut *actor;
                    step(
                        position,
                        velocity_y,
                        1.0,
                        false,
                        frame == 0,
                        1.0 / 60.0,
                        map,
                        &world.content,
                        path,
                    );
                    actor.player.x = actor.position.x;
                    actor.player.y = actor.position.y;
                }
                app.update();
                let actor_position = app
                    .world()
                    .get::<GlobalTransform>(actor)
                    .unwrap()
                    .translation();
                let shadow_position = app
                    .world()
                    .get::<GlobalTransform>(shadow)
                    .unwrap()
                    .translation();
                assert_eq!(
                    actor_position.truncate(),
                    app.world().get::<Actor>(actor).unwrap().position.round(),
                    "The rendered character must follow its moving jump"
                );
                peak = peak.max(actor_position.y);
                assert_eq!(shadow_position.x, actor_position.x);
                assert_eq!(
                    shadow_position.y,
                    ground + 1.0,
                    "Shadow must remain on its supporting surface"
                );
                assert_eq!(
                    app.world().get::<Transform>(shadow).unwrap().scale,
                    Vec3::ONE
                );
                assert_eq!(
                    app.world().get::<Visibility>(shadow),
                    Some(&Visibility::Inherited)
                );
            }
            assert!(peak > ground + 30.0, "Exercise a complete jump");
            assert_eq!(app.world().get::<Actor>(actor).unwrap().position.y, ground);
        }
    }

    #[test]
    fn wide_maps_do_not_expose_panorama_edges() {
        let backdrop = Backdrop {
            base: Vec2::new(810.0, 124.0),
            parallax: 0.44,
        };
        assert_eq!(backdrop.x(360.0, 1620.0, WIDTH), 810.0);
        assert_eq!(backdrop.x(860.0, 1620.0, WIDTH), 1030.0);
        for scale in 2..=4 {
            let width = canvas_size(scale).x as f32;
            for camera in [120.0, 260.0, 1200.0, 1920.0, 8000.0] {
                let x = backdrop.x(camera, 1620.0, width);
                assert!(x - 810.0 <= camera - width / 2.0);
                assert!(x + 810.0 >= camera + width / 2.0);
                assert_eq!(backdrop.x(camera, width, width), camera);
            }
        }
    }

    #[test]
    fn scene_and_ui_share_the_same_viewport_across_display_sizes() {
        let mut app = App::new();
        app.init_resource::<UiScale>()
            .init_resource::<crate::settings::Settings>()
            .init_resource::<Assets<Image>>()
            .add_systems(Update, fit_window);
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.world_mut()
            .spawn((WorldCamera, RenderTarget::Image(image.clone().into())));
        let window = app.world_mut().spawn(Window::default()).id();
        let camera = app
            .world_mut()
            .spawn((
                Camera::default(),
                Projection::Orthographic(OrthographicProjection::default_2d()),
                OuterCamera,
            ))
            .id();
        for (scale, width, height, dpi, size, position) in [
            (2, 1440, 810, 1.0, (1440, 810), (0, 0)),
            (2, 2560, 1440, 1.0, (2160, 1215), (200, 112)),
            (2, 3440, 1440, 1.0, (2160, 1215), (640, 112)),
            (2, 1080, 2560, 1.0, (720, 405), (180, 1077)),
            (2, 3840, 2160, 2.0, (3600, 2025), (120, 67)),
            (2, 2160, 1215, 1.5, (2160, 1215), (0, 0)),
            (2, 320, 180, 1.0, (320, 180), (0, 0)),
            (3, 1440, 810, 1.0, (1440, 810), (0, 0)),
            (3, 2560, 1440, 1.0, (2400, 1350), (80, 45)),
            (3, 3840, 2160, 2.0, (3840, 2160), (0, 0)),
            (4, 1440, 810, 1.0, (1440, 808), (0, 1)),
            (4, 2560, 1440, 1.0, (2520, 1414), (20, 13)),
            (4, 3840, 2160, 2.0, (3600, 2020), (120, 70)),
            (4, 1080, 2560, 1.0, (1080, 606), (0, 977)),
        ] {
            app.world_mut()
                .resource_mut::<crate::settings::Settings>()
                .choose_scale(scale);
            let mut w = app.world_mut().get_mut::<Window>(window).unwrap();
            w.resolution.set_scale_factor(dpi);
            w.resolution.set_physical_resolution(width, height);
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera)
                .unwrap()
                .viewport
                .as_ref()
                .unwrap();
            assert_eq!(viewport.physical_size, UVec2::from(size));
            assert_eq!(viewport.physical_position, UVec2::from(position));
            let ui_size =
                viewport.physical_size.as_vec2() / (dpi * app.world().resource::<UiScale>().0);
            let view = canvas_size(scale).as_vec2();
            assert!((ui_size - view / view.x * WINDOW_SIZE.x as f32).length() < 0.01);
            let Projection::Orthographic(p) = app.world().get::<Projection>(camera).unwrap() else {
                panic!("Expected orthographic camera");
            };
            let scene_size = viewport.physical_size.as_vec2() / dpi * p.scale;
            assert!((scene_size - view).length() < 0.01);
            assert_eq!(
                app.world()
                    .resource::<Assets<Image>>()
                    .get(&image)
                    .unwrap()
                    .size(),
                canvas_size(scale)
            );
        }
    }

    #[test]
    fn keyboard_events_reach_game_controls() {
        use bevy::input::{
            ButtonState, InputPlugin,
            keyboard::{Key, KeyboardInput},
        };
        let mut app = App::new();
        app.add_plugins(InputPlugin);
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyD,
            logical_key: Key::Character("d".into()),
            state: ButtonState::Pressed,
            text: Some("d".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
        assert!(
            app.world()
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::KeyD)
        );
    }
    #[test]
    fn walking_jumping_and_bounds() {
        let world = yapshire_shared::World::bundled();
        let map = &world.maps["yapshire:town"];
        let mut pos = Vec2::new(244.0, 0.0);
        let mut vy = 0.0;
        for _ in 0..60 {
            step(
                &mut pos,
                &mut vy,
                1.0,
                false,
                false,
                1.0 / 60.0,
                map,
                &world.content,
                "maps/town.tmj",
            );
        }
        assert!((pos.x - 306.0).abs() < 0.1);
        step(
            &mut pos,
            &mut vy,
            0.0,
            false,
            true,
            1.0 / 60.0,
            map,
            &world.content,
            "maps/town.tmj",
        );
        assert!(pos.y > 0.0);
        for _ in 0..180 {
            step(
                &mut pos,
                &mut vy,
                -1.0,
                true,
                false,
                1.0 / 60.0,
                map,
                &world.content,
                "maps/town.tmj",
            );
        }
        assert_eq!(pos, Vec2::new(12.0, 0.0));
        assert_eq!(vy, 0.0);
        for _ in 0..1000 {
            step(
                &mut pos,
                &mut vy,
                1.0,
                true,
                false,
                1.0 / 60.0,
                map,
                &world.content,
                "maps/town.tmj",
            );
        }
        assert!(
            pos.x < 1360.0,
            "falling off the pier returns to the map spawn"
        );
    }
    #[test]
    fn collision_tiles_drive_platforms_walls_and_ceilings() {
        let world = yapshire_shared::World::bundled();
        let mut map = world.maps["yapshire:tackle_shop"].clone();
        let platform = world.content.gid("yapshire:deck/0_0").unwrap();
        let solid = world.content.gid("yapshire:stone/2_0").unwrap();
        for x in 14..=16 {
            map.layers[2].data[11 * 30 + x] = platform;
        }
        let mut position = Vec2::new(244.0, 0.0);
        let mut velocity = 0.0;
        for frame in 0..90 {
            step(
                &mut position,
                &mut velocity,
                0.0,
                false,
                frame == 0,
                1.0 / 60.0,
                &map,
                &world.content,
                "maps/tackle-shop.tmj",
            );
        }
        assert_eq!(
            position.y, 32.0,
            "jump through a platform, then land on its top"
        );
        for row in 9..13 {
            map.layers[2].data[row * 30 + 17] = solid;
        }
        for _ in 0..120 {
            step(
                &mut position,
                &mut velocity,
                1.0,
                true,
                false,
                1.0 / 60.0,
                &map,
                &world.content,
                "maps/tackle-shop.tmj",
            );
        }
        assert_eq!(position.x, 266.0, "a wall blocks the player's body");
        for x in 14..=16 {
            map.layers[2].data[11 * 30 + x] = 0;
            map.layers[2].data[9 * 30 + x] = solid;
        }
        position = Vec2::new(244.0, 0.0);
        let mut peak = 0.0_f32;
        for frame in 0..90 {
            step(
                &mut position,
                &mut velocity,
                0.0,
                false,
                frame == 0,
                1.0 / 60.0,
                &map,
                &world.content,
                "maps/tackle-shop.tmj",
            );
            peak = peak.max(position.y);
        }
        assert_eq!(peak, 22.0, "the head stops at the ceiling");
        assert_eq!(position.y, 0.0);
        position = Vec2::new(80.0, 40.0);
        velocity = -1000.0;
        step(
            &mut position,
            &mut velocity,
            0.0,
            false,
            false,
            0.05,
            &map,
            &world.content,
            "maps/tackle-shop.tmj",
        );
        assert_eq!(
            position.y, 0.0,
            "a fast fall must not pass through the floor"
        );
        assert_eq!(velocity, 0.0);
    }
    #[test]
    fn bubble_is_readable_and_bounded() {
        let text = bubble_text(&"你好呀".repeat(40));
        assert_eq!(text.lines().count(), 3);
        assert!(text.ends_with('…'));
    }
}
