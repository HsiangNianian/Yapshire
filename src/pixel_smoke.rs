//! Drives the actual native brush, WebSocket room, texture upload and player collision.
use crate::{
    Session,
    game::{self, Actor, OuterCamera, WorldCamera},
    maps::Maps,
    pixels::Brush,
    settings::Settings,
    ui::{Chat, Menu},
};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};

#[derive(Resource, Default)]
pub(crate) struct Check {
    stage: u8,
    since: f32,
    captures: u8,
}

pub(crate) fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut check: ResMut<Check>,
    mut menu: ResMut<Menu>,
    mut chat: ResMut<Chat>,
    mut settings: ResMut<Settings>,
    session: Res<Session>,
    maps: Res<Maps>,
    brush: Res<Brush>,
    actors: Query<&Actor>,
    mut window: Single<&mut Window>,
    cameras: (
        Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
        Single<&Camera, (With<OuterCamera>, Without<WorldCamera>)>,
    ),
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut exit: MessageWriter<AppExit>,
) {
    if std::env::var("YAPSHIRE_SMOKE").as_deref() != Ok("pixels") {
        return;
    }
    let now = time.elapsed_secs();
    assert!(now < 90.0, "Pixel smoke timed out at {}", check.stage);
    keys.release_all();
    mouse.release_all();
    let elapsed = now - check.since;
    let canvas_size = game::canvas_size(settings.pixel_scale()).as_vec2();
    let point = |window: &mut Window, x: f32, y: f32| {
        let canvas = cameras
            .0
            .0
            .world_to_viewport(cameras.0.1, Vec3::new(x, 208.0 - y, 0.0))
            .unwrap();
        let viewport = cameras.1.physical_viewport_rect().unwrap();
        let physical = viewport.min.as_vec2() + canvas / canvas_size * viewport.size().as_vec2();
        window.set_physical_cursor_position(Some(physical.as_dvec2()));
    };
    let capture = |commands: &mut Commands, name| {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("artifacts/pixels-{name}.png")))
            .observe(|_: On<ScreenshotCaptured>, mut check: ResMut<Check>| {
                check.captures += 1;
            });
    };
    let mine = actors.iter().find(|a| Some(a.player.id) == session.you);
    let grid = maps.pixels.get("yapshire:town").unwrap();
    match check.stage {
        0 if now > 1.0 => {
            menu.name = "Pixel QA".into();
            menu.request = Some(crate::network::Mode::HostLan(0));
        }
        1 if session.connected && elapsed > 0.5 => {
            keys.press(KeyCode::KeyV);
        }
        2 if elapsed > 0.5 => {
            assert!(brush.open);
            keys.press(KeyCode::Digit2);
        }
        3 => {
            point(&mut window, 270.0, 176.0);
            mouse.press(MouseButton::Left);
            if elapsed < 1.8 {
                return;
            }
            assert!(
                (160..220).any(|y| (248..288).any(|x| grid.get(x, y) & 3 == 2)),
                "Sand must arrive from the server"
            );
            capture(&mut commands, "sand");
            keys.press(KeyCode::Digit3);
        }
        4 => {
            point(&mut window, 281.0, 174.0);
            mouse.press(MouseButton::Left);
            if elapsed < 1.8 {
                return;
            }
            assert!(
                (165..220).any(|y| (256..310).any(|x| grid.get(x, y) & 3 == 3)),
                "Water must flow into the scene"
            );
            capture(&mut commands, "water");
            keys.press(KeyCode::Digit1);
        }
        5 => {
            point(
                &mut window,
                238.0 + (elapsed * 5.0) as u32 as f32 % 3.0 * 6.0,
                210.0,
            );
            mouse.press(MouseButton::Left);
            if elapsed < 2.2 {
                return;
            }
            assert!(
                !grid.solid(244, 210),
                "Digging must remove ground; water may flow into the hole"
            );
            assert!(
                mine.unwrap().position.y < -2.0,
                "Player collision must follow the hole: {:?}",
                mine.unwrap().position
            );
            capture(&mut commands, "dug");
            chat.open = true;
        }
        6 => {
            point(&mut window, 212.0, 212.0);
            mouse.press(MouseButton::Left);
            if elapsed < 0.5 {
                return;
            }
            assert!(grid.solid(212, 212), "Chat must block the material brush");
            chat.open = false;
            settings.choose_scale(3);
        }
        7 if elapsed > 0.8 => {}
        8 => {
            point(&mut window, 212.0, 212.0);
            mouse.press(MouseButton::Left);
            if elapsed < 0.6 {
                return;
            }
            assert!(
                !grid.solid(212, 212),
                "3x mouse projection must hit the targeted pixel"
            );
            settings.choose_scale(4);
        }
        9 if elapsed > 0.8 => {}
        10 => {
            point(&mut window, 214.0, 223.0);
            mouse.press(MouseButton::Left);
            if elapsed < 0.6 {
                return;
            }
            assert!(
                !grid.solid(214, 223),
                "4x mouse projection must hit the targeted pixel"
            );
            capture(&mut commands, "4x");
        }
        11 if elapsed > 0.6 && check.captures == 4 => {
            info!(
                "PIXEL GPU PASS: actual LAN brush, sand, water, digging, updated player collision, chat input isolation, 2x/3x/4x mouse coordinates and chunk texture upload"
            );
            exit.write(AppExit::Success);
        }
        _ => return,
    }
    check.stage += 1;
    check.since = now;
    info!("Pixel smoke completed step {}", check.stage);
}
