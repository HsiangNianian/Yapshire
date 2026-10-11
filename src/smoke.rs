//! Opt-in GPU acceptance run. Input goes through game systems, never the desktop.
use crate::{
    Session,
    game::Actor,
    ui::{Action, Menu, Page},
};
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::WindowCloseRequested,
};

#[derive(Resource)]
pub struct Smoke {
    mode: String,
    stage: u8,
    since: f32,
    start_x: f32,
    room: usize,
    recording: Option<std::path::PathBuf>,
    frame: u32,
    last_frame: f32,
    local_world: Option<yapshire_shared::World>,
}
impl Default for Smoke {
    fn default() -> Self {
        Self {
            mode: std::env::var("YAPSHIRE_SMOKE").unwrap_or_default(),
            stage: 0,
            since: 0.0,
            start_x: 0.0,
            room: 0,
            recording: std::env::var_os("YAPSHIRE_RECORD").map(Into::into),
            frame: 0,
            last_frame: 0.0,
            local_world: None,
        }
    }
}

pub fn drive(
    mut commands: Commands,
    mut smoke: ResMut<Smoke>,
    time: Res<Time>,
    mut menu: ResMut<Menu>,
    mut clubs: ResMut<crate::clubs::Browser>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    session: Res<Session>,
    maps: Res<crate::maps::Maps>,
    actors: Query<&Actor>,
    mut buttons: Query<(&Action, &mut Interaction)>,
    mut keyboard: MessageWriter<KeyboardInput>,
    window: Single<(Entity, &Window)>,
    mut close: MessageWriter<WindowCloseRequested>,
) {
    if smoke.mode.is_empty()
        || smoke.mode.starts_with("fishing")
        || smoke.mode.starts_with("editor")
        || smoke.mode.starts_with("i18n")
        || smoke.mode.starts_with("clubs")
    {
        return;
    }
    mouse.release(MouseButton::Left);
    let now = time.elapsed_secs();
    assert!(
        now < 90.0,
        "GPU smoke timed out in stage {}: {}",
        smoke.stage,
        menu.status
    );
    let host = smoke.mode.starts_with("host");
    let cloud = smoke.mode.ends_with("cloud");
    let name = if host { "Rowan" } else { "June" };
    let mine = actors.iter().find(|a| Some(a.player.id) == session.you);
    let capture = |commands: &mut Commands, mode: &str, tag: &str| {
        std::fs::create_dir_all("artifacts").unwrap();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("artifacts/{mode}-{tag}.png")));
    };
    let key =
        |keyboard: &mut MessageWriter<KeyboardInput>, key_code, logical_key, text, pressed| {
            keyboard.write(KeyboardInput {
                key_code,
                logical_key,
                text,
                state: if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
                repeat: false,
                window: window.0,
            });
        };
    if smoke.mode == "display" {
        let advance = match smoke.stage {
            0 | 2 | 4 if now - smoke.since > 3.0 => {
                let fullscreen = smoke.stage == 2;
                assert_eq!(
                    window.1.mode != bevy::window::WindowMode::Windowed,
                    fullscreen
                );
                assert!(!window.1.resizable && !window.1.enabled_buttons.maximize);
                if !fullscreen {
                    assert_eq!(window.1.size(), crate::game::WINDOW_SIZE.as_vec2());
                }
                let tag = match smoke.stage {
                    0 => "window",
                    2 => "fullscreen",
                    _ => "restored",
                };
                info!(
                    "GPU SMOKE display {tag}: {:?} physical pixels",
                    window.1.physical_size()
                );
                capture(&mut commands, &smoke.mode, tag);
                if smoke.stage != 4 {
                    key(&mut keyboard, KeyCode::F11, Key::F11, None, true);
                }
                true
            }
            1 | 3 if now - smoke.since > 0.25 => {
                key(&mut keyboard, KeyCode::F11, Key::F11, None, false);
                true
            }
            5 if now - smoke.since > 1.0 => {
                info!("GPU SMOKE display PASS: fixed window, F11 fullscreen, restored size");
                close.write(WindowCloseRequested { window: window.0 });
                true
            }
            _ => false,
        };
        if advance {
            smoke.stage += 1;
            smoke.since = now;
        }
        return;
    }
    let mut action = None;
    // Window focus changes can clear held input while recording two clients.
    if smoke.stage == 4 && smoke.mode != "readme" {
        let (code, letter) = if host {
            (KeyCode::KeyD, "d")
        } else {
            (KeyCode::KeyA, "a")
        };
        key(
            &mut keyboard,
            code,
            Key::Character(letter.into()),
            None,
            true,
        );
    }
    let advance = if smoke.mode == "readme" {
        match smoke.stage {
            0 if now > 3.0 => {
                capture(&mut commands, "readme", "home");
                true
            }
            1 if now - smoke.since > 0.6 => {
                action = Some(Action::Go(Page::Host));
                true
            }
            2 if now - smoke.since > 0.6 => {
                menu.room_name = "Lakeside Friends".into();
                action = Some(Action::Hosting(true));
                true
            }
            3 if now - smoke.since > 0.8 => {
                capture(&mut commands, "readme", "host");
                true
            }
            4 if now - smoke.since > 0.8 => {
                info!("README CAPTURE PASS: native home and online host form");
                close.write(WindowCloseRequested { window: window.0 });
                true
            }
            _ => false,
        }
    } else {
        match smoke.stage {
            0 if now > 3.0 => {
                smoke.local_world = Some(maps.world().unwrap());
                if cloud {
                    if let Ok(server) = std::env::var("YAPSHIRE_TEST_SERVER") {
                        *clubs = crate::clubs::Browser::new(
                            vec![crate::clubs::SavedClub::new("Smoke Club", &server).unwrap()],
                            &server,
                        );
                        menu.server = server;
                    }
                    menu.server_password =
                        std::env::var("YAPSHIRE_TEST_PASSWORD").unwrap_or_default();
                }
                capture(&mut commands, &smoke.mode, "menu");
                menu.name = name.into();
                menu.room_name = "Sunset Club".into();
                action = Some(Action::Go(if host {
                    Page::Host
                } else if cloud {
                    Page::Cloud
                } else {
                    Page::Lan
                }));
                true
            }
            1 if now - smoke.since > 0.5 => {
                if host {
                    action = Some(Action::Hosting(cloud));
                    capture(&mut commands, &smoke.mode, "host");
                    true
                } else if let Some(index) = menu
                    .rooms
                    .iter()
                    .position(|r| r.name == if cloud { "Sunset Club" } else { "Rowan" })
                {
                    smoke.room = index;
                    capture(&mut commands, &smoke.mode, "lobby");
                    true
                } else {
                    false
                }
            }
            2 if now - smoke.since > 0.8 => {
                action = Some(if host {
                    Action::Connect
                } else {
                    Action::JoinRoom(smoke.room)
                });
                true
            }
            3 if session.connected && actors.iter().count() >= 2 => {
                if let Some(folder) = std::env::var_os("YAPSHIRE_TEST_MAPS") {
                    let expected =
                        yapshire_shared::World::load(std::path::Path::new(&folder)).unwrap();
                    assert_eq!(
                        maps.world().unwrap(),
                        expected,
                        "Runtime must use server maps"
                    );
                    assert_eq!(
                        session.local_world, smoke.local_world,
                        "Keep the local world for leaving"
                    );
                    info!(
                        "GPU SMOKE {} verified server world {}",
                        smoke.mode, expected.revision
                    );
                }
                smoke.start_x = mine.unwrap().position.x;
                let (code, letter) = if host {
                    (KeyCode::KeyD, "d")
                } else {
                    (KeyCode::KeyA, "a")
                };
                key(
                    &mut keyboard,
                    code,
                    Key::Character(letter.into()),
                    None,
                    true,
                );
                key(&mut keyboard, KeyCode::Space, Key::Space, None, true);
                true
            }
            4 if now - smoke.since > 1.5 => {
                let moved = (mine.unwrap().position.x - smoke.start_x).abs();
                assert!(moved > 40.0, "Real keyboard movement failed: {moved}");
                info!("GPU SMOKE {} walked {:.1} world pixels", smoke.mode, moved);
                let (code, letter) = if host {
                    (KeyCode::KeyD, "d")
                } else {
                    (KeyCode::KeyA, "a")
                };
                key(
                    &mut keyboard,
                    code,
                    Key::Character(letter.into()),
                    None,
                    false,
                );
                key(&mut keyboard, KeyCode::Space, Key::Space, None, false);
                key(&mut keyboard, KeyCode::Enter, Key::Enter, None, true);
                true
            }
            5 if now - smoke.since > 0.25 => {
                key(&mut keyboard, KeyCode::Enter, Key::Enter, None, false);
                key(
                    &mut keyboard,
                    KeyCode::KeyH,
                    Key::Character("h".into()),
                    Some(format!("Hello from {name}!").into()),
                    true,
                );
                true
            }
            6 if now - smoke.since > 0.25 => {
                key(
                    &mut keyboard,
                    KeyCode::KeyH,
                    Key::Character("h".into()),
                    None,
                    false,
                );
                key(&mut keyboard, KeyCode::Enter, Key::Enter, None, true);
                true
            }
            7 if now - smoke.since > 1.0
                && ["Hello from Rowan!", "Hello from June!"]
                    .iter()
                    .all(|s| session.log.iter().any(|line| line.to_string().contains(s))) =>
            {
                capture(&mut commands, &smoke.mode, "chat");
                info!(
                    "GPU SMOKE {} PASS: lobby join, walking, chat received by both players",
                    smoke.mode
                );
                true
            }
            8 if now - smoke.since > 5.0 => {
                menu.leave = true;
                menu.go(Page::Home);
                true
            }
            9 if now - smoke.since > 1.0 => {
                assert!(!session.connected && actors.is_empty());
                assert_eq!(
                    Some(maps.world().unwrap()),
                    smoke.local_world,
                    "Leaving restores local editor maps"
                );
                info!(
                    "GPU SMOKE {} PASS: local maps restored after leaving",
                    smoke.mode
                );
                close.write(WindowCloseRequested { window: window.0 });
                true
            }
            _ => false,
        }
    };
    if let Some(action) = action {
        let mut found = false;
        for (candidate, mut interaction) in &mut buttons {
            let matches = match (candidate, action) {
                (Action::Go(a), Action::Go(b)) => *a == b,
                (Action::Hosting(a), Action::Hosting(b)) => *a == b,
                (Action::Connect, Action::Connect) => true,
                (Action::JoinRoom(a), Action::JoinRoom(b)) => *a == b,
                (Action::Club(crate::clubs::Action::Join(_, code)), Action::JoinRoom(index)) => {
                    menu.rooms
                        .get(index)
                        .is_some_and(|room| room.code.as_bytes() == code)
                }
                _ => false,
            };
            if matches {
                *interaction = Interaction::Pressed;
                mouse.press(MouseButton::Left);
                found = true;
                break;
            }
        }
        assert!(found, "Expected menu button is missing");
    }
    if advance {
        smoke.stage += 1;
        smoke.since = now;
    }
}

/// Capture the real rendered multiplayer session for README media.
pub fn record(mut commands: Commands, mut smoke: ResMut<Smoke>, time: Res<Time>) {
    let Some(directory) = smoke.recording.as_ref() else {
        return;
    };
    let now = time.elapsed_secs();
    if !(4..=8).contains(&smoke.stage)
        || (smoke.stage == 8 && now - smoke.since > 4.0)
        || now - smoke.last_frame < 1.0 / 15.0
    {
        return;
    }
    std::fs::create_dir_all(directory).expect("Create recording directory");
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(
            directory.join(format!("{:05}.png", smoke.frame)),
        ));
    smoke.last_frame = if smoke.frame == 0 {
        now
    } else {
        smoke.last_frame + 1.0 / 15.0
    };
    smoke.frame += 1;
}
