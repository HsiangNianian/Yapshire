use crate::{
    Session,
    game::{Actor, Art, font},
    i18n::{I18n, Localized, Message, tr},
    network::{self, ClientMessage, Mode},
};
use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
};
use std::sync::{Mutex, mpsc};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Host,
    Lan,
    Cloud,
    Playing,
    Editor,
}
#[derive(Clone, Copy, PartialEq, Eq, Component)]
pub enum Field {
    Name,
    Address,
    Server,
    ServerPassword,
    Room,
    RoomName,
    Port,
    ClubAlias,
    ClubServer,
    ClubPassword,
}

#[derive(Resource)]
pub struct Menu {
    pub page: Page,
    pub active: Option<Field>,
    pub name: String,
    address: String,
    pub(crate) server: String,
    pub(crate) server_password: String,
    room: String,
    pub(crate) room_name: String,
    port: String,
    cloud_host: bool,
    preedit: String,
    selected: bool,
    pub status: Message,
    pub connecting: bool,
    pub request: Option<Mode>,
    pub leave: bool,
    pub dirty: bool,
    pub(crate) rooms: Vec<network::RoomEntry>,
    scan: Option<Mutex<mpsc::Receiver<Result<Vec<network::RoomEntry>, Message>>>>,
    lobby_error: Message,
    room_page: usize,
    pub(crate) club_editor: Option<Option<u64>>,
    pub(crate) club_alias: String,
    pub(crate) club_server: String,
    pub(crate) club_password: String,
    pub(crate) club_submit: bool,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            page: Page::Home,
            active: None,
            name: "Wanderer".into(),
            address: "127.0.0.1:4761".into(),
            server: network::DEFAULT_SERVER.trim().into(),
            server_password: String::new(),
            room: String::new(),
            room_name: String::new(),
            port: network::PORT.to_string(),
            cloud_host: false,
            preedit: String::new(),
            selected: false,
            status: Message::default(),
            connecting: false,
            request: None,
            leave: false,
            dirty: true,
            rooms: Vec::new(),
            scan: None,
            lobby_error: Message::default(),
            room_page: 0,
            club_editor: None,
            club_alias: String::new(),
            club_server: String::new(),
            club_password: String::new(),
            club_submit: false,
        }
    }
}

