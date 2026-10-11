use crate::{
    Session,
    fishing::{Action, FISH, Fishing, Panel, Stage},
    game::{Actor, Art},
    i18n::{I18n, Message, tr},
    ui::{self, Menu, Page},
};
use bevy::{
    prelude::*,
    sprite::{BorderRect, TextureSlicer},
    ui::widget::NodeImageMode,
};

#[derive(Component)]
pub(crate) struct Root;
#[derive(Component)]
pub(crate) struct ItemIcon;
#[derive(Component)]
pub(crate) struct Prompt;
#[derive(Component)]
pub(crate) enum Readout {
    Coins,
    Bait,
    Prompt,
    Notice,
    Title,
    Detail,
}
#[derive(Component)]
pub(crate) enum Fill {
    Landing,
    Tension,
    Bite,
}
#[derive(Component)]
pub(crate) enum Visual {
    Fish,
    Line,
    Float,
}

fn icon(commands: &mut Commands, parent: Entity, art: &Art, index: usize, size: f32) -> Entity {
    commands
        .spawn((
            ItemIcon,
            ImageNode::from_atlas_image(
                art.items.clone(),
                TextureAtlas {
                    layout: art.items_atlas.clone(),
                    index,
                },
            ),
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(parent),
        ))
        .id()
}

fn frame(commands: &mut Commands, parent: Entity, image: Handle<Image>, node: Node) -> Entity {
    let entity = commands.spawn((node, ChildOf(parent))).id();
    commands.spawn((
        ImageNode::new(image).with_mode(NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(16.0),
            ..default()
        })),
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ZIndex(-1),
        ChildOf(entity),
    ));
    entity
}

fn row(commands: &mut Commands, parent: Entity, gap: f32) -> Entity {
    commands
        .spawn((
            Node {
                column_gap: px(gap),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(parent),
        ))
        .id()
}

fn slot(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    index: usize,
    name: impl Into<Message>,
    amount: impl Into<Message>,
    owned: bool,
) {
    let cell = frame(
        commands,
        parent,
        art.slot.clone(),
        Node {
            width: px(114),
            height: px(114),
            padding: UiRect::all(px(9)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(3),
            ..default()
        },
    );
    let item = icon(commands, cell, art, index, 48.0);
    if !owned {
        commands.entity(item).insert(
            ImageNode::from_atlas_image(
                art.items.clone(),
                TextureAtlas {
                    layout: art.items_atlas.clone(),
                    index,
                },
            )
            .with_color(Color::srgba(0.55, 0.62, 0.57, 0.4)),
        );
    }
    ui::label(commands, cell, art, name, 14.0, ui::INK);
    ui::label(
        commands,
        cell,
        art,
        amount,
        15.0,
        if owned { ui::GOLD } else { ui::MUTED },
    );
}

fn shop_item(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    index: usize,
    title: impl Into<Message>,
    detail: impl Into<Message>,
    action: Action,
) {
    let button = ui::button(
        commands,
        parent,
        art,
        title,
        detail,
        ui::Action::Fishing(action),
        false,
    );
    commands.entity(button).insert(Node {
        width: percent(100),
        height: px(70),
        min_height: px(70),
        padding: UiRect {
            left: px(80),
            right: px(12),
            top: px(8),
            bottom: px(8),
        },
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        row_gap: px(6),
        border: UiRect::bottom(px(3)),
        ..default()
    });
    let item = icon(commands, button, art, index, 48.0);
    commands.entity(item).insert(Node {
        position_type: PositionType::Absolute,
        left: px(16),
        top: px(9),
        width: px(48),
        height: px(48),
        ..default()
    });
}

fn meter(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    title: impl Into<Message>,
    fill: Fill,
) {
    ui::label(commands, parent, art, title, 15.0, ui::INK);
    let track = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(15),
                border: UiRect::all(px(3)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(74, 93, 80)),
            BorderColor::all(ui::MUTED),
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        fill,
        Node {
            width: percent(0),
            height: percent(100),
            ..default()
        },
        BackgroundColor(ui::GREEN),
        ChildOf(track),
    ));
}

