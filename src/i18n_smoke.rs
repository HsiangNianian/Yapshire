//! Native language/settings acceptance. Uses isolated saves and an offline UI
//! session fixture; this does not claim to test multiplayer connectivity.
use crate::{
    Session,
    editor::{self, Editor, Tool},
    fishing::{Fishing, Panel},
    game::{self, Art},
    i18n::{I18n, Language},
    settings::{self, Settings},
    ui::{Menu, Page},
};
use bevy::{
    ecs::system::SystemParam,
    input::touch::{TouchInput, TouchPhase},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::WindowCloseRequested,
};

#[derive(Resource, Default)]
pub(crate) struct Check {
    stage: u8,
    since: f32,
    painted: u32,
}

#[derive(SystemParam)]
pub(crate) struct Controls<'w, 's> {
    window: Single<'w, 's, (Entity, &'static mut Window)>,
    camera: Single<'w, 's, &'static Camera, With<game::OuterCamera>>,
    target: Single<'w, 's, &'static bevy::camera::RenderTarget, With<game::WorldCamera>>,
    settings: Query<
        'w,
        's,
        (
            &'static settings::Action,
            &'static mut Interaction,
            &'static ComputedNode,
        ),
        Without<editor::Action>,
    >,
    editor: Query<
        'w,
        's,
        (&'static editor::Action, &'static mut Interaction),
        Without<settings::Action>,
    >,
    mouse: ResMut<'w, ButtonInput<MouseButton>>,
    keys: ResMut<'w, ButtonInput<KeyCode>>,
    close: MessageWriter<'w, WindowCloseRequested>,
    touch: MessageWriter<'w, TouchInput>,
}

impl Controls<'_, '_> {
    fn settings(&mut self, action: settings::Action) {
        let (_, mut interaction, _) = self
            .settings
            .iter_mut()
            .find(|(a, _, _)| **a == action)
            .expect("Missing settings button");
        *interaction = Interaction::Pressed;
        self.mouse.press(MouseButton::Left);
    }
    fn editor(&mut self, action: editor::Action) {
        let (_, mut interaction) = self
            .editor
            .iter_mut()
            .find(|(a, _)| **a == action)
            .expect("Missing editor button");
        *interaction = Interaction::Pressed;
        self.mouse.press(MouseButton::Left);
    }
}

pub(crate) fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut check: ResMut<Check>,
    mut menu: ResMut<Menu>,
    i18n: Res<I18n>,
    settings: Res<Settings>,
    editor: Res<Editor>,
    mut fishing: ResMut<Fishing>,
    mut session: ResMut<Session>,
    art: Res<Art>,
    mut controls: Controls,
    texts: Query<&Text>,
    images: Res<Assets<Image>>,
) {
    let mode = std::env::var("YAPSHIRE_SMOKE").unwrap_or_default();
    if !mode.starts_with("i18n") {
        return;
    }
    if mode == "i18n-compact" && check.stage == 0 && controls.window.1.width() != 844.0 {
        controls.window.1.resolution.set(844.0, 390.0);
    }
    for variable in [
        "YAPSHIRE_SETTINGS_DIR",
        "YAPSHIRE_MAP_DIR",
        "YAPSHIRE_SAVE_DIR",
    ] {
        assert!(
            std::env::var_os(variable).is_some(),
            "Set {variable} for isolated native acceptance"
        );
    }
    let now = time.elapsed_secs();
    assert!(now < 90.0, "I18n smoke timed out at stage {}", check.stage);
    controls.keys.release_all();
    controls.mouse.release(MouseButton::Left);
    controls.window.1.set_physical_cursor_position(None);
    if now - check.since < if check.stage == 0 { 3.0 } else { 0.9 } {
        return;
    }
    let has = |part: &str| texts.iter().any(|text| text.contains(part));
    let capture = |commands: &mut Commands, tag: &str| {
        std::fs::create_dir_all("artifacts").unwrap();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("artifacts/{mode}-{tag}.png")));
    };
    let check_scale = |scale| {
        assert_eq!(settings.pixel_scale(), scale);
        let bevy::camera::RenderTarget::Image(target) = &*controls.target else {
            panic!("Expected pixel render target");
        };
        assert_eq!(
            images.get(&target.handle).unwrap().size(),
            game::canvas_size(scale)
        );
    };
    if mode == "i18n-reload" {
        assert_eq!(i18n.language, Language::Chinese);
        check_scale(4);
        assert!(has("创建房间"));
        if check.stage == 0 {
            capture(&mut commands, "home-zh");
        } else {
            info!("I18N RELOAD PASS: Chinese language and 4x pixel scale restored at startup");
            let window = controls.window.0;
            controls.close.write(WindowCloseRequested { window });
        }
        check.stage += 1;
        check.since = now;
        return;
    }
    match check.stage {
        0 => {
            assert_eq!(i18n.language, Language::English);
            let (_, _, gear) = controls
                .settings
                .iter()
                .find(|(a, _, _)| **a == settings::Action::Open)
                .unwrap();
            assert!(gear.size().min_element() / controls.window.1.scale_factor() >= 47.9);
            capture(&mut commands, "home-en");
            menu.name = "小风{name}".into();
        }
        1 if mode == "i18n-compact" => {
            let position = Vec2::new(controls.window.1.width() - 36.0, 36.0);
            let window = controls.window.0;
            controls.touch.write(TouchInput {
                phase: TouchPhase::Started,
                position,
                window,
                id: 1,
                force: None,
            });
        }
        1 => controls.settings(settings::Action::Open),
        2 => {
            assert!(settings.open && has("Choose your language"));
            let window = controls.window.0;
            controls.touch.write(TouchInput {
                phase: TouchPhase::Ended,
                position: Vec2::ZERO,
                window,
                id: 1,
                force: None,
            });
            capture(&mut commands, "settings-en");
        }
        3 => controls.settings(settings::Action::Language(Language::Chinese)),
        4 => {
            assert_eq!(i18n.language, Language::Chinese);
            assert!(has("选择语言和像素缩放") && has("创建房间"));
            capture(&mut commands, "settings-zh");
        }
        5 => controls.settings(settings::Action::Close),
        6 => {
            assert!(!settings.open && menu.page == Page::Home);
            assert_eq!(menu.name, "小风{name}");
            capture(&mut commands, "home-zh");
        }
        7 => menu.go(Page::Editor),
        8 => {
            assert!(has("地图工坊") && has("物件"));
            capture(&mut commands, "editor-zh");
        }
        9 => {
            let ui = editor::CANVAS
                + ((UVec2::new(50, 5) - editor.offset).as_vec2() + Vec2::splat(0.5))
                    * editor.cell_size();
            let viewport = controls.camera.physical_viewport_rect().unwrap();
            let physical = viewport.min.as_vec2()
                + ui * (viewport.width() as f32 / game::WINDOW_SIZE.x as f32);
            controls
                .window
                .1
                .set_physical_cursor_position(Some(physical.as_dvec2()));
            controls.mouse.press(MouseButton::Left);
        }
        10 => {
            assert!(editor.dirty());
            check.painted = editor.doc().map.layers[4].data[5 * 90 + 50];
            controls.keys.press(KeyCode::KeyE);
            controls.settings(settings::Action::Open);
        }
        11 => {
            // Opening settings must pause editor shortcuts, including saving.
            controls.keys.press(KeyCode::KeyE);
            controls.keys.press(KeyCode::ControlLeft);
            controls.keys.press(KeyCode::KeyS);
            controls.settings(settings::Action::Language(Language::English));
        }
        12 => {
            assert_eq!(i18n.language, Language::English);
            assert!(editor.dirty() && editor.tool == Tool::Brush && has("Map workshop"));
            assert_eq!(editor.doc().map.layers[4].data[5 * 90 + 50], check.painted);
            controls.keys.press(KeyCode::Escape);
        }
        13 => {
            assert!(!settings.open && menu.page == Page::Editor && editor.dialog.is_none());
            capture(&mut commands, "editor-en");
        }
        14 => controls.editor(editor::Action::Done),
        15 => {
            assert!(
                editor.dialog.is_some(),
                "Done must open the unsaved dialog; dirty={}, blocked={}",
                editor.dirty(),
                settings.blocks_input()
            );
            controls.editor(editor::Action::Discard);
        }
        16 => {
            assert!(menu.page == Page::Home && !editor.dirty());
            session.connected = true;
            session.you = Some(1);
            session.label = "Test town / 测试小镇".into();
            menu.go(Page::Playing);
            game::spawn_actor(
                &mut commands,
                &art,
                crate::network::Player {
                    map: "yapshire:town".into(),
                    id: 1,
                    name: menu.name.clone(),
                    x: 260.0,
                    y: 0.0,
                    moving: false,
                    facing: false,
                    indoors: false,
                    fishing: false,
                },
            );
        }
        17 => controls.settings(settings::Action::Open),
        18 => controls.settings(settings::Action::Language(Language::Chinese)),
        19 => {
            assert!(session.connected && settings.open);
            controls.settings(settings::Action::Close);
        }
        20 => {
            fishing.panel = Panel::Bag;
            fishing.dirty = true;
        }
        21 => {
            assert!(has("你的背包") && has("竹钓竿") && has("沙丁鱼"));
            capture(&mut commands, "satchel-zh");
        }
        22 => {
            fishing.panel = Panel::Shop;
            fishing.dirty = true;
        }
        23 => {
            assert!(has("玛拉的渔具柜台") && has("出售渔获"));
            capture(&mut commands, "shop-zh");
        }
        24 => {
            fishing.panel = Panel::None;
            fishing.dirty = true;
        }
        25 => controls.settings(settings::Action::Open),
        26 => controls.settings(settings::Action::Language(Language::English)),
        27 => controls.settings(settings::Action::Scale(3)),
        28 => {
            check_scale(3);
            assert!(session.connected);
            capture(&mut commands, "scale-3-en");
        }
        29 => controls.settings(settings::Action::Scale(4)),
        30 => {
            check_scale(4);
            capture(&mut commands, "scale-4-en");
        }
        31 => controls.settings(settings::Action::Close),
        32 => capture(&mut commands, "world-4"),
        33 => controls.keys.press(KeyCode::F11),
        34 => {
            check_scale(4);
            assert_ne!(controls.window.1.mode, bevy::window::WindowMode::Windowed);
            capture(&mut commands, "fullscreen-4");
        }
        35 => controls.keys.press(KeyCode::F11),
        36 => controls.settings(settings::Action::Open),
        37 => controls.settings(settings::Action::Language(Language::Chinese)),
        38 => {
            assert_eq!(controls.window.1.size(), game::WINDOW_SIZE.as_vec2());
            check_scale(4);
            capture(&mut commands, "scale-4-zh");
        }
        39 => controls.settings(settings::Action::Scale(2)),
        40 => {
            check_scale(2);
            capture(&mut commands, "scale-2-zh");
        }
        41 => controls.settings(settings::Action::Scale(4)),
        42 => controls.settings(settings::Action::Close),
        43 => {
            menu.leave = true;
            menu.go(Page::Home);
        }
        _ => {
            let path = std::path::PathBuf::from(std::env::var_os("YAPSHIRE_SETTINGS_DIR").unwrap())
                .join("settings.json");
            let saved: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            assert_eq!(saved["language"], "zh-CN");
            assert_eq!(saved["pixel_scale"], 4);
            info!(
                "I18N SMOKE PASS: settings, language, editor draft, modal input isolation, satchel/shop, 2x/3x/4x pixels, fullscreen and saved preferences"
            );
            let window = controls.window.0;
            controls.close.write(WindowCloseRequested { window });
        }
    }
    check.stage += 1;
    check.since = now;
}
