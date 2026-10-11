//! Pixel noticeboard: each saved Club owns the room rows beneath its heading.
use crate::{
    clubs::{Action as ClubAction, Browser, Club},
    game::Art,
    i18n::{Message, tr},
    icons::Icon,
    settings::Settings,
    ui::{self, Action, BaseColor, CREAM, Field, GREEN, INK, MUTED, Menu, Page},
};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    ui::FocusPolicy,
};

const SAND: Color = ui::SURFACE;
const PAPER: Color = Color::srgb_u8(20, 32, 31);
const WARN: Color = Color::srgb_u8(225, 158, 112);

#[derive(Component)]
pub(crate) struct RoomScroll;

fn node(commands: &mut Commands, parent: Entity, node: Node) -> Entity {
    commands.spawn((node, ChildOf(parent))).id()
}

fn icon(commands: &mut Commands, parent: Entity, art: &Art, icon: Icon, size: f32) {
    commands.spawn((
        icon.image(art),
        Node {
            width: px(size),
            height: px(size),
            flex_shrink: 0.0,
            ..default()
        },
        FocusPolicy::Pass,
        ChildOf(parent),
    ));
}

fn button(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    text: impl Into<Message>,
    icon_kind: Icon,
    action: Action,
    primary: bool,
) -> Entity {
    let color = if primary { GREEN } else { SAND };
    let entity = commands
        .spawn((
            Button,
            action,
            BaseColor(color),
            Node {
                min_height: px(38),
                padding: UiRect::axes(px(10), px(7)),
                column_gap: px(8),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::bottom(px(2)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(color),
            BorderColor::all(if primary { INK } else { MUTED }),
            ChildOf(parent),
        ))
        .id();
    icon(commands, entity, art, icon_kind, 24.0);
    ui::label(
        commands,
        entity,
        art,
        text,
        18.0,
        if primary { CREAM } else { INK },
    );
    entity
}

fn club_button(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    key: &'static str,
    icon: Icon,
    action: ClubAction,
    primary: bool,
) -> Entity {
    button(
        commands,
        parent,
        art,
        tr(key),
        icon,
        Action::Club(action),
        primary,
    )
}

fn row(commands: &mut Commands, parent: Entity) -> Entity {
    node(
        commands,
        parent,
        Node {
            column_gap: px(8),
            align_items: AlignItems::Center,
            ..default()
        },
    )
}

pub fn render(
    commands: &mut Commands,
    root: Entity,
    art: &Art,
    menu: &Menu,
    browser: &Browser,
    scale: f32,
) {
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(40),
                right: px(40),
                top: px(40),
                bottom: px(40),
                padding: UiRect::all(px(22)),
                border: UiRect::all(px(3)),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            BackgroundColor(ui::PANEL),
            BorderColor::all(MUTED),
            ChildOf(root),
        ))
        .id();

    let header = row(commands, panel);
    icon(commands, header, art, Icon::Town, 40.0);
    let title = node(
        commands,
        header,
        Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            ..default()
        },
    );
    ui::label(commands, title, art, tr("clubs.title"), 30.0, INK);
    ui::label(commands, title, art, tr("clubs.subtitle"), 14.0, MUTED);
    club_button(
        commands,
        header,
        art,
        "clubs.add",
        Icon::Folder,
        ClubAction::Add,
        true,
    );
    club_button(
        commands,
        header,
        art,
        "clubs.refresh",
        Icon::Reload,
        ClubAction::Refresh,
        false,
    );
    button(
        commands,
        header,
        art,
        tr("common.back"),
        Icon::Left,
        Action::Back,
        false,
    );
    node(
        commands,
        header,
        Node {
            width: px((crate::settings::corner_space(scale) - 66.0).max(0.0)),
            flex_shrink: 0.0,
            ..default()
        },
    );

    let body = node(
        commands,
        panel,
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            column_gap: px(20),
            ..default()
        },
    );
    let sidebar = commands
        .spawn((
            Node {
                width: px(336),
                flex_shrink: 0.0,
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(PAPER),
            ChildOf(body),
        ))
        .id();

    if let Some(editing) = menu.club_editor {
        ui::label(
            commands,
            sidebar,
            art,
            tr(if editing.is_some() {
                "clubs.edit_title"
            } else {
                "clubs.add_title"
            }),
            24.0,
            INK,
        );
        ui::label(commands, sidebar, art, tr("clubs.alias_hint"), 16.0, MUTED);
        ui::field(commands, sidebar, art, tr("clubs.alias"), Field::ClubAlias);
        ui::field(commands, sidebar, art, tr("menu.server"), Field::ClubServer);
        ui::label(
            commands,
            sidebar,
            art,
            "wss://yap-server.mmstudio.games",
            14.0,
            MUTED,
        );
        ui::field(
            commands,
            sidebar,
            art,
            tr("menu.server_password"),
            Field::ClubPassword,
        );
        ui::label(
            commands,
            sidebar,
            art,
            tr("clubs.password_hint"),
            14.0,
            MUTED,
        );
        club_button(
            commands,
            sidebar,
            art,
            "clubs.save",
            Icon::Save,
            ClubAction::Save,
            true,
        );
        club_button(
            commands,
            sidebar,
            art,
            "clubs.cancel",
            Icon::Left,
            ClubAction::Cancel,
            false,
        );
    } else {
        ui::label(commands, sidebar, art, tr("clubs.selected"), 14.0, MUTED);
        if let Some(club) = browser.selected() {
            let id = club.saved.id;
            ui::label(commands, sidebar, art, club.saved.alias.clone(), 24.0, INK);
            ui::label(
                commands,
                sidebar,
                art,
                club.saved.server.clone(),
                14.0,
                MUTED,
            );
            let actions = row(commands, sidebar);
            club_button(
                commands,
                actions,
                art,
                "clubs.edit",
                Icon::Settings,
                ClubAction::Edit(id),
                false,
            );
            club_button(
                commands,
                actions,
                art,
                "clubs.remove",
                Icon::Discard,
                ClubAction::Remove(id),
                false,
            );
            if browser.removing == Some(id) {
                ui::label(commands, sidebar, art, tr("clubs.remove_hint"), 16.0, WARN);
                club_button(
                    commands,
                    sidebar,
                    art,
                    "clubs.confirm_remove",
                    Icon::Discard,
                    ClubAction::ConfirmRemove(id),
                    false,
                );
                club_button(
                    commands,
                    sidebar,
                    art,
                    "clubs.cancel",
                    Icon::Left,
                    ClubAction::CancelRemove,
                    false,
                );
            } else {
                ui::field(
                    commands,
                    sidebar,
                    art,
                    tr("menu.server_password"),
                    Field::ServerPassword,
                );
                club_button(
                    commands,
                    sidebar,
                    art,
                    "clubs.apply_password",
                    Icon::Reload,
                    ClubAction::RefreshSelected,
                    false,
                );
                ui::label(
                    commands,
                    sidebar,
                    art,
                    tr("clubs.password_hint"),
                    14.0,
                    MUTED,
                );
                club_button(
                    commands,
                    sidebar,
                    art,
                    "clubs.host",
                    Icon::Town,
                    ClubAction::Host(id),
                    true,
                );
            }
        } else {
            ui::label(commands, sidebar, art, tr("clubs.choose"), 18.0, INK);
            if !menu.server.is_empty() {
                ui::label(commands, sidebar, art, menu.server.clone(), 14.0, MUTED);
                ui::field(
                    commands,
                    sidebar,
                    art,
                    tr("menu.server_password"),
                    Field::ServerPassword,
                );
            }
            club_button(
                commands,
                sidebar,
                art,
                "clubs.official",
                Icon::Town,
                ClubAction::Official,
                false,
            );
        }
        if browser.removing.is_none() {
            ui::field(commands, sidebar, art, tr("menu.room_code"), Field::Room);
            button(
                commands,
                sidebar,
                art,
                if menu.connecting {
                    tr("menu.connecting")
                } else {
                    tr("clubs.join_invite")
                },
                Icon::Right,
                Action::Connect,
                false,
            );
        }
    }
    let status = ui::label(commands, sidebar, art, "", 16.0, WARN);
    commands.entity(status).insert(ui::Status);

    let directory = node(
        commands,
        body,
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        },
    );
    let list = commands
        .spawn((
            RoomScroll,
            ScrollPosition(Vec2::new(0.0, browser.scroll)),
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                padding: UiRect::right(px(8)),
                ..default()
            },
            ChildOf(directory),
        ))
        .id();
    if browser.clubs.is_empty() {
        ui::label(commands, list, art, tr("clubs.empty"), 24.0, INK);
    }
    for club in &browser.clubs {
        render_club(commands, list, art, club, browser.selected);
    }
    let navigation = row(commands, directory);
    let count = ui::label(
        commands,
        navigation,
        art,
        tr("clubs.total")
            .arg("clubs", browser.clubs.len().to_string())
            .arg(
                "rooms",
                browser
                    .clubs
                    .iter()
                    .filter_map(|c| c.snapshot.as_ref())
                    .map(|s| s.rooms.len())
                    .sum::<usize>()
                    .to_string(),
            ),
        14.0,
        MUTED,
    );
    commands.entity(count).insert(Node {
        flex_grow: 1.0,
        ..default()
    });
    club_button(
        commands,
        navigation,
        art,
        "clubs.up",
        Icon::Left,
        ClubAction::ScrollUp,
        false,
    );
    club_button(
        commands,
        navigation,
        art,
        "clubs.down",
        Icon::Right,
        ClubAction::ScrollDown,
        false,
    );
    ui::label(commands, panel, art, tr("clubs.latency_hint"), 14.0, MUTED);
}