pub fn render(
    i18n: Res<I18n>,
    mut commands: Commands,
    menu: Res<Menu>,
    art: Res<Art>,
    mut fishing: ResMut<Fishing>,
    roots: Query<Entity, With<Root>>,
) {
    if !fishing.dirty && !menu.is_changed() && !i18n.is_changed() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    fishing.dirty = false;
    if menu.page != Page::Playing {
        return;
    }
    let root = commands
        .spawn((
            Root,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            GlobalZIndex(10),
        ))
        .id();
    let wallet = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(24),
                top: px(24),
                padding: UiRect::axes(px(10), px(4)),
                column_gap: px(8),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(ui::PANEL),
            ChildOf(root),
        ))
        .id();
    for (index, readout) in [(3, Readout::Coins), (2, Readout::Bait)] {
        icon(&mut commands, wallet, &art, index, 24.0);
        let text = ui::label(&mut commands, wallet, &art, "", 18.0, ui::CREAM);
        commands.entity(text).insert(readout);
    }
    icon(&mut commands, wallet, &art, 11, 24.0);
    ui::label(&mut commands, wallet, &art, "I", 18.0, ui::CREAM);
    let notice = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(68),
                left: px(24),
                max_width: px(590),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    let text = ui::label(&mut commands, notice, &art, "", 18.0, ui::CREAM);
    commands.entity(text).insert((
        Readout::Notice,
        TextShadow {
            offset: Vec2::splat(2.0),
            color: ui::PANEL,
        },
    ));
    let prompt = commands
        .spawn((
            Prompt,
            Node {
                position_type: PositionType::Absolute,
                top: px(108),
                left: px(24),
                padding: UiRect::all(px(10)),
                ..default()
            },
            BackgroundColor(ui::PANEL),
            ChildOf(root),
        ))
        .id();
    let text = ui::label(&mut commands, prompt, &art, "", 18.0, ui::CREAM);
    commands.entity(text).insert(Readout::Prompt);
    if !fishing.modal() {
        return;
    }
    let panel = frame(
        &mut commands,
        root,
        art.panel.clone(),
        Node {
            position_type: PositionType::Absolute,
            left: px(24),
            top: px(112),
            width: px(if fishing.panel == Panel::None {
                376
            } else {
                520
            }),
            padding: UiRect::all(px(20)),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        },
    );
    match fishing.panel {
        Panel::Bag => {
            ui::label(&mut commands, panel, &art, tr("fishing.bag"), 30.0, ui::INK);
            ui::label(
                &mut commands,
                panel,
                &art,
                tr("fishing.supplies"),
                15.0,
                ui::MUTED,
            );
            let equipment = row(&mut commands, panel, 8.0);
            for (index, name, count, owned) in [
                (
                    0,
                    tr("fishing.rod"),
                    if fishing.progress.rod {
                        tr("fishing.equipped")
                    } else {
                        tr("fishing.not_owned")
                    },
                    fishing.progress.rod,
                ),
                (
                    1,
                    tr("fishing.hook"),
                    if fishing.progress.hook {
                        tr("fishing.equipped")
                    } else {
                        tr("fishing.not_owned")
                    },
                    fishing.progress.hook,
                ),
                (
                    2,
                    tr("fishing.bait"),
                    Message::from(format!("x{}", fishing.progress.bait)),
                    fishing.progress.bait > 0,
                ),
                (
                    3,
                    tr("fishing.coins"),
                    Message::from(format!("x{}", fishing.progress.coins)),
                    fishing.progress.coins > 0,
                ),
            ] {
                slot(&mut commands, equipment, &art, index, name, &count, owned);
            }
            ui::label(
                &mut commands,
                panel,
                &art,
                tr("fishing.catch"),
                15.0,
                ui::MUTED,
            );
            let catches = row(&mut commands, panel, 8.0);
            for (index, (name, price)) in FISH.iter().enumerate() {
                let count = fishing.progress.catches[index];
                slot(
                    &mut commands,
                    catches,
                    &art,
                    4 + index,
                    tr(name),
                    &format!("x{count} / {price}c"),
                    count > 0,
                );
            }
            ui::label(
                &mut commands,
                panel,
                &art,
                tr("fishing.catch_value").arg("coins", fishing.progress.value().to_string()),
                18.0,
                ui::INK,
            );
        }
        Panel::Shop => {
            ui::label(
                &mut commands,
                panel,
                &art,
                tr("fishing.counter"),
                27.0,
                ui::INK,
            );
            ui::label(
                &mut commands,
                panel,
                &art,
                tr("fishing.counter_hint"),
                18.0,
                ui::MUTED,
            );
            shop_item(
                &mut commands,
                panel,
                &art,
                0,
                tr("fishing.buy_rod"),
                if fishing.progress.rod {
                    tr("fishing.owned_hint")
                } else {
                    tr("fishing.rod_price")
                },
                Action::Rod,
            );
            shop_item(
                &mut commands,
                panel,
                &art,
                1,
                tr("fishing.buy_hook"),
                if fishing.progress.hook {
                    tr("fishing.hook_owned")
                } else {
                    tr("fishing.hook_price")
                },
                Action::Hook,
            );
            shop_item(
                &mut commands,
                panel,
                &art,
                2,
                tr("fishing.buy_bait"),
                tr("fishing.bait_price"),
                Action::Bait,
            );
            shop_item(
                &mut commands,
                panel,
                &art,
                11,
                tr("fishing.sell"),
                tr("fishing.sell_value")
                    .arg(
                        "fish",
                        fishing.progress.catches.iter().sum::<u32>().to_string(),
                    )
                    .arg("coins", fishing.progress.value().to_string()),
                Action::Sell,
            );
        }
        Panel::None => {
            let title = ui::label(&mut commands, panel, &art, "", 24.0, ui::INK);
            commands.entity(title).insert(Readout::Title);
            let view = commands
                .spawn((
                    ImageNode::new(art.water.clone()),
                    Node {
                        width: px(320),
                        height: px(140),
                        align_self: AlignSelf::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    ChildOf(panel),
                ))
                .id();
            let index = match &fishing.stage {
                Stage::Reeling(fight) => 4 + fight.fish,
                Stage::Result {
                    fish: Some(fish), ..
                } => 4 + *fish,
                _ => 12,
            };
            commands.spawn((
                Visual::Line,
                ImageNode::solid_color(ui::CREAM),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(3),
                    ..default()
                },
                ChildOf(view),
            ));
            let fish = icon(&mut commands, view, &art, index, 64.0);
            commands.entity(fish).insert((
                Visual::Fish,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(64),
                    height: px(64),
                    ..default()
                },
            ));
            let bob = icon(
                &mut commands,
                view,
                &art,
                if matches!(fishing.stage, Stage::Bite(_)) {
                    14
                } else {
                    8
                },
                48.0,
            );
            commands.entity(bob).insert((
                Visual::Float,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(48),
                    height: px(48),
                    ..default()
                },
            ));
            if let Stage::Result { fish: Some(_), .. } = fishing.stage {
                for left in [32, 236] {
                    let sparkle = icon(&mut commands, view, &art, 13, 48.0);
                    commands.entity(sparkle).insert(Node {
                        position_type: PositionType::Absolute,
                        left: px(left),
                        top: px(40),
                        width: px(48),
                        height: px(48),
                        ..default()
                    });
                }
            }
            let detail = ui::label(&mut commands, panel, &art, "", 18.0, ui::INK);
            commands.entity(detail).insert(Readout::Detail);
            match fishing.stage {
                Stage::Reeling(_) => {
                    meter(
                        &mut commands,
                        panel,
                        &art,
                        tr("fishing.landing"),
                        Fill::Landing,
                    );
                    meter(
                        &mut commands,
                        panel,
                        &art,
                        tr("fishing.tension"),
                        Fill::Tension,
                    );
                }
                Stage::Bite(_) => meter(
                    &mut commands,
                    panel,
                    &art,
                    tr("fishing.set_hook"),
                    Fill::Bite,
                ),
                _ => {}
            }
        }
    }
    let close = ui::button(
        &mut commands,
        panel,
        &art,
        if fishing.active() {
            tr("fishing.put_away")
        } else {
            tr("fishing.back")
        },
        "",
        ui::Action::Fishing(Action::Close),
        false,
    );
    commands.entity(close).insert(Node {
        width: percent(100),
        height: px(40),
        min_height: px(40),
        padding: UiRect::axes(px(12), px(6)),
        align_items: AlignItems::Center,
        ..default()
    });
}

