use crate::{
    game::Art,
    i18n::{I18n, tr},
    icons::Icon,
    settings::Settings,
    ui::{self, Field, Menu, Page},
    wilds::{Action, Wilds},
};
use bevy::{
    prelude::*,
    ui::{FocusPolicy, RelativeCursorPosition},
};
use yapshire_shared::construction::Layer;
mod palette;

#[derive(Component)]
pub(crate) struct Root;
#[derive(Component)]
#[require(RelativeCursorPosition)]
pub(crate) struct Panel;

fn panel(commands: &mut Commands, parent: Entity, node: Node) -> Entity {
    commands
        .spawn((
            Panel,
            node,
            BackgroundColor(ui::PANEL),
            BorderColor::all(ui::EDGE),
            ChildOf(parent),
        ))
        .id()
}

fn action(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    key: &'static str,
    icon: Icon,
    act: Action,
) -> Entity {
    let button = ui::button(
        commands,
        parent,
        art,
        if key.is_empty() { "".into() } else { tr(key) },
        "",
        ui::Action::Wild(act),
        false,
    );
    commands.entity(button).insert(Node {
        padding: UiRect::axes(px(12), px(8)),
        min_height: px(44),
        column_gap: px(10),
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        ..default()
    });
    let image = commands
        .spawn((
            icon.image(art),
            Node {
                width: px(24),
                height: px(24),
                flex_shrink: 0.0,
                ..default()
            },
            FocusPolicy::Pass,
        ))
        .id();
    commands.entity(button).insert_children(0, &[image]);
    button
}

