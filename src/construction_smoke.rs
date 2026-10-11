//! Native tile construction acceptance: real UI touches, world pointer input and save restart.
use crate::{
    game::{self, Actor},
    i18n::{I18n, Language},
    settings::Settings,
    ui::{self, Menu, Page},
    wilds::{Action, Wilds},
};
use bevy::{
    ecs::system::SystemParam,
    input::touch::{TouchInput, TouchPhase},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use yapshire_shared::{
    construction::{Collision, Layer},
    terrain::TILE,
};

#[derive(Resource, Default)]
pub(crate) struct Check {
    stage: u32,
    since: f32,
    cursor: Option<Vec2>,
    forest_shot: bool,
}

fn enabled() -> bool {
    std::env::var("YAPSHIRE_SMOKE").is_ok_and(|s| s.starts_with("wilds-tiles"))
}

pub(crate) fn cursor(check: Res<Check>, mut window: Single<&mut Window>) {
    if enabled() {
        window.set_physical_cursor_position(check.cursor.map(|p| p.as_dvec2()));
    }
}

#[derive(SystemParam)]
pub(crate) struct Controls<'w, 's> {
    window: Single<'w, 's, (Entity, &'static mut Window)>,
    outer: Single<'w, 's, &'static Camera, With<game::OuterCamera>>,
    camera: Single<'w, 's, &'static Transform, With<game::WorldCamera>>,
    buttons: Query<
        'w,
        's,
        (
            &'static ui::Action,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    touch: MessageWriter<'w, TouchInput>,
    mouse: ResMut<'w, ButtonInput<MouseButton>>,
    keys: ResMut<'w, ButtonInput<KeyCode>>,
}
impl Controls<'_, '_> {
    fn tap(&mut self, target: Action) {
        let (_, node, transform) = self
            .buttons
            .iter()
            .find(|(a, _, _)| matches!(a, ui::Action::Wild(a) if *a == target))
            .expect("Missing construction button");
        let position = (transform.translation
            + self.outer.physical_viewport_rect().unwrap().min.as_vec2())
            / self.window.1.scale_factor();
        assert!(
            node.size().min_element() > 0.0
                && position.cmpge(Vec2::ZERO).all()
                && position.cmplt(self.window.1.size()).all(),
            "Tile button outside the viewport: {position}"
        );
        self.touch.write(TouchInput {
            phase: TouchPhase::Started,
            position,
            window: self.window.0,
            id: 8,
            force: None,
        });
    }
    fn point(&self, x: i32, y: i32, settings: &Settings) -> Vec2 {
        let viewport = self.outer.physical_viewport_rect().unwrap();
        let view = game::canvas_size(settings.pixel_scale()).as_vec2();
        let relative = ((Vec2::new(x as f32, y as f32) + 0.5) * TILE
            - self.camera.translation.truncate())
            / view;
        viewport.min.as_vec2()
            + Vec2::new(relative.x + 0.5, 0.5 - relative.y) * viewport.size().as_vec2()
    }
}
fn capture(commands: &mut Commands, tag: &str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(format!("artifacts/construction-{tag}.png")));
}
fn assert_saved(wilds: &Wilds) {
    let run = wilds.active.as_ref().unwrap();
    for (layer, id) in [
        (Layer::Wall, "yapshire:cedar"),
        (Layer::Solid, "yapshire:planks"),
        (Layer::Decor, "yapshire:flowers"),
    ] {
        assert_eq!(run.saved.terrain.cell(layer, 1026, 161, &run.catalog), id);
    }
    assert_eq!(
        run.saved.terrain.collision(1026, 161, &run.catalog),
        Collision::Solid
    );
}

pub(crate) fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut check: ResMut<Check>,
    mut controls: Controls,
    mut wilds: ResMut<Wilds>,
    mut menu: ResMut<Menu>,
    mut settings: ResMut<Settings>,
    mut i18n: ResMut<I18n>,
    mut actors: Query<&mut Actor>,
    art: Res<game::Art>,
    mut session: ResMut<crate::Session>,
    mut exit: MessageWriter<AppExit>,
) {
    if !enabled() {
        return;
    }
    for key in [
        "YAPSHIRE_WORLD_DIR",
        "YAPSHIRE_MAP_DIR",
        "YAPSHIRE_SETTINGS_DIR",
        "YAPSHIRE_SAVE_DIR",
    ] {
        assert!(
            std::env::var_os(key).is_some(),
            "Set {key} for isolated acceptance"
        );
    }
    let now = time.elapsed_secs();
    assert!(
        now < 120.0,
        "Construction smoke timed out at {}",
        check.stage
    );
    if now - check.since < if check.stage == 0 { 3.0 } else { 1.0 } {
        return;
    }
    controls.mouse.release_all();
    controls.keys.release_all();
    controls.touch.write(TouchInput {
        phase: TouchPhase::Ended,
        position: Vec2::ZERO,
        window: controls.window.0,
        id: 8,
        force: None,
    });
    let mode = std::env::var("YAPSHIRE_SMOKE").unwrap();
    if mode == "wilds-tiles-art" {
        match check.stage {
            0 => {
                settings.choose_scale(2);
                settings.choose_ui_scale(100);
                i18n.language = Language::Chinese;
                menu.go(Page::Playing);
                session.connected = true;
                session.you = Some(1);
                session.label = "Yapshire".into();
                game::spawn_actor(
                    &mut commands,
                    &art,
                    crate::network::Player {
                        id: 1,
                        name: "Wanderer".into(),
                        map: "yapshire:town".into(),
                        x: 340.0,
                        y: 0.0,
                        moving: false,
                        facing: false,
                        indoors: false,
                        fishing: false,
                    },
                );
            }
            1 => capture(&mut commands, "village-art"),
            2 => {
                let mut actor = actors.single_mut().unwrap();
                actor.position.x = 990.0;
                actor.player.x = 990.0;
            }
            3 => capture(&mut commands, "harbor-art"),
            4 => {
                info!("CONSTRUCTION ART REVIEW: native village and harbor captures complete");
                exit.write(AppExit::Success);
            }
            _ => {}
        }
        check.stage += 1;
        check.since = now;
        return;
    }
    let reload = mode.ends_with("reload");
    match check.stage {
        0 => {
            capture(&mut commands, "town-tiles");
        }
        1 => {
            menu.go(Page::Worlds);
            menu.world_name = "A builder's clearing".into();
            menu.world_seed = "42".into();
            settings.choose_scale(2);
            settings.choose_ui_scale(100);
            i18n.language = Language::English;
        }
        2 => {
            wilds.pending = Some(if reload {
                Action::Open(0)
            } else {
                Action::Create
            });
        }
        3 => {
            if reload {
                assert_saved(&wilds);
                wilds.pending = Some(Action::Leave);
                check.stage = 29;
                info!("CONSTRUCTION RELOAD PASS: three independent tile layers restored from disk");
            } else {
                if !check.forest_shot {
                    capture(&mut commands, "forest");
                    check.forest_shot = true;
                    check.since = now;
                    return;
                }
                controls.keys.press(KeyCode::KeyB);
            }
        }
        4 => {
            let run = wilds.active.as_ref().unwrap();
            assert!(run.palette_open);
            capture(&mut commands, "catalog-en");
            controls.tap(Action::Piece(run.catalog.index("yapshire:cedar").unwrap()));
        }
        5 => {
            let run = wilds.active.as_ref().unwrap();
            assert!(run.using_catalog);
            assert_eq!(run.layer, Layer::Wall);
            assert_eq!(run.catalog.pieces[run.selected_piece].id, "yapshire:cedar");
            controls.keys.press(KeyCode::KeyB);
        }
        6 => {
            check.cursor = Some(controls.point(1026, 161, &settings));
            controls.mouse.press(MouseButton::Right);
        }
        7 => {
            let run = wilds.active.as_ref().unwrap();
            assert_eq!(
                run.saved.terrain.cell(Layer::Wall, 1026, 161, &run.catalog),
                "yapshire:cedar"
            );
            assert_eq!(
                run.saved.terrain.collision(1026, 161, &run.catalog),
                Collision::None
            );
            check.cursor = None;
            controls.tap(Action::Layer(Layer::Solid));
        }
        8 => {
            let run = wilds.active.as_ref().unwrap();
            assert_eq!(run.layer, Layer::Solid);
            wilds.pending = Some(Action::Piece(run.catalog.index("yapshire:planks").unwrap()));
        }
        9 => {
            check.cursor = Some(controls.point(1026, 161, &settings));
            controls.mouse.press(MouseButton::Right);
        }
        10 => {
            let run = wilds.active.as_ref().unwrap();
            assert_eq!(
                run.saved
                    .terrain
                    .cell(Layer::Solid, 1026, 161, &run.catalog),
                "yapshire:planks"
            );
            wilds.pending = Some(Action::Piece(
                run.catalog.index("yapshire:flowers").unwrap(),
            ));
        }
        11 => {
            check.cursor = Some(controls.point(1026, 161, &settings));
            controls.mouse.press(MouseButton::Right);
        }
        12 => {
            assert_saved(&wilds);
            capture(&mut commands, "three-layers");
            controls.keys.press(KeyCode::KeyD);
        }
        13 => {
            assert!(
                actors.single().unwrap().position.x <= 1026.0 * TILE - 5.9,
                "Actor walked through constructed timber"
            );
            check.cursor = Some(controls.point(1026, 161, &settings));
            controls.mouse.press(MouseButton::Left);
        }
        14 => {
            let run = wilds.active.as_ref().unwrap();
            assert_eq!(
                run.saved
                    .terrain
                    .cell(Layer::Decor, 1026, 161, &run.catalog),
                ""
            );
            assert_eq!(
                run.saved
                    .terrain
                    .cell(Layer::Solid, 1026, 161, &run.catalog),
                "yapshire:planks"
            );
            assert_eq!(
                run.saved.terrain.cell(Layer::Wall, 1026, 161, &run.catalog),
                "yapshire:cedar"
            );
            controls.mouse.press(MouseButton::Right);
        }
        15 => {
            assert_saved(&wilds);
            check.cursor = None;
            wilds.pending = Some(Action::Group(1));
            controls.keys.press(KeyCode::KeyB);
        }
        16 => {
            assert!(wilds.active.as_ref().unwrap().palette_open);
            wilds.pending = Some(Action::Group(1));
            i18n.language = Language::Chinese;
            settings.choose_ui_scale(150);
        }
        17 => {
            capture(&mut commands, "catalog-zh-ui150");
            let run = wilds.active.as_ref().unwrap();
            assert_eq!(run.catalog.groups[run.palette_group], "nature/garden.tsj");
            controls.tap(Action::Piece(run.catalog.index("yapshire:trunk").unwrap()));
        }
        18 => {
            assert_eq!(wilds.active.as_ref().unwrap().layer, Layer::Wall);
            controls.window.1.resolution.set(844.0, 390.0);
        }
        19 => {
            capture(&mut commands, "catalog-compact");
            controls.tap(Action::Palette);
        }
        20 => {
            assert!(!wilds.active.as_ref().unwrap().palette_open);
            wilds.pending = Some(Action::Leave);
        }
        21 => {
            assert!(wilds.active.is_none());
            wilds.pending = Some(Action::Open(0));
        }
        22 => {
            assert_saved(&wilds);
            wilds.pending = Some(Action::Leave);
        }
        23 => {
            assert!(wilds.active.is_none());
            info!(
                "CONSTRUCTION SMOKE PASS: catalog touch input, per-layer pointer placement/erasing, collision, bilingual scaled/compact UI and save/resume"
            );
            exit.write(AppExit::Success);
        }
        30 => {
            assert!(wilds.active.is_none());
            exit.write(AppExit::Success);
        }
        _ => {}
    }
    controls
        .window
        .1
        .set_physical_cursor_position(check.cursor.map(|p| p.as_dvec2()));
    info!("CONSTRUCTION stage {}", check.stage);
    check.stage += 1;
    check.since = now;
}