impl Menu {
    fn field(&self, field: Field) -> &str {
        match field {
            Field::Name => &self.name,
            Field::Address => &self.address,
            Field::Server => &self.server,
            Field::ServerPassword => &self.server_password,
            Field::Room => &self.room,
            Field::RoomName => &self.room_name,
            Field::Port => &self.port,
            Field::ClubAlias => &self.club_alias,
            Field::ClubServer => &self.club_server,
            Field::ClubPassword => &self.club_password,
        }
    }
    fn field_mut(&mut self, field: Field) -> &mut String {
        match field {
            Field::Name => &mut self.name,
            Field::Address => &mut self.address,
            Field::Server => {
                self.server_password.clear();
                self.rooms.clear();
                self.scan = None;
                &mut self.server
            }
            Field::ServerPassword => {
                self.rooms.clear();
                self.scan = None;
                &mut self.server_password
            }
            Field::Room => &mut self.room,
            Field::RoomName => &mut self.room_name,
            Field::Port => &mut self.port,
            Field::ClubAlias => &mut self.club_alias,
            Field::ClubServer => {
                self.club_password.clear();
                &mut self.club_server
            }
            Field::ClubPassword => &mut self.club_password,
        }
    }
    fn fields(&self) -> Vec<Field> {
        match self.page {
            Page::Home => vec![Field::Name],
            Page::Host if self.cloud_host => {
                vec![Field::Server, Field::ServerPassword, Field::RoomName]
            }
            Page::Host => vec![Field::Port],
            Page::Lan => vec![Field::Address],
            Page::Cloud if self.club_editor.is_some() => {
                vec![Field::ClubAlias, Field::ClubServer, Field::ClubPassword]
            }
            Page::Cloud => vec![Field::ServerPassword, Field::Room],
            Page::Playing | Page::Editor => vec![],
        }
    }
    pub(crate) fn go(&mut self, page: Page) {
        self.page = page;
        self.active = None;
        self.preedit.clear();
        self.status.clear();
        self.dirty = true;
        self.rooms.clear();
        self.scan = None;
        self.room_page = 0;
        self.club_editor = None;
        self.club_password.clear();
        if page == Page::Lan {
            self.refresh_rooms();
        }
    }
    fn refresh_rooms(&mut self) {
        if self.scan.is_some() {
            return;
        }
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let result = network::discover_lan();
            let _ = send.send(result.map_err(Message::from));
        });
        self.scan = Some(Mutex::new(receive));
        self.lobby_error.clear();
        self.dirty = true;
    }
    fn connect(&mut self) {
        if self.connecting {
            return;
        }
        if self.page == Page::Cloud {
            if let Err(error) = self.resolve_invite() {
                self.status = tr("menu.invite_invalid").arg("error", error);
                return;
            }
        }
        if self.page == Page::Cloud || (self.page == Page::Host && self.cloud_host) {
            match network::normalize_server(&self.server) {
                Ok(address) => self.server = address,
                Err(error) => {
                    self.status = tr("menu.server_invalid").arg("error", error);
                    return;
                }
            }
        }
        self.name = network::clean(&self.name, 12);
        if self.name.is_empty() {
            self.status = tr("menu.name_required");
            return;
        }
        self.request = match self.page {
            Page::Host if self.cloud_host => {
                self.room_name = network::clean(&self.room_name, 24);
                if self.room_name.is_empty() {
                    self.status = tr("menu.room_required");
                    None
                } else {
                    Some(Mode::HostCloud {
                        server: self.server.clone(),
                        room_name: self.room_name.clone(),
                    })
                }
            }
            Page::Host => match self.port.parse::<u16>() {
                Ok(port) if port > 0 => Some(Mode::HostLan(port)),
                _ => {
                    self.status = tr("menu.port_invalid");
                    None
                }
            },
            Page::Lan => Some(Mode::JoinLan(self.address.clone())),
            Page::Cloud => Some(Mode::JoinCloud {
                server: self.server.clone(),
                room: self.room.clone(),
            }),
            _ => None,
        };
        self.active = None;
    }

    pub(crate) fn select_server(&mut self, server: &str, password: &str) {
        if self.server != server {
            self.room.clear();
        }
        self.server = server.into();
        self.server_password = password.into();
        self.rooms.clear();
        self.scan = None;
        self.room_page = 0;
        self.active = None;
        self.status.clear();
        self.dirty = true;
    }

    pub(crate) fn join_room_code(&mut self, code: &str) {
        self.room = code.into();
        self.connect();
    }

    pub(crate) fn host_selected_club(&mut self) {
        self.cloud_host = true;
        self.go(Page::Host);
    }

    fn resolve_invite(&mut self) -> Result<(), String> {
        let (server, room) = network::room_target(&self.server, &self.room)?;
        if server != self.server {
            *self.field_mut(Field::Server) = server;
            self.lobby_error.clear();
            self.room_page = 0;
            self.dirty = true;
        }
        self.room = room;
        Ok(())
    }

    fn preview_invite(&mut self) {
        if self.active == Some(Field::Room) && self.room.contains("://") {
            // Show the destination in the server field immediately after paste,
            // before joining; never send the previous server's password there.
            let _ = self.resolve_invite();
        }
    }
}

#[derive(Resource, Default)]
pub struct Chat {
    pub open: bool,
    value: String,
    preedit: String,
    selected: bool,
}

#[derive(Component, Clone, Copy)]
pub enum Action {
    Go(Page),
    Focus(Field),
    Hosting(bool),
    Connect,
    Back,
    Copy,
    Refresh,
    OfficialServer,
    JoinRoom(usize),
    NextRooms,
    Fishing(crate::fishing::Action),
    Club(crate::clubs::Action),
}
#[derive(Component)]
pub struct Root;
#[derive(Component)]
pub struct FieldValue(Field);
#[derive(Component)]
pub struct Status;
#[derive(Component)]
pub struct ChatValue;
#[derive(Component)]
pub struct ChatLog;
#[derive(Component)]
pub(crate) struct PlayOverlay;
#[derive(Component)]
pub struct RoomStatus;
#[derive(Component)]
pub struct BaseColor(pub(crate) Color);

pub(crate) const INK: Color = Color::srgb_u8(228, 225, 207);
pub(crate) const GREEN: Color = Color::srgb_u8(65, 90, 78);
pub(crate) const CREAM: Color = Color::srgb_u8(241, 227, 197);
pub(crate) const MUTED: Color = Color::srgb_u8(169, 183, 166);
pub(crate) const PANEL: Color = Color::srgba_u8(27, 39, 38, 244);
pub(crate) const SURFACE: Color = Color::srgb_u8(45, 61, 55);
pub(crate) const EDGE: Color = Color::srgb_u8(97, 112, 94);
pub(crate) const GOLD: Color = Color::srgb_u8(213, 177, 112);