pub(crate) fn render(
    mut commands: Commands,
    mut wilds: ResMut<Wilds>,
    menu: Res<Menu>,
    art: Res<Art>,
    assets: Res<AssetServer>,
    maps: Res<crate::maps::Maps>,
    palette_scroll: Query<(&palette::Scroll, &ScrollPosition)>,
    settings: Res<Settings>,
    i18n: Res<I18n>,
    roots: Query<Entity, With<Root>>,
    mut previous: Local<Option<(Page, Vec2)>>,
) {
    let layout = (menu.page, crate::game::ui_size(&settings));
    if !wilds.dirty && !i18n.is_changed() && *previous == Some(layout) {
        return;
    }
    *previous = Some(layout);
    wilds.dirty = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !matches!(menu.page, Page::Worlds | Page::Exploring) {
        return;
    }
    let root = commands
        .spawn((
            Root,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            FocusPolicy::Pass,
        ))
        .id();
    if menu.page == Page::Worlds {
        let frame = panel(
            &mut commands,
            root,
            Node {
                position_type: PositionType::Absolute,
                left: px(24),
                right: px(24),
                top: px(24),
                bottom: px(24),
                padding: UiRect::all(px(22)),
                border: UiRect::all(px(2)),
                row_gap: px(14),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        );
        ui::label(&mut commands, frame, &art, tr("wilds.title"), 30.0, ui::INK);
        ui::label(
            &mut commands,
            frame,
            &art,
            tr("wilds.subtitle"),
            16.0,
            ui::MUTED,
        );
        let body = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    column_gap: px(24),
                    ..default()
                },
                ChildOf(frame),
            ))
            .id();
        let create = panel(
            &mut commands,
            body,
            Node {
                width: px(320),
                flex_shrink: 0.0,
                overflow: Overflow::scroll_y(),
                ..default()
            },
        );
        let create = ui::scroll_content(&mut commands, create, 12.0, false);
        ui::field(
            &mut commands,
            create,
            &art,
            tr("wilds.name"),
            Field::WorldName,
        );
        ui::field(
            &mut commands,
            create,
            &art,
            tr("wilds.seed"),
            Field::WorldSeed,
        );
        ui::label(
            &mut commands,
            create,
            &art,
            tr("wilds.seed_hint"),
            14.0,
            ui::MUTED,
        );
        action(
            &mut commands,
            create,
            &art,
            "wilds.create",
            Icon::Town,
            Action::Create,
        );
        ui::button(
            &mut commands,
            create,
            &art,
            tr("common.back"),
            "",
            ui::Action::Back,
            false,
        );
        let list = panel(
            &mut commands,
            body,
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        );
        let list = ui::scroll_content(&mut commands, list, 12.0, false);
        if wilds.entries.is_empty() {
            ui::label(
                &mut commands,
                list,
                &art,
                tr("wilds.empty"),
                18.0,
                ui::MUTED,
            );
        }
        for (i, entry) in wilds.entries.iter().enumerate() {
            let subtitle = entry.seed.map_or_else(
                || tr("wilds.unreadable"),
                |seed| tr("wilds.seed_value").arg("seed", seed.to_string()),
            );
            let button = ui::button(
                &mut commands,
                list,
                &art,
                entry.name.clone(),
                subtitle,
                ui::Action::Wild(Action::Open(i)),
                false,
            );
            commands.entity(button).insert(Node {
                min_height: px(86),
                width: percent(100),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                flex_shrink: 0.0,
                ..default()
            });
        }
        if !wilds.notice.is_empty() {
            ui::label(
                &mut commands,
                frame,
                &art,
                wilds.notice.clone(),
                16.0,
                ui::GOLD,
            );
        }
        return;
    }
    let Some(run) = &wilds.active else {
        return;
    };
    let heading = panel(
        &mut commands,
        root,
        Node {
            position_type: PositionType::Absolute,
            left: px(24),
            top: px(24),
            max_width: percent(55),
            padding: UiRect::all(px(12)),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            ..default()
        },
    );
    ui::label(
        &mut commands,
        heading,
        &art,
        run.saved.terrain.name.clone(),
        22.0,
        ui::INK,
    );
    ui::label(
        &mut commands,
        heading,
        &art,
        tr("wilds.seed_value").arg("seed", run.saved.terrain.seed.to_string()),
        14.0,
        ui::MUTED,
    );
    ui::label(
        &mut commands,
        heading,
        &art,
        wilds.notice.clone(),
        14.0,
        ui::GOLD,
    );
    let actions = panel(
        &mut commands,
        root,
        Node {
            position_type: PositionType::Absolute,
            right: px(84),
            top: px(24),
            column_gap: px(8),
            ..default()
        },
    );
    action(
        &mut commands,
        actions,
        &art,
        "wilds.save",
        Icon::Save,
        Action::Save,
    );
    action(
        &mut commands,
        actions,
        &art,
        "wilds.leave",
        Icon::Left,
        Action::Leave,
    );
    let footer = panel(
        &mut commands,
        root,
        Node {
            position_type: PositionType::Absolute,
            left: px(24),
            right: px(24),
            bottom: px(18),
            padding: UiRect::all(px(10)),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        },
    );
    let bar = commands
        .spawn((
            Node {
                column_gap: px(8),
                ..default()
            },
            ChildOf(footer),
        ))
        .id();
    for (slot, id) in ["yapshire:earth", "yapshire:rock", "yapshire:sand"]
        .into_iter()
        .enumerate()
    {
        let key = ["wilds.dirt", "wilds.stone", "wilds.sand"][slot];
        let title = tr("wilds.material")
            .arg("key", (slot + 1).to_string())
            .arg("name", tr(key))
            .arg("count", run.saved.terrain.inventory[slot].to_string());
        let button = ui::button(
            &mut commands,
            bar,
            &art,
            title,
            "",
            ui::Action::Wild(Action::Select(slot)),
            !run.using_catalog && slot == run.selected,
        );
        commands.entity(button).insert(Node {
            padding: UiRect::all(px(10)),
            column_gap: px(10),
            min_height: px(50),
            align_items: AlignItems::Center,
            flex_grow: 1.0,
            flex_basis: px(0),
            ..default()
        });
        if let Some(piece) = run.catalog.get(id) {
            palette::image(&mut commands, button, piece, &maps, &assets, 28.0);
        }
    }
    action(
        &mut commands,
        bar,
        &art,
        "wilds.build",
        Icon::Tiles,
        Action::Palette,
    );
    let layers = commands
        .spawn((
            Node {
                column_gap: px(8),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(footer),
        ))
        .id();
    for layer in Layer::ALL {
        let button = ui::button(
            &mut commands,
            layers,
            &art,
            tr(palette::layer_key(layer)),
            "",
            ui::Action::Wild(Action::Layer(layer)),
            run.layer == layer,
        );
        commands.entity(button).insert(Node {
            padding: UiRect::axes(px(10), px(6)),
            min_height: px(30),
            align_items: AlignItems::Center,
            ..default()
        });
    }
    if run.using_catalog {
        let piece = &run.catalog.pieces[run.selected_piece];
        palette::image(&mut commands, layers, piece, &maps, &assets, 24.0);
        ui::label(
            &mut commands,
            layers,
            &art,
            tr("wilds.selected_piece").arg("name", palette::title(piece, &i18n)),
            16.0,
            ui::GOLD,
        );
    }
    if run.palette_open {
        let scroll = palette_scroll
            .iter()
            .find(|(group, _)| group.0 == run.palette_group)
            .map_or(Vec2::ZERO, |(_, position)| position.0);
        palette::render(
            &mut commands,
            root,
            run,
            &art,
            &maps,
            &assets,
            &i18n,
            scroll,
        );
    }
    ui::label(
        &mut commands,
        footer,
        &art,
        tr("wilds.controls"),
        14.0,
        ui::MUTED,
    );
}

/// Give thumbnail buttons readable hover and accessibility names without replacing their pixel art.
pub(crate) fn hint(
    wilds: Res<Wilds>,
    i18n: Res<I18n>,
    mut buttons: Query<(
        &ui::Action,
        &Interaction,
        &Name,
        Option<&mut bevy::a11y::AccessibilityNode>,
    )>,
    mut labels: Query<&mut Text, With<palette::HoverLabel>>,
) {
    let Some(run) = wilds.active.as_ref().filter(|r| r.palette_open) else {
        return;
    };
    let mut name = palette::title(&run.catalog.pieces[run.selected_piece], &i18n);
    for (action, interaction, title, accessible) in &mut buttons {
        if !matches!(action, ui::Action::Wild(Action::Piece(_))) {
            continue;
        }
        if let Some(mut accessible) = accessible {
            accessible.set_label(title.as_str());
        }
        if *interaction != Interaction::None {
            name = title.to_string();
        }
    }
    for mut label in &mut labels {
        **label = tr("wilds.selected_piece")
            .arg("name", name.clone())
            .render(&i18n);
    }
}