fn render_club(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    club: &Club,
    selected: Option<u64>,
) {
    let id = club.saved.id;
    let card = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                padding: UiRect::all(px(10)),
                row_gap: px(7),
                border: UiRect::all(px(2)),
                ..default()
            },
            BackgroundColor(PAPER),
            BorderColor::all(if selected == Some(id) { GREEN } else { SAND }),
            ChildOf(parent),
        ))
        .id();
    let header = row(commands, card);
    let color = if selected == Some(id) { GREEN } else { SAND };
    let title = commands
        .spawn((
            Button,
            Action::Club(ClubAction::Select(id)),
            BaseColor(color),
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                flex_basis: px(0),
                padding: UiRect::all(px(9)),
                column_gap: px(10),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(color),
            ChildOf(header),
        ))
        .id();
    icon(commands, title, art, Icon::Town, 32.0);
    let caption = node(
        commands,
        title,
        Node {
            min_width: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            ..default()
        },
    );
    ui::label(
        commands,
        caption,
        art,
        club.saved.alias.clone(),
        23.0,
        if selected == Some(id) { CREAM } else { INK },
    );
    ui::label(
        commands,
        caption,
        art,
        club.saved.server.clone(),
        13.0,
        if selected == Some(id) { CREAM } else { MUTED },
    );
    let counts = node(
        commands,
        header,
        Node {
            width: px(164),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            ..default()
        },
    );
    let state = if club.pending.is_some() {
        "clubs.refreshing"
    } else if club.error.is_some() {
        "clubs.offline"
    } else if club.snapshot.is_some() {
        "clubs.online"
    } else {
        "clubs.waiting"
    };
    ui::label(
        commands,
        counts,
        art,
        tr(state),
        16.0,
        if club.error.is_some() { WARN } else { ui::GOLD },
    );
    if let Some(snapshot) = &club.snapshot {
        ui::label(
            commands,
            counts,
            art,
            tr("clubs.summary")
                .arg("rooms", snapshot.rooms.len().to_string())
                .arg(
                    "players",
                    snapshot
                        .rooms
                        .iter()
                        .map(|r| r.players)
                        .sum::<usize>()
                        .to_string(),
                ),
            14.0,
            MUTED,
        );
    }
    club_button(
        commands,
        header,
        art,
        if club.expanded {
            "clubs.collapse"
        } else {
            "clubs.expand"
        },
        if club.expanded {
            Icon::Eye
        } else {
            Icon::EyeClosed
        },
        ClubAction::Toggle(id),
        false,
    );

    if let Some(error) = &club.error {
        ui::label(commands, card, art, error.clone(), 15.0, WARN);
        if club.snapshot.is_some() {
            ui::label(commands, card, art, tr("clubs.stale"), 14.0, WARN);
        }
    }
    if !club.expanded {
        return;
    }
    if let Some(snapshot) = &club.snapshot {
        if snapshot.rooms.is_empty() {
            ui::label(commands, card, art, tr("clubs.no_rooms"), 17.0, MUTED);
        }
        for room in &snapshot.rooms {
            let room_row = commands
                .spawn((
                    Node {
                        min_height: px(64),
                        margin: UiRect::left(px(14)),
                        padding: UiRect::axes(px(12), px(9)),
                        column_gap: px(10),
                        align_items: AlignItems::Center,
                        border: UiRect::left(px(3)),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(ui::PANEL),
                    BorderColor::all(SAND),
                    ChildOf(card),
                ))
                .id();
            let title = node(
                commands,
                room_row,
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
            );
            ui::label(commands, title, art, room.name.clone(), 22.0, INK);
            ui::label(commands, title, art, room.code.clone(), 14.0, MUTED);
            let count = if room.capacity == 0 {
                tr("clubs.players_unknown").arg("count", room.players.to_string())
            } else {
                tr("clubs.players")
                    .arg("count", room.players.to_string())
                    .arg("capacity", room.capacity.to_string())
            };
            let details = node(
                commands,
                room_row,
                Node {
                    width: px(128),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5),
                    ..default()
                },
            );
            ui::label(commands, details, art, count, 18.0, INK);
            let latency = if club.error.is_some() {
                tr("clubs.latency_unknown")
            } else {
                snapshot.latency_ms.map_or_else(
                    || tr("clubs.latency_unknown"),
                    |ms| tr("clubs.latency").arg("ms", ms.to_string()),
                )
            };
            ui::label(
                commands,
                details,
                art,
                latency,
                16.0,
                if snapshot.latency_ms.is_some_and(|ms| ms > 250) {
                    WARN
                } else {
                    ui::GOLD
                },
            );
            if club.error.is_some() || (room.capacity > 0 && room.players >= room.capacity) {
                let label = ui::label(
                    commands,
                    room_row,
                    art,
                    tr(if club.error.is_some() {
                        "clubs.offline"
                    } else {
                        "clubs.full"
                    }),
                    18.0,
                    MUTED,
                );
                commands.entity(label).insert(Node {
                    width: px(88),
                    flex_shrink: 0.0,
                    ..default()
                });
            } else if let Ok(code) = room.code.as_bytes().try_into() {
                club_button(
                    commands,
                    room_row,
                    art,
                    "clubs.join",
                    Icon::Right,
                    ClubAction::Join(id, code),
                    true,
                );
            }
        }
    } else if club.error.is_none() {
        ui::label(commands, card, art, tr("clubs.waiting"), 17.0, MUTED);
    }
}

pub fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    settings: Res<Settings>,
    menu: Res<Menu>,
    mut browser: ResMut<Browser>,
    mut areas: Query<(&ComputedNode, &mut ScrollPosition), With<RoomScroll>>,
) {
    let delta: f32 = wheel
        .read()
        .map(|event| {
            event.y
                * if event.unit == MouseScrollUnit::Line {
                    36.0
                } else {
                    1.0
                }
        })
        .sum();
    if menu.page != Page::Cloud || settings.blocks_input() {
        return;
    }
    for (computed, mut position) in &mut areas {
        // Refreshed UI entities have no measured layout until the next frame.
        // Preserve the scroll offset through that frame instead of clamping to 0.
        if computed.size().y <= 0.0 {
            continue;
        }
        let max = ((computed.content_size().y - computed.size().y)
            * computed.inverse_scale_factor())
        .max(0.0);
        browser.scroll = (browser.scroll - delta).clamp(0.0, max);
        position.0.y = browser.scroll;
    }
}