pub fn buttons(
    mut settings: ResMut<crate::settings::Settings>,
    mut clubs: ResMut<crate::clubs::Browser>,
    mut interactions: Query<
        (&Interaction, &Action, &mut BackgroundColor, &BaseColor),
        Changed<Interaction>,
    >,
    mut menu: ResMut<Menu>,
    session: Res<Session>,
    mut fishing: ResMut<crate::fishing::Fishing>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
) {
    if settings.blocks_input() {
        return;
    }
    for (interaction, action, mut color, base) in &mut interactions {
        *color = match interaction {
            Interaction::Hovered => BackgroundColor(Color::srgb_u8(79, 96, 76)),
            Interaction::Pressed => BackgroundColor(Color::srgb_u8(101, 104, 74)),
            Interaction::None => BackgroundColor(base.0),
        };
        if *interaction != Interaction::Pressed {
            continue;
        }
        if menu.connecting && !matches!(action, Action::Back) {
            continue;
        }
        match *action {
            Action::Go(page) => menu.go(page),
            Action::Focus(field) => {
                menu.active = Some(field);
                menu.selected = true;
                menu.preedit.clear();
            }
            Action::Hosting(cloud) => {
                menu.cloud_host = cloud;
                menu.active = None;
                menu.dirty = true;
            }
            Action::Connect => menu.connect(),
            Action::Back => {
                menu.leave = true;
                menu.connecting = false;
                menu.go(Page::Home);
            }
            Action::Copy => {
                menu.status = match arboard::Clipboard::new()
                    .and_then(|mut c| c.set_text(session.invite.clone()))
                {
                    Ok(_) => tr("menu.invite_copied"),
                    Err(_) => tr("menu.invite").arg("invite", &session.invite),
                };
            }
            Action::Refresh => {
                if menu.page == Page::Cloud {
                    clubs.handle(crate::clubs::Action::Refresh, &mut menu, &mut settings);
                } else {
                    menu.refresh_rooms();
                }
            }
            Action::OfficialServer => {
                menu.server = network::DEFAULT_SERVER.trim().into();
                menu.server_password.clear();
                menu.scan = None;
                menu.rooms.clear();
                menu.dirty = true;
                menu.active = None;
                if menu.page == Page::Cloud {
                    clubs.handle(crate::clubs::Action::Official, &mut menu, &mut settings);
                }
            }
            Action::JoinRoom(index) => {
                if let Some(room) = menu.rooms.get(index).cloned() {
                    if menu.page == Page::Lan {
                        menu.address = room.address;
                    } else {
                        menu.room = room.code;
                    }
                    menu.connect();
                }
            }
            Action::NextRooms => {
                menu.room_page = (menu.room_page + 1) % menu.rooms.len().div_ceil(3).max(1);
                menu.dirty = true;
            }
            Action::Club(action) => {
                if mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed() {
                    clubs.handle(action, &mut menu, &mut settings);
                    break;
                }
            }
            Action::Fishing(action) => {
                // Rebuilt shop buttons under a held mouse must not buy repeatedly.
                if mouse.just_pressed(MouseButton::Left) {
                    fishing.command = Some(action);
                }
            }
        }
    }
}

fn insert(value: &mut String, selected: &mut bool, text: &str, limit: usize) {
    if *selected {
        value.clear();
        *selected = false;
    }
    let remaining = limit.saturating_sub(value.chars().count());
    value.extend(text.chars().filter(|c| !c.is_control()).take(remaining));
}

fn edit(
    value: &mut String,
    selected: &mut bool,
    event: &KeyboardInput,
    control: bool,
    limit: usize,
) {
    match event.key_code {
        KeyCode::KeyA if control => *selected = true,
        KeyCode::KeyV if control => {
            if let Ok(text) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                insert(value, selected, &text, limit);
            }
        }
        KeyCode::KeyC if control => {
            let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(value.clone()));
        }
        KeyCode::Backspace => {
            if *selected {
                value.clear();
                *selected = false;
            } else {
                value.pop();
            }
        }
        _ if !control => {
            if let Some(text) = &event.text {
                insert(value, selected, text, limit);
            }
        }
        _ => {}
    }
}