pub fn refresh(
    maps: Res<crate::maps::Maps>,
    i18n: Res<I18n>,
    time: Res<Time>,
    fishing: Res<Fishing>,
    session: Res<Session>,
    actors: Query<&Actor>,
    mut texts: Query<(&Readout, &mut Text)>,
    mut fills: Query<(&Fill, &mut Node, &mut BackgroundColor), (Without<Visual>, Without<Prompt>)>,
    mut visuals: Query<(&Visual, &mut Node), (Without<Fill>, Without<Prompt>)>,
    mut prompts: Query<&mut Node, (With<Prompt>, Without<Fill>, Without<Visual>)>,
) {
    let interaction = actors
        .iter()
        .find(|a| Some(a.player.id) == session.you)
        .and_then(|a| {
            maps.by_id(&a.player.map)
                .and_then(|m| m.interaction(a.position.x, a.position.y))
        });
    let prompt = if fishing.modal() {
        Message::default()
    } else {
        match interaction.map(|o| o.kind.as_str()) {
            Some("portal") => tr(if fishing.indoors {
                "fishing.prompt.leave"
            } else {
                "fishing.prompt.enter"
            }),
            Some("shop") => tr("fishing.prompt.shop"),
            Some("fishing") => tr("fishing.prompt.cast"),
            _ => Message::default(),
        }
    };
    for mut node in &mut prompts {
        node.display = if prompt.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
    for (part, mut text) in &mut texts {
        let value: Message = match part {
            Readout::Coins => fishing.progress.coins.to_string().into(),
            Readout::Bait => fishing.progress.bait.to_string().into(),
            Readout::Prompt => prompt.clone(),
            Readout::Notice => {
                if !fishing.save_error.is_empty() {
                    fishing.save_error.clone()
                } else if fishing.notice_time > 0.0 {
                    fishing.notice.clone()
                } else {
                    Message::default()
                }
            }
            Readout::Title => match &fishing.stage {
                Stage::Waiting(_) => tr("fishing.watch"),
                Stage::Bite(_) => tr("fishing.bite"),
                Stage::Reeling(fight) => if fight.pull > 0.8 {
                    tr("fishing.dash")
                } else {
                    tr("fishing.reel")
                }
                .into(),
                Stage::Result { fish, .. } => if fish.is_some() {
                    tr("fishing.won")
                } else {
                    tr("fishing.lost")
                }
                .into(),
                Stage::Idle => Message::default(),
            },
            Readout::Detail => match &fishing.stage {
                Stage::Waiting(_) => tr("fishing.waiting_hint"),
                Stage::Bite(_) => tr("fishing.bite_hint"),
                Stage::Reeling(_) => tr("fishing.reel_hint"),
                Stage::Result { message, .. } => message.clone(),
                Stage::Idle => Message::default(),
            },
        };
        let value = value.render(&i18n);
        if **text != value {
            **text = value;
        }
    }
    for (fill, mut node, mut color) in &mut fills {
        let value = match (&fishing.stage, fill) {
            (Stage::Reeling(fight), Fill::Landing) => {
                color.0 = Color::srgb_u8(92, 153, 115);
                fight.landed
            }
            (Stage::Reeling(fight), Fill::Tension) => {
                color.0 = if fight.tension > 0.75 {
                    Color::srgb_u8(199, 73, 65)
                } else {
                    Color::srgb_u8(205, 159, 76)
                };
                fight.tension
            }
            (Stage::Bite(left), Fill::Bite) => left / 1.5,
            _ => 0.0,
        };
        node.width = percent(value.clamp(0.0, 1.0) * 100.0);
    }
    let t = time.elapsed_secs();
    let (fish_x, fish_y) = match &fishing.stage {
        Stage::Reeling(fight) => (
            36.0 + (1.0 - fight.landed) * 170.0 + (t * 9.0).sin() * fight.pull * 9.0,
            32.0 + (t * 4.0).sin() * (5.0 + fight.pull * 14.0),
        ),
        Stage::Result { fish: Some(_), .. } => (128.0, 26.0 + (t * 3.0).sin() * 5.0),
        Stage::Result { .. } => (248.0, 44.0),
        _ => (192.0 + (t * 1.5).sin() * 22.0, 44.0 + (t * 2.0).sin() * 8.0),
    };
    for (visual, mut node) in &mut visuals {
        match visual {
            Visual::Fish => {
                node.left = px(fish_x.round());
                node.top = px(fish_y.round());
            }
            Visual::Line => {
                node.left = px((fish_x + 6.0).round());
                node.top = px(5);
                node.height = px((fish_y + 32.0).round());
                node.display = if matches!(fishing.stage, Stage::Reeling(_)) {
                    Display::Flex
                } else {
                    Display::None
                };
            }
            Visual::Float => {
                node.left = px(64);
                node.top = px(2.0
                    + (t * if matches!(fishing.stage, Stage::Bite(_)) {
                        15.0
                    } else {
                        2.0
                    })
                    .sin()
                        * 3.0);
                node.display = if matches!(fishing.stage, Stage::Waiting(_) | Stage::Bite(_)) {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}
