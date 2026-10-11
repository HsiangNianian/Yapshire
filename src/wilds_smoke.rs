//! Opt-in native acceptance. All data directories must be isolated.
use crate::{
    game::{self, Actor},
    i18n::{I18n, Language},
    settings::Settings,
    ui::{Menu, Page},
    wilds::{self, Action, Wilds},
};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use yapshire_shared::terrain::{Block, TILE};

#[derive(Resource, Default)]
pub(crate) struct Check {
    stage: u32,
    since: f32,
    cursor: Option<Vec2>,
    start: Vec2,
    menu_shot: bool,
}

pub(crate) fn cursor(check: Res<Check>, mut window: Single<&mut Window>) {
    if std::env::var("YAPSHIRE_SMOKE")
        .is_ok_and(|m| m.starts_with("wilds") && !m.starts_with("wilds-tiles"))
    {
        window.set_physical_cursor_position(check.cursor.map(|p| p.as_dvec2()));
    }
}

fn capture(commands: &mut Commands, name: &str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(format!("artifacts/wilds-{name}.png")));
}

pub(crate) fn drive(
    mut commands: Commands,
    mut check: ResMut<Check>,
    time: Res<Time>,
    mut menu: ResMut<Menu>,
    mut wilds: ResMut<Wilds>,
    mut settings: ResMut<Settings>,
    mut i18n: ResMut<I18n>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut window: Single<&mut Window>,
    outer: Single<&Camera, With<game::OuterCamera>>,
    camera: Single<&Transform, With<game::WorldCamera>>,
    mut actors: Query<&mut Actor>,
    chunks: Query<Entity, With<wilds::ChunkRoot>>,
    mut exit: MessageWriter<AppExit>,
    mut buttons: Query<(&crate::settings::Action, &mut Interaction)>,
) {
    let mode = std::env::var("YAPSHIRE_SMOKE").unwrap_or_default();
    if !mode.starts_with("wilds") || mode.starts_with("wilds-tiles") {
        return;
    }
    for key in [
        "YAPSHIRE_WORLD_DIR",
        "YAPSHIRE_SETTINGS_DIR",
        "YAPSHIRE_MAP_DIR",
        "YAPSHIRE_SAVE_DIR",
    ] {
        assert!(
            std::env::var_os(key).is_some(),
            "Set {key} for isolated acceptance"
        );
    }
    let now = time.elapsed_secs();
    assert!(now < 90.0, "World smoke timed out at {}", check.stage);
    if now - check.since < if check.stage == 8 { 0.2 } else { 0.8 } {
        return;
    }
    mouse.release_all();
    keys.release_all();
    let point = |cell: IVec2| {
        let viewport = outer.physical_viewport_rect().unwrap();
        let view = game::canvas_size(settings.pixel_scale()).as_vec2();
        let relative = ((cell.as_vec2() + 0.5) * TILE - camera.translation.truncate()) / view;
        viewport.min.as_vec2()
            + Vec2::new(relative.x + 0.5, 0.5 - relative.y) * viewport.size().as_vec2()
    };
    match check.stage {
        0 => {
            menu.go(Page::Worlds);
            menu.world_name = "Autumn Wilds".into();
            menu.world_seed = "42".into();
            settings.choose_scale(2);
            settings.choose_ui_scale(100);
            i18n.language = Language::English;
        }
        1 => {
            if !check.menu_shot {
                capture(&mut commands, "world-list-en");
                check.menu_shot = true;
                check.since = now;
                return;
            }
            wilds.pending = Some(if mode == "wilds-reload" {
                Action::Open(0)
            } else {
                Action::Create
            });
        }
        2 => {
            let run = wilds.active.as_ref().unwrap();
            if mode == "wilds-reload" {
                assert_eq!(run.saved.terrain.block(1026, 160), Block::Air);
                assert_eq!(run.saved.terrain.block(1027, 161), Block::Dirt);
                assert_eq!(run.saved.terrain.inventory, [1, 0, 0]);
                assert!(
                    Vec2::from_array(run.saved.terrain.position)
                        .distance(Vec2::from_array(run.saved.terrain.spawn()))
                        < 1.0
                );
                info!("WILDS RELOAD PASS: edited terrain, inventory and position restored");
                wilds.pending = Some(Action::Leave);
                check.stage = 30;
                check.since = now;
                return;
            }
            assert_eq!(run.saved.terrain.seed, 42);
            assert!(chunks.iter().count() <= 12);
            check.start = actors.single().unwrap().position;
            capture(&mut commands, "surface-en");
            check.cursor = Some(point(IVec2::new(1026, 160)));
            mouse.press(MouseButton::Left);
        }
        3 => {
            let run = wilds.active.as_ref().unwrap();
            assert_eq!(run.saved.terrain.block(1026, 160), Block::Air);
            assert_eq!(run.saved.terrain.inventory, [1, 0, 0]);
            check.cursor = Some(point(IVec2::new(1027, 161)));
            mouse.press(MouseButton::Right);
        }
        4 => {
            let run = wilds.active.as_ref().unwrap();
            assert_eq!(run.saved.terrain.block(1027, 161), Block::Dirt);
            assert_eq!(run.saved.terrain.inventory, [0, 0, 0]);
            capture(&mut commands, "built-en");
            // A second, separate edit must remain in the player's inventory.
            check.cursor = Some(point(IVec2::new(1022, 160)));
            mouse.press(MouseButton::Left);
        }
        5 => {
            assert_eq!(
                wilds.active.as_ref().unwrap().saved.terrain.inventory,
                [1, 0, 0]
            );
            check.cursor = None;
            keys.press(KeyCode::KeyA);
        }
        6 => {
            assert!(
                actors.single().unwrap().position.x < check.start.x - 25.0,
                "walking did not move"
            );
            wilds.pending = Some(Action::Recall);
        }
        7 => {
            keys.press(KeyCode::Space);
        }
        8 => {
            assert!(
                actors.single().unwrap().position.y > check.start.y + 4.0,
                "jump did not rise"
            );
        }
        9 => {
            let world = &wilds.active.as_ref().unwrap().saved.terrain;
            actors.single_mut().unwrap().teleport(Vec2::new(
                200.5 * TILE,
                (world.surface(200) + 1) as f32 * TILE,
            ));
        }
        10 => {
            assert!(chunks.iter().count() <= 12, "old chunks leaked");
            assert!(
                camera.translation.x < 4000.0,
                "camera did not follow streamed terrain"
            );
            capture(&mut commands, "distant-terrain");
        }
        11 => {
            wilds.pending = Some(Action::Recall);
            settings.choose_ui_scale(150);
            settings.choose_scale(4);
            i18n.language = Language::Chinese;
            capture(&mut commands, "surface-ui150-zh");
        }
        12 => {
            check.cursor = Some(point(IVec2::new(1025, 160)));
            mouse.press(MouseButton::Left);
            let (_, mut interaction) = buttons
                .iter_mut()
                .find(|(a, _)| **a == crate::settings::Action::Open)
                .unwrap();
            *interaction = Interaction::Pressed;
        }
        13 => {
            assert_eq!(
                wilds.active.as_ref().unwrap().saved.terrain.inventory,
                [1, 0, 0],
                "settings allowed mining through modal"
            );
            let (_, mut interaction) = buttons
                .iter_mut()
                .find(|(a, _)| **a == crate::settings::Action::Close)
                .unwrap();
            *interaction = Interaction::Pressed;
            mouse.press(MouseButton::Left);
            check.cursor = None;
            wilds.pending = Some(Action::Save);
        }
        14 => {
            wilds.pending = Some(Action::Leave);
        }
        15 => {
            assert!(wilds.active.is_none());
            assert_eq!(chunks.iter().count(), 0);
            assert_eq!(wilds.entries.len(), 1);
            if check.menu_shot {
                capture(&mut commands, "world-list-zh");
                check.menu_shot = false;
                check.since = now;
                return;
            }
            wilds.pending = Some(Action::Open(0));
        }
        16 => {
            let world = &wilds.active.as_ref().unwrap().saved.terrain;
            assert_eq!(world.block(1026, 160), Block::Air);
            assert_eq!(world.block(1027, 161), Block::Dirt);
            assert_eq!(world.inventory, [1, 0, 0]);
            assert!(Vec2::from_array(world.position).distance(check.start) < 1.0);
            assert!(chunks.iter().count() <= 12);
            let cave = (1100..1300)
                .flat_map(|x| (8..140).map(move |y| (x, y)))
                .find(|&(x, y)| {
                    world.block(x, y - 1).solid()
                        && world.clear_for_player([(x as f32 + 0.5) * TILE, y as f32 * TILE])
                })
                .expect("Seed 42 has a walkable cave");
            actors.single_mut().unwrap().teleport(Vec2::new(
                (cave.0 as f32 + 0.5) * TILE,
                cave.1 as f32 * TILE,
            ));
        }
        17 => {
            assert!(chunks.iter().count() <= 12);
            capture(&mut commands, "cave-zh");
        }
        18 => {
            wilds.pending = Some(Action::Recall);
        }
        19 => {
            wilds.pending = Some(Action::Leave);
        }
        20 => {
            assert!(wilds.active.is_none());
            assert_eq!(actors.iter().count(), 0);
            info!(
                "WILDS SMOKE PASS: create, dig/place with real pointer, material conservation, walk/jump, chunk unload/reload, scaling, modal isolation and save/resume"
            );
            exit.write(AppExit::Success);
        }
        30 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
    window.set_physical_cursor_position(check.cursor.map(|p| p.as_dvec2()));
    check.stage += 1;
    check.since = now;
}