pub fn keyboard(
    settings: Res<crate::settings::Settings>,
    mut input: MessageReader<KeyboardInput>,
    mut ime: MessageReader<Ime>,
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<Menu>,
    mut chat: ResMut<Chat>,
    mut session: ResMut<Session>,
    mut window: Single<&mut Window>,
    mut fishing: ResMut<crate::fishing::Fishing>,
) {
    if menu.page == Page::Editor || settings.blocks_input() {
        input.clear();
        ime.clear();
        window.ime_enabled = false;
        return;
    }
    let mut composing = if chat.open {
        !chat.preedit.is_empty()
    } else {
        !menu.preedit.is_empty()
    };
    let mut committed = false;
    for event in ime.read() {
        match event {
            Ime::Preedit { value, .. } => {
                composing |= !value.is_empty();
                if chat.open {
                    chat.preedit = value.clone();
                } else {
                    menu.preedit = value.clone();
                }
            }
            Ime::Commit { value, .. } => {
                committed = true;
                if chat.open {
                    let Chat {
                        value: text,
                        selected,
                        preedit,
                        ..
                    } = &mut *chat;
                    insert(text, selected, value, 80);
                    preedit.clear();
                } else if let Some(field) = menu.active {
                    let mut selected = menu.selected;
                    insert(
                        menu.field_mut(field),
                        &mut selected,
                        value,
                        field_limit(field),
                    );
                    menu.selected = selected;
                    menu.preedit.clear();
                    menu.preview_invite();
                }
            }
            Ime::Disabled { .. } => {
                chat.preedit.clear();
                menu.preedit.clear();
            }
            _ => {}
        }
    }
    let control = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    for event in input.read().filter(|e| e.state.is_pressed()) {
        if event.logical_key == Key::Escape && !event.repeat {
            if composing {
                continue;
            }
            if chat.open {
                chat.open = false;
                chat.preedit.clear();
            } else if menu.club_editor.is_some() {
                menu.club_editor = None;
                menu.club_password.clear();
                menu.active = None;
                menu.dirty = true;
            } else if menu.page == Page::Playing && (fishing.modal() || fishing.indoors) {
                fishing.command = Some(crate::fishing::Action::Close);
            } else if menu.active.is_some() {
                menu.active = None;
            } else {
                menu.leave = true;
                menu.connecting = false;
                menu.go(Page::Home);
            }
            continue;
        }
        if event.logical_key == Key::Enter && !event.repeat {
            if menu.page == Page::Playing && fishing.modal() {
                continue;
            }
            if composing || committed {
                continue;
            }
            if menu.page == Page::Playing {
                if chat.open {
                    let text = network::clean(&chat.value, 80);
                    if text.is_empty() {
                        chat.open = false;
                        continue;
                    }
                    if let Some(link) = &session.link {
                        if link.send.try_send(ClientMessage::Chat { text }).is_ok() {
                            chat.value.clear();
                            chat.open = false;
                        }
                    } else {
                        session.log(tr("menu.draft_kept"));
                    }
                } else if session.connected {
                    chat.open = true;
                    chat.selected = false;
                }
            } else if menu.page == Page::Cloud && menu.club_editor.is_some() {
                menu.club_submit = true;
            } else if menu.page == Page::Home {
                menu.active = None;
            } else {
                menu.connect();
            }
            continue;
        }
        if menu.page != Page::Playing && event.logical_key == Key::Tab {
            let fields = menu.fields();
            if !fields.is_empty() {
                let next = menu
                    .active
                    .and_then(|f| fields.iter().position(|&x| x == f))
                    .map_or(0, |i| (i + 1) % fields.len());
                menu.active = Some(fields[next]);
                menu.selected = true;
            }
            continue;
        }
        if composing || committed {
            continue;
        }
        if chat.open {
            let Chat {
                value, selected, ..
            } = &mut *chat;
            edit(value, selected, event, control, 80);
        } else if let Some(field) = menu.active {
            let mut selected = menu.selected;
            edit(
                menu.field_mut(field),
                &mut selected,
                event,
                control,
                field_limit(field),
            );
            menu.selected = selected;
            menu.preview_invite();
        } else if menu.page == Page::Home {
            match event.key_code {
                KeyCode::Digit1 => menu.go(Page::Host),
                KeyCode::Digit2 => menu.go(Page::Lan),
                KeyCode::Digit3 => menu.go(Page::Cloud),
                KeyCode::Digit4 | KeyCode::F2 => menu.go(Page::Editor),
                _ => {}
            }
        } else if menu.page == Page::Host
            && matches!(event.key_code, KeyCode::Digit1 | KeyCode::Digit2)
        {
            menu.cloud_host = event.key_code == KeyCode::Digit2;
            menu.dirty = true;
        }
    }
    window.ime_enabled = chat.open || menu.active.is_some();
    window.ime_position = if chat.open {
        Vec2::new(100.0, window.height() - 72.0)
    } else {
        Vec2::new(window.width() * 0.2, window.height() * 0.55)
    };
}

pub fn discover(time: Res<Time>, mut elapsed: Local<f32>, mut menu: ResMut<Menu>) {
    if menu.page != Page::Lan {
        *elapsed = 0.0;
        return;
    }
    let result = menu
        .scan
        .as_ref()
        .and_then(|rx| match rx.lock().unwrap().try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(tr("menu.lobby_stopped"))),
        });
    if let Some(result) = result {
        menu.scan = None;
        match result {
            Ok(rooms) => {
                menu.rooms = rooms;
                menu.room_page = menu.room_page.min(menu.rooms.len().saturating_sub(1) / 3);
            }
            Err(error) => {
                menu.lobby_error = error;
                menu.rooms.clear();
            }
        }
        menu.dirty = true;
        *elapsed = 0.0;
    }
    *elapsed += time.delta_secs();
    if *elapsed >= 8.0 && !menu.connecting {
        menu.refresh_rooms();
        *elapsed = 0.0;
    }
}

