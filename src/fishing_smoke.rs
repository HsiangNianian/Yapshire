//! Opt-in native fishing acceptance: real movement, shop controls and reel input.
use crate::{
    Session,
    fishing::{self, Fishing, Panel, Progress, Stage},
    game::Actor,
    network::Mode,
    ui::{Chat, Menu},
};
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

#[derive(Resource, Default)]
pub(crate) struct Check {
    stage: u8,
    since: f32,
    release: Vec<KeyCode>,
    reel_shot: bool,
    value: u32,
    transition_shots: u8,
    cast_shot: bool,
    float_shot: bool,
}

fn key(writer: &mut MessageWriter<KeyboardInput>, window: Entity, code: KeyCode, pressed: bool) {
    let logical_key = match code {
        KeyCode::Space => Key::Space,
        KeyCode::Escape => Key::Escape,
        KeyCode::Enter => Key::Enter,
        KeyCode::KeyE => Key::Character("e".into()),
        _ => Key::Character("".into()),
    };
    writer.write(KeyboardInput {
        key_code: code,
        logical_key,
        state: if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
        text: None,
        repeat: false,
        window,
    });
}

pub(crate) fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut check: ResMut<Check>,
    mut menu: ResMut<Menu>,
    session: Res<Session>,
    fishing: Res<Fishing>,
    chat: Res<Chat>,
    actors: Query<&Actor>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<Entity, With<Window>>,
    mut keyboard: MessageWriter<KeyboardInput>,
    mut exit: MessageWriter<AppExit>,
    mut buttons: Query<(&crate::ui::Action, &mut Interaction)>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    icons: Query<&ImageNode, With<crate::fishing_ui::ItemIcon>>,
    images: Res<Assets<Image>>,
) {
    let mode = std::env::var("YAPSHIRE_SMOKE").unwrap_or_default();
    if !mode.starts_with("fishing") {
        return;
    }
    assert!(
        std::env::var_os("YAPSHIRE_SAVE_DIR").is_some(),
        "Fishing smoke requires an isolated YAPSHIRE_SAVE_DIR"
    );
    let now = time.elapsed_secs();
    assert!(
        now < 100.0,
        "Fishing GPU smoke timed out at step {}: {}",
        check.stage,
        menu.status
    );
    for code in check.release.drain(..) {
        key(&mut keyboard, *window, code, false);
    }
    let mine = actors.iter().find(|a| Some(a.player.id) == session.you);
    let x = mine.map_or(0.0, |a| a.position.x);
    let capture = |commands: &mut Commands, name: &str| {
        std::fs::create_dir_all("artifacts").unwrap();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("artifacts/{mode}-{name}.png")));
    };
    let mut tap = None;
    if check.stage == 2 && x >= 800.0 && check.transition_shots == 0 {
        capture(&mut commands, "street-to-shop");
        check.transition_shots = 1;
    }
    if check.stage == 13 && x >= 1110.0 && check.transition_shots == 1 {
        capture(&mut commands, "quay-to-pier");
        check.transition_shots = 2;
    }
    if check.stage == 14 && matches!(fishing.stage, Stage::Waiting(_)) {
        if fishing.anim_time > 0.3 && !check.cast_shot {
            capture(&mut commands, "casting");
            check.cast_shot = true;
        }
        if fishing.anim_time > 1.2 && !check.float_shot {
            capture(&mut commands, "float");
            check.float_shot = true;
        }
    }
    let verify_icons = || {
        for index in 0..8 {
            let node = icons
                .iter()
                .find(|image| {
                    image
                        .texture_atlas
                        .as_ref()
                        .is_some_and(|atlas| atlas.index == index)
                })
                .expect("Inventory must render every tackle and fish sprite");
            assert!(
                images.get(&node.image).is_some(),
                "Item texture must be loaded on the GPU path"
            );
        }
    };
    // Keep the injected walk held if another test window takes focus.
    if matches!(check.stage, 2 | 4 | 11 | 13 | 19 | 21) {
        let direction = if matches!(check.stage, 11 | 19) {
            KeyCode::KeyA
        } else {
            KeyCode::KeyD
        };
        key(&mut keyboard, *window, direction, true);
        key(&mut keyboard, *window, KeyCode::ShiftLeft, true);
    }
    let advance = match check.stage {
        0 if now > 2.0 => {
            menu.name = "Rowan".into();
            menu.request = Some(if mode.ends_with("cloud") {
                Mode::HostCloud {
                    server: std::env::var("YAPSHIRE_TEST_SERVER").expect("Set local Worker URL"),
                    room_name: "Fishing smoke".into(),
                }
            } else {
                Mode::HostLan(4777)
            });
            true
        }
        1 if session.connected && mine.is_some() => {
            assert_eq!(
                fishing.progress,
                Progress::default(),
                "Use a fresh save directory"
            );
            key(&mut keyboard, *window, KeyCode::KeyD, true);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, true);
            true
        }
        2 if x >= fishing::SHOP_DOOR - 3.0 => {
            key(&mut keyboard, *window, KeyCode::KeyD, false);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, false);
            tap = Some(KeyCode::KeyE);
            true
        }
        3 if fishing.indoors && now - check.since > 0.5 => {
            capture(&mut commands, "interior");
            key(&mut keyboard, *window, KeyCode::KeyD, true);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, true);
            true
        }
        4 if x >= fishing::COUNTER - 8.0 => {
            key(&mut keyboard, *window, KeyCode::KeyD, false);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, false);
            tap = Some(KeyCode::KeyE);
            true
        }
        5 if fishing.panel == Panel::Shop && now - check.since > 0.5 => {
            capture(&mut commands, "shop");
            tap = Some(KeyCode::Digit1);
            true
        }
        6 if fishing.progress.rod => {
            tap = Some(KeyCode::Digit2);
            true
        }
        7 if fishing.progress.hook => {
            let (_, mut interaction) = buttons
                .iter_mut()
                .find(|(action, _)| {
                    matches!(action, crate::ui::Action::Fishing(fishing::Action::Bait))
                })
                .expect("Bait purchase button");
            *interaction = Interaction::Pressed;
            mouse.press(MouseButton::Left);
            true
        }
        8 if fishing.progress.bait == 5 && now - check.since > 0.5 => {
            mouse.release(MouseButton::Left);
            assert_eq!(fishing.progress.coins, 30);
            capture(&mut commands, "purchases");
            tap = Some(KeyCode::KeyI);
            true
        }
        9 if fishing.panel == Panel::Bag && now - check.since > 0.6 => {
            verify_icons();
            capture(&mut commands, "bag-tackle");
            tap = Some(KeyCode::KeyI);
            true
        }
        10 if fishing.panel == Panel::None => {
            key(&mut keyboard, *window, KeyCode::KeyA, true);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, true);
            true
        }
        11 if x <= fishing::SHOP_EXIT + 3.0 => {
            key(&mut keyboard, *window, KeyCode::KeyA, false);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, false);
            tap = Some(KeyCode::KeyE);
            true
        }
        12 if !fishing.indoors => {
            key(&mut keyboard, *window, KeyCode::KeyD, true);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, true);
            true
        }
        13 if x >= fishing::PIER_START + 40.0 => {
            key(&mut keyboard, *window, KeyCode::KeyD, false);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, false);
            capture(&mut commands, "coast");
            tap = Some(KeyCode::KeyE);
            true
        }
        14 if matches!(fishing.stage, Stage::Result { fish: None, .. }) => {
            assert_eq!(fishing.progress.bait, 4);
            assert_eq!(fishing.progress.value(), 0);
            assert!(!chat.open);
            capture(&mut commands, "missed-bite");
            tap = Some(KeyCode::Escape);
            true
        }
        15 if matches!(fishing.stage, Stage::Idle) => {
            tap = Some(KeyCode::KeyE);
            true
        }
        16 if matches!(fishing.stage, Stage::Bite(_)) => {
            capture(&mut commands, "bite");
            tap = Some(KeyCode::Space);
            true
        }
        17 => {
            if let Stage::Reeling(fight) = &fishing.stage {
                let held = keys.pressed(KeyCode::Space);
                if (held && fight.tension > 0.65) || (!held && fight.tension < 0.3) {
                    key(&mut keyboard, *window, KeyCode::Space, !held);
                }
                if fight.elapsed > 3.0 && !check.reel_shot {
                    capture(&mut commands, "reeling");
                    check.reel_shot = true;
                }
                false
            } else if let Stage::Result { fish, message } = &fishing.stage
                && fishing.anim_time > 1.0
            {
                assert!(fish.is_some(), "Controlled reeling failed: {message}");
                assert_eq!(fishing.progress.catches.iter().sum::<u32>(), 1);
                assert_eq!(fishing.progress.bait, 3);
                assert_eq!(fishing.progress.coins, 30);
                check.value = fishing.progress.value();
                capture(&mut commands, "catch");
                key(&mut keyboard, *window, KeyCode::Space, false);
                tap = Some(KeyCode::KeyI);
                true
            } else {
                false
            }
        }
        18 if fishing.panel == Panel::Bag && now - check.since > 0.6 => {
            verify_icons();
            capture(&mut commands, "bag-catch");
            tap = Some(KeyCode::KeyI);
            key(&mut keyboard, *window, KeyCode::KeyA, true);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, true);
            true
        }
        19 if x <= fishing::SHOP_DOOR + 3.0 => {
            key(&mut keyboard, *window, KeyCode::KeyA, false);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, false);
            tap = Some(KeyCode::KeyE);
            true
        }
        20 if fishing.indoors => {
            key(&mut keyboard, *window, KeyCode::KeyD, true);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, true);
            true
        }
        21 if x >= fishing::COUNTER - 8.0 => {
            key(&mut keyboard, *window, KeyCode::KeyD, false);
            key(&mut keyboard, *window, KeyCode::ShiftLeft, false);
            tap = Some(KeyCode::KeyE);
            true
        }
        22 if fishing.panel == Panel::Shop => {
            tap = Some(KeyCode::Digit4);
            true
        }
        23 if fishing.progress.value() == 0 && now - check.since > 0.5 => {
            assert_eq!(fishing.progress.coins, 30 + check.value);
            let saved: Progress = serde_json::from_slice(
                &std::fs::read(fishing::profile_path(&menu.name).unwrap()).unwrap(),
            )
            .unwrap();
            assert_eq!(
                saved, fishing.progress,
                "Purchases and catch sales must survive a restart"
            );
            capture(&mut commands, "sold");
            true
        }
        24 if now - check.since > 2.0 => {
            info!(
                "FISHING GPU PASS: initial wallet, shop icons, purchases, tackle inventory, open-water cast, missed bite, controlled catch, catch inventory, sell, persisted balance"
            );
            exit.write(AppExit::Success);
            true
        }
        _ => false,
    };
    if let Some(code) = tap {
        key(&mut keyboard, *window, code, true);
        check.release.push(code);
    }
    if advance {
        info!("Fishing smoke completed step {}", check.stage);
        check.stage += 1;
        check.since = now;
    }
}
