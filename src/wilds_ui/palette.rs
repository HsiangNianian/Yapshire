use super::*;
use crate::{maps::Maps, wilds::Run};
use yapshire_shared::construction::{Layer, Piece};

#[derive(Component)]
pub(crate) struct Scroll(pub usize);
#[derive(Component)]
pub(crate) struct HoverLabel;

fn compact_action(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    key: &'static str,
    icon: Icon,
    act: Action,
) {
    let id = action(commands, parent, art, key, icon, act);
    commands.entity(id).insert(Node {
        padding: UiRect::all(px(4)),
        min_height: px(32),
        min_width: px(40),
        column_gap: px(8),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        flex_shrink: 0.0,
        ..default()
    });
}

pub(super) fn title(piece: &Piece, i18n: &I18n) -> String {
    let name = piece
        .id
        .strip_prefix("yapshire:")
        .unwrap_or(&piece.id)
        .split('/')
        .next()
        .unwrap();
    let key = format!("tiles.{name}");
    let text = i18n.text(&key);
    if text == key {
        piece.label.clone()
    } else {
        text.into()
    }
}

pub(super) fn image(
    commands: &mut Commands,
    button: Entity,
    piece: &Piece,
    maps: &Maps,
    assets: &AssetServer,
    size: f32,
) {
    let x = (piece.index % piece.columns * 16) as f32;
    let y = (piece.index / piece.columns * 16) as f32;
    let image = commands
        .spawn((
            ImageNode {
                image: assets.load(maps.image_path(&piece.image)),
                rect: Some(Rect::new(x, y, x + 16.0, y + 16.0)),
                ..default()
            },
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                ..default()
            },
            FocusPolicy::Pass,
        ))
        .id();
    commands.entity(button).insert_children(0, &[image]);
}

pub(super) fn layer_key(layer: Layer) -> &'static str {
    match layer {
        Layer::Wall => "wilds.wall_layer",
        Layer::Solid => "wilds.solid_layer",
        Layer::Decor => "wilds.decor_layer",
    }
}

pub(super) fn render(
    commands: &mut Commands,
    root: Entity,
    run: &Run,
    art: &Art,
    maps: &Maps,
    assets: &AssetServer,
    i18n: &I18n,
    scroll: Vec2,
) {
    let frame = panel(
        commands,
        root,
        Node {
            position_type: PositionType::Absolute,
            right: px(24),
            top: px(128),
            bottom: px(158),
            width: px(392),
            padding: UiRect::all(px(12)),
            border: UiRect::all(px(2)),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        },
    );
    let header = commands
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(frame),
        ))
        .id();
    ui::label(commands, header, art, tr("wilds.catalog"), 22.0, ui::INK);
    compact_action(
        commands,
        header,
        art,
        "wilds.close_palette",
        Icon::Done,
        Action::Palette,
    );
    let group = &run.catalog.groups[run.palette_group];
    let group_key = match group.as_str() {
        "terrain/ground.tsj" => "wilds.group_ground",
        "terrain/water.tsj" => "wilds.group_water",
        "buildings/tackle.tsj" => "wilds.group_shop",
        "objects/harbor.tsj" => "wilds.group_harbor",
        "terrain/natural.tsj" => "wilds.group_natural",
        "buildings/modules.tsj" => "wilds.group_building",
        "nature/garden.tsj" => "wilds.group_nature",
        "objects/street.tsj" => "wilds.group_garden",
        _ => "",
    };
    let groups = commands
        .spawn((
            Node {
                column_gap: px(8),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(frame),
        ))
        .id();
    compact_action(commands, groups, art, "", Icon::Left, Action::Group(-1));
    ui::label(
        commands,
        groups,
        art,
        if group_key.is_empty() {
            group.clone().into()
        } else {
            tr(group_key)
        },
        18.0,
        ui::INK,
    );
    compact_action(commands, groups, art, "", Icon::Right, Action::Group(1));
    let grid = panel(
        commands,
        frame,
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    );
    commands
        .entity(grid)
        .insert((Scroll(run.palette_group), ScrollPosition(scroll)));
    let grid = ui::scroll_content(commands, grid, 4.0, false);
    commands.entity(grid).insert(Node {
        width: percent(100),
        flex_wrap: FlexWrap::Wrap,
        align_content: AlignContent::FlexStart,
        align_items: AlignItems::FlexStart,
        column_gap: px(4),
        row_gap: px(4),
        flex_shrink: 0.0,
        ..default()
    });
    for (index, piece) in run
        .catalog
        .pieces
        .iter()
        .enumerate()
        .filter(|(_, p)| &p.source == group)
    {
        let button = ui::button(
            commands,
            grid,
            art,
            "",
            "",
            ui::Action::Wild(Action::Piece(index)),
            run.using_catalog && run.selected_piece == index,
        );
        let name = title(piece, i18n);
        commands.entity(button).insert((
            Node {
                width: px(40),
                height: px(40),
                padding: UiRect::all(px(4)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Name::new(name),
        ));
        image(commands, button, piece, maps, assets, 32.0);
    }
    let selected = &run.catalog.pieces[run.selected_piece];
    let label = ui::label(
        commands,
        frame,
        art,
        tr("wilds.selected_piece")
            .arg("name", title(selected, i18n))
            .render(i18n),
        16.0,
        ui::GOLD,
    );
    commands.entity(label).insert(HoverLabel);
    ui::label(
        commands,
        frame,
        art,
        tr("wilds.catalog_hint"),
        14.0,
        ui::MUTED,
    );
}