fn field_limit(field: Field) -> usize {
    match field {
        Field::Name => 12,
        Field::Room => 280,
        Field::RoomName => 24,
        Field::ClubAlias => 32,
        Field::Port => 5,
        Field::ServerPassword | Field::ClubPassword => 128,
        _ => 200,
    }
}

pub(crate) fn label(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    text: impl Into<Message>,
    size: f32,
    color: Color,
) -> Entity {
    let text = text.into();
    let entity = commands
        .spawn((
            Text::new(text.to_string()),
            font(art, size),
            TextColor(color),
        ))
        .id();
    if matches!(text, Message::Key { .. }) {
        commands.entity(entity).insert(Localized(text));
    }
    commands.entity(parent).add_child(entity);
    entity
}

pub(crate) fn button(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    title: impl Into<Message>,
    subtitle: impl Into<Message>,
    action: Action,
    primary: bool,
) -> Entity {
    let subtitle = subtitle.into();
    let color = if primary { GREEN } else { SURFACE };
    let entity = commands
        .spawn((
            Button,
            action,
            BaseColor(color),
            Node {
                width: percent(100),
                min_height: px(if subtitle.is_empty() { 44.0 } else { 68.0 }),
                padding: UiRect::axes(px(16), px(10)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: px(6),
                border: UiRect::left(px(3)),
                ..default()
            },
            BackgroundColor(color),
            BorderColor::all(if primary { GOLD } else { EDGE }),
        ))
        .id();
    commands.entity(parent).add_child(entity);
    label(
        commands,
        entity,
        art,
        title,
        20.0,
        if primary { CREAM } else { INK },
    );
    if !subtitle.is_empty() {
        label(
            commands,
            entity,
            art,
            subtitle,
            16.0,
            if primary {
                Color::srgb_u8(206, 215, 169)
            } else {
                MUTED
            },
        );
    }
    entity
}

pub(crate) fn field(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    title: impl Into<Message>,
    which: Field,
) {
    label(commands, parent, art, title, 18.0, MUTED);
    let entity = commands
        .spawn((
            Button,
            Action::Focus(which),
            BaseColor(Color::srgb_u8(20, 32, 31)),
            Node {
                width: percent(100),
                min_height: px(48),
                padding: UiRect::all(px(12)),
                border: UiRect::all(px(2)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(20, 32, 31)),
            BorderColor::all(EDGE),
        ))
        .id();
    commands.entity(parent).add_child(entity);
    let text = label(
        commands,
        entity,
        art,
        "",
        if matches!(which, Field::Server | Field::ClubServer) {
            16.0
        } else {
            24.0
        },
        INK,
    );
    commands
        .entity(text)
        .insert((FieldValue(which), TextLayout::new_with_no_wrap()));
}

pub fn render(
    i18n: Res<I18n>,
    mut commands: Commands,
    mut menu: ResMut<Menu>,
    art: Res<Art>,
    roots: Query<Entity, With<Root>>,
    clubs: Res<crate::clubs::Browser>,
    scale: Res<UiScale>,
    mut last_scale: Local<f32>,
) {
    if !menu.dirty && !i18n.is_changed() && *last_scale == scale.0 {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    menu.dirty = false;
    *last_scale = scale.0;
    if menu.page == Page::Editor {
        return;
    }
    let root = commands
        .spawn((
            Root,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id();
    if menu.page == Page::Playing {
        let room = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(crate::settings::corner_space(scale.0)),
                    top: px(24),
                    width: px(280),
                    padding: UiRect::all(px(12)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                },
                BackgroundColor(PANEL),
            ))
            .id();
        commands.entity(root).add_child(room);
        let status = label(&mut commands, room, &art, "", 16.0, CREAM);
        commands.entity(status).insert(RoomStatus);
        button(
            &mut commands,
            room,
            &art,
            tr("menu.copy_invite"),
            "",
            Action::Copy,
            true,
        );
        let log = commands
            .spawn((
                PlayOverlay,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(24),
                    bottom: px(62),
                    width: px(460),
                    max_height: px(126),
                    overflow: Overflow::clip(),
                    padding: UiRect::all(px(12)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(22, 34, 33, 145)),
            ))
            .id();
        commands.entity(root).add_child(log);
        let text = label(&mut commands, log, &art, "", 16.0, CREAM);
        commands.entity(text).insert(ChatLog);
        let bar = commands
            .spawn((
                PlayOverlay,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(24),
                    right: px(24),
                    bottom: px(16),
                    min_height: px(38),
                    padding: UiRect::axes(px(14), px(6)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(34, 53, 47, 220)),
                BorderColor::all(Color::srgb_u8(189, 187, 150)),
            ))
            .id();
        commands.entity(root).add_child(bar);
        let input = label(&mut commands, bar, &art, "", 16.0, CREAM);
        commands.entity(input).insert(ChatValue);
        let status = label(&mut commands, bar, &art, "", 14.0, MUTED);
        commands.entity(status).insert(Status);
        return;
    }
    commands
        .entity(root)
        .insert(BackgroundColor(Color::srgba_u8(18, 29, 32, 12)));
    if menu.page == Page::Cloud {
        crate::clubs_ui::render(&mut commands, root, &art, &menu, &clubs, scale.0);
        return;
    }
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: percent(6),
                top: percent(8),
                width: px(if menu.page == Page::Home { 372 } else { 424 }),
                padding: UiRect::all(px(24)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                border: UiRect::all(px(2)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(EDGE),
        ))
        .id();
    commands.entity(root).add_child(panel);
    label(&mut commands, panel, &art, "Y A P S H I R E", 16.0, GOLD);
    label(&mut commands, panel, &art, "Yapshire", 36.0, INK);
    if menu.page == Page::Home {
        label(&mut commands, panel, &art, tr("menu.intro"), 18.0, MUTED);
    }
    match menu.page {
        Page::Home => {
            field(&mut commands, panel, &art, tr("menu.nickname"), Field::Name);
            button(
                &mut commands,
                panel,
                &art,
                tr("menu.host"),
                tr("menu.host_hint"),
                Action::Go(Page::Host),
                true,
            );
            button(
                &mut commands,
                panel,
                &art,
                tr("menu.lan"),
                tr("menu.lan_hint"),
                Action::Go(Page::Lan),
                false,
            );
            button(
                &mut commands,
                panel,
                &art,
                tr("menu.cloud"),
                tr("menu.cloud_hint"),
                Action::Go(Page::Cloud),
                false,
            );
            button(
                &mut commands,
                panel,
                &art,
                tr("menu.editor"),
                "",
                Action::Go(Page::Editor),
                false,
            );
            label(
                &mut commands,
                panel,
                &art,
                tr("menu.choose_hint"),
                14.0,
                MUTED,
            );
        }
        Page::Host => {
            label(&mut commands, panel, &art, tr("menu.host_title"), 24.0, INK);
            button(
                &mut commands,
                panel,
                &art,
                tr("menu.host_lan"),
                "",
                Action::Hosting(false),
                !menu.cloud_host,
            );
            button(
                &mut commands,
                panel,
                &art,
                tr("menu.host_cloud"),
                "",
                Action::Hosting(true),
                menu.cloud_host,
            );
            if menu.cloud_host {
                field(&mut commands, panel, &art, tr("menu.server"), Field::Server);
                field(
                    &mut commands,
                    panel,
                    &art,
                    tr("menu.server_password"),
                    Field::ServerPassword,
                );
                field(
                    &mut commands,
                    panel,
                    &art,
                    tr("menu.room_name"),
                    Field::RoomName,
                );
            } else {
                field(&mut commands, panel, &art, tr("menu.port"), Field::Port);
            }
            button(
                &mut commands,
                panel,
                &art,
                if menu.connecting {
                    tr("menu.opening")
                } else {
                    tr("menu.open")
                },
                "",
                Action::Connect,
                true,
            );
            button(
                &mut commands,
                panel,
                &art,
                tr("common.back"),
                "",
                Action::Back,
                false,
            );
        }
        Page::Lan => {
            label(&mut commands, panel, &art, tr("menu.lan_title"), 24.0, INK);
            label(
                &mut commands,
                panel,
                &art,
                tr("menu.invite_hint"),
                18.0,
                MUTED,
            );
            field(
                &mut commands,
                panel,
                &art,
                tr("menu.address"),
                Field::Address,
            );
            label(
                &mut commands,
                panel,
                &art,
                tr("menu.address_hint"),
                16.0,
                MUTED,
            );
            button(
                &mut commands,
                panel,
                &art,
                if menu.connecting {
                    tr("menu.connecting")
                } else {
                    tr("menu.join")
                },
                "",
                Action::Connect,
                true,
            );
            button(
                &mut commands,
                panel,
                &art,
                tr("common.back"),
                "",
                Action::Back,
                false,
            );
        }
        Page::Cloud | Page::Playing | Page::Editor => {}
    }
    let status = label(
        &mut commands,
        panel,
        &art,
        "",
        16.0,
        Color::srgb_u8(225, 158, 112),
    );
    commands.entity(status).insert(Status);
    let caption = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: percent(62),
            top: percent(12),
            width: px(460),
            flex_direction: FlexDirection::Column,
            row_gap: px(16),
            ..default()
        })
        .id();
    commands.entity(root).add_child(caption);
    if menu.page == Page::Host && menu.cloud_host {
        button(
            &mut commands,
            caption,
            &art,
            tr("menu.official_server"),
            "",
            Action::OfficialServer,
            false,
        );
        if menu.page == Page::Host {
            label(
                &mut commands,
                caption,
                &art,
                tr("menu.server_saved_hint"),
                16.0,
                CREAM,
            );
        }
    }
    label(
        &mut commands,
        caption,
        &art,
        tr("menu.caption"),
        if menu.page == Page::Cloud { 28.0 } else { 24.0 },
        Color::srgb_u8(29, 47, 51),
    );
    if menu.page == Page::Lan {
        let lobby = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(62),
                    top: percent(34),
                    width: px(460),
                    padding: UiRect::all(px(22)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(EDGE),
            ))
            .id();
        commands.entity(root).add_child(lobby);
        label(
            &mut commands,
            lobby,
            &art,
            if menu.page == Page::Lan {
                tr("menu.lobby_lan")
            } else {
                tr("menu.lobby_cloud")
            },
            24.0,
            INK,
        );
        let detail = if menu.scan.is_some() {
            tr("menu.looking")
        } else {
            tr("menu.room_count").arg("count", menu.rooms.len().to_string())
        };
        label(&mut commands, lobby, &art, &detail, 16.0, MUTED);
        if menu.rooms.is_empty() && menu.scan.is_none() {
            label(
                &mut commands,
                lobby,
                &art,
                if menu.lobby_error.is_empty() {
                    tr("menu.no_rooms")
                } else {
                    menu.lobby_error.clone()
                },
                18.0,
                INK,
            );
        }
        for (index, room) in menu
            .rooms
            .iter()
            .enumerate()
            .skip(menu.room_page * 3)
            .take(3)
        {
            let subtitle = tr("menu.room_players")
                .arg("count", room.players.to_string())
                .arg(
                    "invite",
                    if menu.page == Page::Lan {
                        &room.address
                    } else {
                        &room.code
                    },
                );
            let title = if menu.page == Page::Lan {
                tr("menu.friend_town").arg("name", &room.name)
            } else {
                room.name.clone().into()
            };
            button(
                &mut commands,
                lobby,
                &art,
                title,
                &subtitle,
                Action::JoinRoom(index),
                false,
            );
        }
        if menu.rooms.len() > 3 {
            button(
                &mut commands,
                lobby,
                &art,
                tr("menu.next_page")
                    .arg("page", (menu.room_page + 1).to_string())
                    .arg("pages", menu.rooms.len().div_ceil(3).to_string()),
                "",
                Action::NextRooms,
                false,
            );
        }
        button(
            &mut commands,
            lobby,
            &art,
            tr("menu.refresh"),
            "",
            Action::Refresh,
            true,
        );
    }
    if menu.page != Page::Cloud {
        label(
            &mut commands,
            caption,
            &art,
            tr("menu.slow"),
            16.0,
            Color::srgb_u8(42, 65, 67),
        );
    }
    let footer = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: px(36),
            bottom: px(24),
            ..default()
        })
        .id();
    commands.entity(root).add_child(footer);
    label(&mut commands, footer, &art, tr("menu.footer"), 16.0, CREAM);
}

