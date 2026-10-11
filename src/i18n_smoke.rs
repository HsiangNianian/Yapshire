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
    button_height: f32,
    text_height: f32,
    cursor: Option<Vec2>,
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
            &'static UiGlobalTransform,
        ),
        Without<editor::Action>,
    >,
    editor: Query<
        'w,
        's,
        (&'static editor::Action, &'static mut Interaction),
        Without<settings::Action>,
    >,
    areas: Query<
        'w,
        's,
        (
            &'static crate::ui::ScrollArea,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static ScrollPosition,
        ),
    >,
    wheel: MessageWriter<'w, bevy::input::mouse::MouseWheel>,
    mouse: ResMut<'w, ButtonInput<MouseButton>>,
    keys: ResMut<'w, ButtonInput<KeyCode>>,
    close: MessageWriter<'w, WindowCloseRequested>,
    touch: MessageWriter<'w, TouchInput>,
}

impl Controls<'_, '_> {
    fn settings(&mut self, action: settings::Action) {
        let (_, mut interaction, _, _) = self
            .settings
            .iter_mut()
            .find(|(a, _, _, _)| **a == action)
            .expect("Missing settings button");
        *interaction = Interaction::Pressed;
        self.mouse.press(MouseButton::Left);
    }
    fn tap_settings(&mut self, action: settings::Action) {
        let (_, _, node, transform) = self
            .settings
            .iter()
            .find(|(a, _, _, _)| **a == action)
            .unwrap();
        let camera_origin = if action == settings::Action::Open {
            Vec2::ZERO
        } else {
            self.camera.physical_viewport_rect().unwrap().min.as_vec2()
        };
        let position = (transform.translation + camera_origin) / self.window.1.scale_factor();
        assert!(position.cmpge(Vec2::ZERO).all() && position.cmplt(self.window.1.size()).all());
        assert!(node.size().min_element() > 0.0);
        self.touch.write(TouchInput {
            phase: TouchPhase::Started,
            position,
            window: self.window.0,
            id: 2,
            force: None,
        });
    }
    fn end_touch(&mut self) {
        self.touch.write(TouchInput {
            phase: TouchPhase::Ended,
            position: Vec2::ZERO,
            window: self.window.0,
            id: 2,
            force: None,
        });
    }
    fn scroll_target(&self) -> Vec2 {
        let (_, node, transform, _) = self
            .areas
            .iter()
            .find(|(area, _, _, _)| area.settings)
            .unwrap();
        assert!(node.content_size().y > node.size().y);
        transform.translation + self.camera.physical_viewport_rect().unwrap().min.as_vec2()
    }
    fn scroll_wheel(&mut self) {
        self.wheel.write(bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y: -20.0,
            window: self.window.0,
        });
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

pub(crate) fn position_cursor(check: Res<Check>, mut window: Single<&mut Window>) {
    if std::env::var("YAPSHIRE_SMOKE")
        .unwrap_or_default()
        .starts_with("i18n")
    {
        window.set_physical_cursor_position(check.cursor.map(|p| p.as_dvec2()));
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
    texts: Query<(&Text, &ComputedNode)>,
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
    assert!(now < 120.0, "I18n smoke timed out at stage {}", check.stage);
    controls.keys.release_all();
    controls.mouse.release(MouseButton::Left);
    controls
        .window
        .1
        .set_physical_cursor_position(check.cursor.map(|p| p.as_dvec2()));
    if now - check.since < if check.stage == 0 { 3.0 } else { 0.9 } {
        return;
    }
    let has = |part: &str| texts.iter().any(|(text, _)| text.contains(part));
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
    let check_ui = |expected: u32, baseline: &Check| {
        assert_eq!(settings.ui_scale(), expected);
        check_scale(4);
        let viewport = controls.camera.physical_viewport_rect().unwrap();
        let unit = viewport.width() as f32 / game::WINDOW_SIZE.x as f32;
        let (_, _, button, _) = controls
            .settings
            .iter()
            .find(|(a, _, _, _)| **a == settings::Action::UiScale(100))
            .unwrap();
        let (_, glyphs) = texts
            .iter()
            .find(|(text, _)| text.as_str() == "100%")
            .unwrap();
        let ratio = expected as f32 / 100.0;
        assert!(
            (button.size().y / unit - baseline.button_height * ratio).abs() < 1.1,
            "Button did not scale"
        );
        assert!(
            (glyphs.size().y / unit - baseline.text_height * ratio).abs() < 1.1,
            "Text did not scale"
        );
        let (_, _, close, transform) = controls
            .settings
            .iter()
            .find(|(a, _, _, _)| **a == settings::Action::Close)
            .unwrap();
        let bounds = Rect::from_center_size(transform.translation, close.size());
        assert!(
            bounds.min.cmpge(Vec2::ZERO).all() && bounds.max.cmple(viewport.size().as_vec2()).all(),
            "Done must stay in the viewport: {bounds:?}"
        );
        info!(
            "UI SCALE VERIFIED: {expected}%, button={} px, text={} px",
            button.size().y,
            glyphs.size().y
        );
    };
    if mode == "i18n-reload" {
        assert_eq!(i18n.language, Language::Chinese);
        check_scale(4);
        let persisted: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                std::path::PathBuf::from(std::env::var_os("YAPSHIRE_SETTINGS_DIR").unwrap())
                    .join("settings.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            settings.ui_scale() as u64,
            persisted["ui_scale"].as_u64().unwrap_or(100)
        );
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
            let (_, _, gear, _) = controls
                .settings
                .iter()
                .find(|(a, _, _, _)| **a == settings::Action::Open)
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
            assert!(has("设置语言、界面大小") && has("创建房间"));
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
            let ui = editor.canvas.min
                + ((UVec2::new(50, 5) - editor.offset).as_vec2() + Vec2::splat(0.5))
                    * editor.cell_size();
            let viewport = controls.camera.physical_viewport_rect().unwrap();
            let physical =
                viewport.min.as_vec2() + ui * (viewport.width() as f32 / editor.ui_size.x);
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
        44 if mode == "i18n-ui" => controls.settings(settings::Action::Open),
        45 => {
            let unit = controls.camera.physical_viewport_rect().unwrap().width() as f32
                / game::WINDOW_SIZE.x as f32;
            check.button_height = controls
                .settings
                .iter()
                .find(|(a, _, _, _)| **a == settings::Action::UiScale(100))
                .unwrap()
                .2
                .size()
                .y
                / unit;
            check.text_height = texts
                .iter()
                .find(|(text, _)| text.as_str() == "100%")
                .unwrap()
                .1
                .size()
                .y
                / unit;
            check_ui(100, &check);
            capture(&mut commands, "ui-100-zh");
        }
        46 => controls.tap_settings(settings::Action::UiScale(125)),
        47 => {
            check_ui(125, &check);
            controls.end_touch();
            capture(&mut commands, "ui-125-zh");
        }
        48 => controls.tap_settings(settings::Action::UiScale(150)),
        49 => {
            check_ui(150, &check);
            controls.end_touch();
            capture(&mut commands, "ui-150-zh");
        }
        50 => controls.tap_settings(settings::Action::UiScale(75)),
        51 => {
            check_ui(75, &check);
            controls.end_touch();
            capture(&mut commands, "ui-75-zh");
        }
        52 => controls.tap_settings(settings::Action::UiScale(150)),
        53 => {
            check_ui(150, &check);
            controls.end_touch();
            controls.settings(settings::Action::Language(Language::English));
        }
        54 => capture(&mut commands, "ui-150-en"),
        55 => controls.tap_settings(settings::Action::Close),
        56 => {
            controls.end_touch();
            assert!(!settings.open);
            capture(&mut commands, "home-150-en");
        }
        57 => menu.go(Page::Editor),
        58 => {
            assert!(
                editor.canvas.max.x <= editor.ui_size.x && editor.canvas.max.y <= editor.ui_size.y
            );
            capture(&mut commands, "editor-150-en");
        }
        59 => {
            let point = editor.canvas.min
                + ((UVec2::new(50, 5) - editor.offset).as_vec2() + Vec2::splat(0.5))
                    * editor.cell_size();
            let viewport = controls.camera.physical_viewport_rect().unwrap();
            let physical =
                viewport.min.as_vec2() + point * (viewport.width() as f32 / editor.ui_size.x);
            controls
                .window
                .1
                .set_physical_cursor_position(Some(physical.as_dvec2()));
            controls.mouse.press(MouseButton::Left);
        }
        60 => {
            assert!(editor.dirty());
            assert_eq!(
                editor.doc().map.layers[4].data[5 * 90 + 50],
                check.painted,
                "Scaled editor painted the wrong cell"
            );
            controls.editor(editor::Action::Done);
        }
        61 => {
            assert!(editor.dialog.is_some());
            controls.editor(editor::Action::Discard);
        }
        62 => {
            assert!(!editor.dirty());
            session.connected = true;
            session.you = Some(1);
            menu.go(Page::Playing);
        }
        63 => {
            fishing.panel = Panel::Bag;
            fishing.dirty = true;
        }
        64 => capture(&mut commands, "satchel-150-en"),
        65 => {
            fishing.panel = Panel::Shop;
            fishing.dirty = true;
        }
        66 => capture(&mut commands, "shop-150-en"),
        67 => {
            fishing.panel = Panel::None;
            fishing.dirty = true;
            menu.leave = true;
            menu.go(Page::Cloud);
        }
        68 => capture(&mut commands, "clubs-150-en"),
        69 => {
            menu.club_editor = Some(None);
            menu.dirty = true;
        }
        70 => capture(&mut commands, "club-form-150-en"),
        71 => {
            menu.go(Page::Home);
            controls.window.1.resolution.set(844.0, 390.0);
        }
        72 => capture(&mut commands, "compact-home-150-en"),
        73 => controls.tap_settings(settings::Action::Open),
        74 => {
            controls.end_touch();
            assert!(
                settings.open,
                "Gear failed at {:?}",
                controls
                    .settings
                    .iter()
                    .map(|(_, interaction, node, transform)| (
                        interaction,
                        node.size(),
                        transform.translation
                    ))
                    .collect::<Vec<_>>()
            );
            controls.settings(settings::Action::Language(Language::Chinese));
        }
        75 => capture(&mut commands, "compact-ui-150-zh"),
        76 => controls.tap_settings(settings::Action::Close),
        77 => {
            controls.end_touch();
            assert!(!settings.open);
            controls.window.1.resolution.set(1440.0, 810.0);
        }
        78 if mode == "i18n-ui" => controls.tap_settings(settings::Action::Open),
        79 => {
            controls.end_touch();
            assert!(settings.open);
            check.cursor = Some(controls.scroll_target());
        }
        80 => controls.scroll_wheel(),
        81 => {
            assert!(
                controls
                    .areas
                    .iter()
                    .any(|(area, _, _, position)| area.settings && position.0.y > 0.0),
                "Settings must scroll at 150%"
            );
            capture(&mut commands, "ui-150-scrolled-zh");
            check.cursor = None;
        }
        82 => controls.tap_settings(settings::Action::Scale(3)),
        83 => {
            check_scale(3);
            assert_eq!(settings.ui_scale(), 150);
            controls.end_touch();
        }
        84 => controls.tap_settings(settings::Action::Scale(4)),
        85 => {
            check_scale(4);
            assert_eq!(settings.ui_scale(), 150);
            controls.end_touch();
        }
        86 => controls.tap_settings(settings::Action::Close),
        87 => {
            controls.end_touch();
            assert!(!settings.open);
            controls.keys.press(KeyCode::F11);
        }
        88 => {
            assert_ne!(controls.window.1.mode, bevy::window::WindowMode::Windowed);
            capture(&mut commands, "fullscreen-ui-150-zh");
        }
        89 => controls.keys.press(KeyCode::F11),
        90 => menu.go(Page::Cloud),
        91 => {
            menu.club_editor = Some(None);
            menu.dirty = true;
        }
        92 => {
            let (_, _, transform, _) = controls
                .areas
                .iter()
                .find(|(area, _, _, _)| !area.settings)
                .unwrap();
            check.cursor = Some(
                transform.translation
                    + controls
                        .camera
                        .physical_viewport_rect()
                        .unwrap()
                        .min
                        .as_vec2(),
            );
        }
        93 => controls.scroll_wheel(),
        94 => {
            assert!(
                controls
                    .areas
                    .iter()
                    .any(|(area, _, _, position)| !area.settings && position.0.y > 0.0),
                "Club form must scroll at 150%"
            );
            capture(&mut commands, "club-form-150-scrolled-zh");
            check.cursor = None;
        }
        95 => menu.dirty = true,
        96 => {
            assert!(
                controls
                    .areas
                    .iter()
                    .any(|(area, _, _, position)| !area.settings && position.0.y > 0.0),
                "Refreshing the Club directory must preserve the form's scroll position"
            );
            menu.go(Page::Home);
        }
        _ => {
            let path = std::path::PathBuf::from(std::env::var_os("YAPSHIRE_SETTINGS_DIR").unwrap())
                .join("settings.json");
            let saved: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            assert_eq!(saved["language"], "zh-CN");
            assert_eq!(saved["pixel_scale"], 4);
            if mode == "i18n-ui" {
                assert_eq!(saved["ui_scale"], 150);
                info!(
                    "UI SCALE SMOKE PASS: physical text/button sizes, real touch targets, editor painting, compact window and saved independent preferences"
                );
            }
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
