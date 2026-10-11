#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clubs;
#[cfg(debug_assertions)]
mod clubs_smoke;
mod clubs_ui;
mod coast;
mod editor;
#[cfg(debug_assertions)]
mod editor_smoke;
mod editor_ui;
mod fishing;
#[cfg(debug_assertions)]
mod fishing_smoke;
mod fishing_ui;
mod game;
mod i18n;
#[cfg(debug_assertions)]
mod i18n_smoke;
mod icons;
mod maps;
mod network;
mod paths;
mod settings;
#[cfg(debug_assertions)]
mod smoke;
mod ui;

use bevy::prelude::*;
use i18n::tr;
use network::{Event, ServerMessage};
use std::collections::VecDeque;

#[derive(Resource, Default)]
struct Session {
    link: Option<network::Link>,
    you: Option<u32>,
    label: String,
    invite: String,
    connected: bool,
    local_world: Option<yapshire_shared::World>,
    log: VecDeque<i18n::Message>,
}

impl Session {
    fn log(&mut self, line: i18n::Message) {
        self.log.push_back(line);
        while self.log.len() > 5 {
            self.log.pop_front();
        }
    }
}

fn main() {
    let adjacent_assets = std::env::current_exe()
        .unwrap_or_default()
        .with_file_name("assets");
    let asset_path = if adjacent_assets.is_dir() {
        adjacent_assets
    } else {
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
    };
    let mut maps = maps::Maps::load(&asset_path).expect("Unable to load Yapshire maps");
    let editor = editor::Editor::load(&mut maps);
    let (settings, i18n) = settings::Settings::load();
    let mut menu = ui::Menu::default();
    menu.server = settings.server().into();
    let clubs = clubs::Browser::new(settings.clubs(), settings.server());
    if let Some(club) = clubs.selected() {
        menu.server = club.saved.server.clone();
    }
    let mut app = App::new();
    app.insert_resource(maps)
        .insert_resource(editor)
        .insert_resource(settings)
        .insert_resource(clubs)
        .insert_resource(i18n)
        .insert_resource(ClearColor(Color::srgb_u8(22, 34, 38)))
        .init_resource::<Session>()
        .insert_resource(menu)
        .init_resource::<ui::Chat>()
        .init_resource::<fishing::Fishing>()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    file_path: asset_path.to_string_lossy().into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    close_when_requested: false,
                    primary_window: Some(Window {
                        title: std::env::var("YAPSHIRE_SMOKE")
                            .map(|mode| format!("YAPSHIRE - AUTOMATED TEST - {mode}"))
                            .unwrap_or_else(|_| "YAPSHIRE - A little place to be together".into()),
                        name: Some("yapshire".into()),
                        resolution: game::WINDOW_SIZE.into(),
                        resizable: false,
                        enabled_buttons: bevy::window::EnabledButtons {
                            maximize: false,
                            ..default()
                        },
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(bevy_ecs_tilemap::TilemapPlugin)
        .add_systems(
            Startup,
            (game::setup, coast::setup, settings::setup).chain(),
        )
        .add_systems(
            Update,
            (
                (settings::update, ui::buttons).chain(),
                ui::discover,
                ui::keyboard,
                clubs::update,
                clubs_ui::scroll,
                editor::update,
                connect_requests,
                network_events,
                fishing::update,
                game::walk,
                game::animate,
                game::follow_camera,
                coast::reload_maps,
                coast::animate,
                coast::animate_rig,
                game::bubbles,
                (
                    ui::render,
                    editor_ui::render,
                    fishing_ui::render,
                    ui::refresh,
                    editor_ui::refresh,
                    fishing_ui::refresh,
                    settings::render,
                    i18n::refresh,
                )
                    .chain(),
                game::fit_window,
                game::capture,
            )
                .chain(),
        );
    #[cfg(debug_assertions)]
    app.init_resource::<smoke::Smoke>()
        .init_resource::<editor_smoke::Check>()
        .init_resource::<i18n_smoke::Check>()
        .init_resource::<fishing_smoke::Check>()
        .init_resource::<clubs_smoke::Check>()
        .add_systems(Update, clubs_smoke::drive.before(ui::buttons))
        .add_systems(Update, fishing_smoke::drive.before(ui::buttons))
        .add_systems(Update, smoke::drive.before(ui::buttons))
        .add_systems(Update, editor_smoke::drive.before(ui::buttons))
        .add_systems(Update, i18n_smoke::drive.before(settings::update))
        .add_systems(Update, smoke::record.after(ui::render));
    #[cfg(debug_assertions)]
    if std::env::var_os("YAPSHIRE_SMOKE").is_some() {
        app.insert_resource(bevy::winit::WinitSettings::continuous());
    }
    app.run();
}

fn connect_requests(
    mut commands: Commands,
    mut menu: ResMut<ui::Menu>,
    mut session: ResMut<Session>,
    mut chat: ResMut<ui::Chat>,
    mut maps: ResMut<maps::Maps>,
    mut settings: ResMut<settings::Settings>,
    actors: Query<Entity, Or<(With<game::Actor>, With<game::Bubble>)>>,
) {
    if menu.leave {
        if let Some(world) = session.local_world.take() {
            maps.apply_world(&world);
        }
        *session = Session::default();
        *chat = ui::Chat::default();
        for entity in &actors {
            commands.entity(entity).despawn();
        }
        menu.leave = false;
    }
    if let Some(mode) = menu.request.take() {
        let world = match maps.world() {
            Ok(world) => world,
            Err(error) => {
                menu.status = tr("menu.connection_error").arg("error", error.to_string());
                menu.dirty = true;
                return;
            }
        };
        session.local_world = Some(world.clone());
        if matches!(
            mode,
            network::Mode::HostCloud { .. } | network::Mode::JoinCloud { .. }
        ) {
            settings.remember_server(&menu.server);
        }
        session.link = Some(network::start_with_options(
            mode,
            menu.name.clone(),
            world,
            menu.server_password.clone(),
        ));
        menu.status = tr("menu.connecting_status");
        menu.connecting = true;
        menu.dirty = true;
    }
}

fn network_events(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut menu: ResMut<ui::Menu>,
    art: Res<game::Art>,
    mut maps: ResMut<maps::Maps>,
    mut actors: Query<(Entity, &mut game::Actor)>,
    bubbles: Query<(Entity, &game::Bubble)>,
) {
    let events: Vec<_> = session
        .link
        .as_ref()
        .map(|link| {
            let receiver = link.events.lock().unwrap();
            let mut events = Vec::new();
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        if !events.iter().any(|event| matches!(event, Event::Error(_))) {
                            events.push(Event::Error(
                                "Connection ended. Return to the menu to rejoin.".into(),
                            ));
                        }
                        break;
                    }
                }
            }
            events
        })
        .unwrap_or_default();
    for event in events {
        match event {
            Event::Room { label, invite } => {
                session.label = label;
                session.invite = invite;
            }
            Event::Error(error) => {
                if let Some(world) = session.local_world.take() {
                    maps.apply_world(&world);
                }
                warn!("{error}");
                menu.status = tr("menu.connection_error").arg("error", error);
                session.log(menu.status.clone());
                menu.connecting = false;
                session.connected = false;
                session.link = None;
                menu.dirty = true;
            }
            Event::Message(message) => match message {
                ServerMessage::World { world } => {
                    maps.apply_world(&world);
                    menu.status = tr("menu.maps_ready");
                }
                ServerMessage::Welcome { you, players } => {
                    session.you = Some(you);
                    session.connected = true;
                    menu.page = ui::Page::Playing;
                    menu.connecting = false;
                    menu.status.clear();
                    menu.active = None;
                    menu.dirty = true;
                    session.log(tr("menu.welcome"));
                    info!(
                        "Connected: {} · {} players · you={you}",
                        session.label,
                        players.len()
                    );
                    for player in players.into_iter().take(16) {
                        game::spawn_actor(&mut commands, &art, player);
                    }
                }
                ServerMessage::Joined { player } => {
                    session.log(tr("menu.joined").arg("name", &player.name));
                    game::spawn_actor(&mut commands, &art, player);
                }
                ServerMessage::Moved { player } => {
                    if player.id == session.you.unwrap_or(0) {
                        continue;
                    }
                    if !player.x.is_finite()
                        || !player.y.is_finite()
                        || maps.by_id(&player.map).is_none()
                    {
                        continue;
                    }
                    for (_, mut actor) in &mut actors {
                        if actor.player.id == player.id {
                            if actor.player.map != player.map {
                                actor.teleport(Vec2::new(player.x, player.y));
                            }
                            actor.player = player.clone();
                        }
                    }
                }
                ServerMessage::Chat { id, text } => {
                    let text = network::clean(&text, 80);
                    if let Some((_, actor)) = actors.iter().find(|(_, actor)| actor.player.id == id)
                    {
                        session.log(format!("{}: {}", actor.player.name, text).into());
                        info!("Chat · {}: {text}", actor.player.name);
                        for (entity, bubble) in &bubbles {
                            if bubble.id == id {
                                commands.entity(entity).despawn();
                            }
                        }
                        game::spawn_bubble(&mut commands, &art, id, actor.position, &text);
                    }
                }
                ServerMessage::Left { id } => {
                    for (entity, actor) in &actors {
                        if actor.player.id == id {
                            session.log(tr("menu.left").arg("name", &actor.player.name));
                            commands.entity(entity).despawn();
                        }
                    }
                    for (entity, bubble) in &bubbles {
                        if bubble.id == id {
                            commands.entity(entity).despawn();
                        }
                    }
                }
            },
        }
    }
}