pub fn refresh(
    i18n: Res<I18n>,
    menu: Res<Menu>,
    chat: Res<Chat>,
    session: Res<Session>,
    time: Res<Time>,
    fishing: Res<crate::fishing::Fishing>,
    actors: Query<&Actor>,
    mut text: Query<(
        &mut Text,
        Option<&FieldValue>,
        Option<&Status>,
        Option<&ChatValue>,
        Option<&ChatLog>,
        Option<&RoomStatus>,
    )>,
    mut borders: Query<(&Action, &mut BorderColor)>,
    mut overlays: Query<&mut Node, With<PlayOverlay>>,
) {
    for mut node in &mut overlays {
        node.display = if fishing.modal() && !chat.open {
            Display::None
        } else {
            Display::Flex
        };
    }
    let cursor = if (time.elapsed_secs() * 2.0) as u32 % 2 == 0 {
        "_"
    } else {
        " "
    };
    for (mut text, field, status, input, log, room) in &mut text {
        let value: Message = if let Some(FieldValue(field)) = field {
            let raw = menu.field(*field);
            let masked = "*".repeat(raw.chars().count());
            let value = if matches!(field, Field::ServerPassword | Field::ClubPassword) {
                masked.as_str()
            } else {
                raw
            };
            if menu.active == Some(*field) {
                let preedit = if matches!(field, Field::ServerPassword | Field::ClubPassword) {
                    "*".repeat(menu.preedit.chars().count())
                } else {
                    menu.preedit.clone()
                };
                format!("{value}{preedit}{cursor}").into()
            } else if value.is_empty() {
                tr("menu.type")
            } else {
                value.into()
            }
        } else if status.is_some() {
            if menu.page == Page::Playing && menu.status.is_empty() && chat.open {
                tr("menu.chat_help")
            } else {
                menu.status.clone()
            }
        } else if input.is_some() {
            if chat.open {
                tr("menu.chat_input")
                    .arg("text", &chat.value)
                    .arg("preedit", &chat.preedit)
                    .arg("cursor", cursor)
            } else {
                tr("menu.controls")
            }
        } else if log.is_some() {
            session
                .log
                .iter()
                .map(|message| message.render(&i18n))
                .collect::<Vec<_>>()
                .join("\n")
                .into()
        } else if room.is_some() {
            tr("menu.room_status")
                .arg("room", &session.label)
                .arg(
                    "state",
                    if session.connected {
                        tr("common.connected")
                    } else {
                        tr("common.disconnected")
                    },
                )
                .arg("count", actors.iter().count().to_string())
        } else {
            continue;
        };
        let value = value.render(&i18n);
        if **text != value {
            **text = value;
        }
    }
    for (action, mut border) in &mut borders {
        if let Action::Focus(field) = action {
            *border = BorderColor::all(if menu.active == Some(*field) {
                GREEN
            } else {
                Color::srgb_u8(195, 193, 158)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_mouse_does_not_repeat_purchases_when_shop_buttons_are_rebuilt() {
        let mut app = App::new();
        app.init_resource::<Menu>()
            .init_resource::<Session>()
            .init_resource::<crate::settings::Settings>()
            .init_resource::<crate::fishing::Fishing>()
            .init_resource::<crate::clubs::Browser>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Touches>()
            .add_systems(Update, buttons);
        let spawn = |world: &mut World| {
            world.spawn((
                Interaction::Pressed,
                Action::Fishing(crate::fishing::Action::Bait),
                BackgroundColor(GREEN),
                BaseColor(GREEN),
            ));
        };
        spawn(app.world_mut());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<crate::fishing::Fishing>()
                .command
                .take(),
            Some(crate::fishing::Action::Bait)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        spawn(app.world_mut());
        app.update();
        assert!(
            app.world()
                .resource::<crate::fishing::Fishing>()
                .command
                .is_none()
        );
    }

    #[test]
    fn host_and_join_use_the_selected_server() {
        let mut menu = Menu {
            page: Page::Host,
            cloud_host: true,
            server: "ws://other.example".into(),
            ..default()
        };
        assert!(menu.fields() == vec![Field::Server, Field::ServerPassword, Field::RoomName]);
        menu.connect();
        assert!(menu.request.is_none());
        assert_eq!(menu.status, tr("menu.room_required"));
        menu.room_name = " Sunset & Friends\n ".into();
        menu.connect();
        assert!(
            matches!(menu.request.take(), Some(Mode::HostCloud { server, room_name }) if server == "ws://other.example" && room_name == "Sunset & Friends")
        );
        menu.page = Page::Cloud;
        menu.room = "MAIN0001".into();
        menu.connect();
        assert!(
            matches!(menu.request.take(), Some(Mode::JoinCloud { server, room }) if server == "ws://other.example" && room == "MAIN0001")
        );
        menu.server_password = "private-password".into();
        *menu.field_mut(Field::Server) = "wss://new.example".into();
        assert!(menu.server_password.is_empty());
    }
    #[test]
    fn pasting_an_invite_shows_its_server_and_clears_the_previous_password() {
        let mut menu = Menu {
            page: Page::Cloud,
            active: Some(Field::Room),
            server: "wss://private.example".into(),
            server_password: "private-password".into(),
            room: "wss://friends.example/room/NIANNIAN".into(),
            ..default()
        };
        menu.preview_invite();
        assert_eq!(menu.server, "wss://friends.example");
        assert_eq!(menu.room, "NIANNIAN");
        assert!(menu.server_password.is_empty());
        assert!(menu.request.is_none());
        menu.connect();
        assert!(
            matches!(menu.request.take(), Some(Mode::JoinCloud { server, room })
            if server == "wss://friends.example" && room == "NIANNIAN")
        );

        menu.server_password = "same-server-password".into();
        menu.room = "wss://friends.example/room/ABCDEFGH".into();
        menu.preview_invite();
        assert_eq!(menu.server_password, "same-server-password");
        menu.room = "wss://other.example/room/ABCDEFGH?password=secret".into();
        menu.connect();
        assert!(menu.request.is_none());
        assert_eq!(menu.server, "wss://friends.example");
    }

    #[test]
    fn text_input_preserves_unicode_and_replaces_selection() {
        let mut value = "你好".to_owned();
        let mut selected = false;
        insert(&mut value, &mut selected, "呀\n朋友", 4);
        assert_eq!(value, "你好呀朋");
        value.pop();
        assert_eq!(value, "你好呀");
        selected = true;
        insert(&mut value, &mut selected, "小风", 12);
        assert_eq!(value, "小风");
        assert!(!selected);
    }
}
