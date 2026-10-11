use crate::{
    editor::{Action, Editor, Tool},
    i18n::{Message, tr},
    icons::Icon,
    maps::MapKind,
};

pub(super) fn for_action(action: Action) -> Option<Icon> {
    Some(match action {
        Action::Tool(Tool::Brush) => Icon::Brush,
        Action::Tool(Tool::Eraser) => Icon::Eraser,
        Action::Tool(Tool::Pick) => Icon::Pick,
        Action::Tool(Tool::Fill) => Icon::Fill,
        Action::Tool(Tool::Stamp) => Icon::Shop,
        Action::Undo => Icon::Undo,
        Action::Redo => Icon::Redo,
        Action::Visibility(_) => Icon::Eye,
        Action::Zoom(n) => {
            if n > 0 {
                Icon::ZoomIn
            } else {
                Icon::ZoomOut
            }
        }
        Action::Grid => Icon::Grid,
        Action::Guides => Icon::Guides,
        Action::Flip(0x8000_0000) => Icon::FlipH,
        Action::Flip(0x4000_0000) => Icon::FlipV,
        Action::Flip(_) => Icon::Swap,
        Action::Save | Action::SaveLeave => Icon::Save,
        Action::Reload => Icon::Reload,
        Action::Reset => Icon::Original,
        Action::Done | Action::Quit => Icon::Done,
        Action::Folder => Icon::Folder,
        Action::Map(MapKind::Town) => Icon::Town,
        Action::Map(_) => Icon::Shop,
        Action::Pan(n) => {
            if n.x < 0 {
                Icon::Left
            } else {
                Icon::Right
            }
        }
        Action::Palette(n) | Action::LayerPage(n) | Action::Stamp(n) => {
            if n < 0 {
                Icon::Left
            } else {
                Icon::Right
            }
        }
        Action::Discard => Icon::Discard,
        Action::Cancel => Icon::KeepEditing,
        Action::Layer(_) | Action::Tile(_) => return None,
    })
}

pub(super) fn shortcut(action: Action) -> Option<&'static str> {
    match action {
        Action::Tool(Tool::Brush) => Some("B"),
        Action::Tool(Tool::Eraser) => Some("E"),
        Action::Tool(Tool::Pick) => Some("I"),
        Action::Tool(Tool::Fill) => Some("F"),
        Action::Tool(Tool::Stamp) => Some("P"),
        _ => None,
    }
}

pub(super) fn hint(action: Action, editor: &Editor) -> Message {
    match action {
        Action::Tool(Tool::Brush) => tr("editor.hint.brush"),
        Action::Tool(Tool::Eraser) => tr("editor.hint.eraser"),
        Action::Tool(Tool::Pick) => tr("editor.hint.pick"),
        Action::Tool(Tool::Fill) => tr("editor.hint.fill"),
        Action::Tool(Tool::Stamp) => tr("editor.stamp_help"),
        Action::Undo => {
            if editor.doc().can_undo() {
                tr("editor.hint.undo")
            } else {
                tr("editor.hint.undo_empty")
            }
        }
        Action::Redo => {
            if editor.doc().can_redo() {
                tr("editor.hint.redo")
            } else {
                tr("editor.hint.redo_empty")
            }
        }
        Action::Visibility(layer) => tr(if editor.doc().map.layers[layer].visible {
            "editor.hint.hide"
        } else {
            "editor.hint.show"
        })
        .arg(
            "layer",
            crate::maps::layer_title(&editor.doc().map.layers[layer].name),
        ),
        Action::Layer(layer) => tr("editor.hint.layer").arg(
            "layer",
            crate::maps::layer_title(&editor.doc().map.layers[layer].name),
        ),
        Action::Tile(gid) => tr("editor.hint.tile").arg("tile", format!("{gid:03}")),
        Action::Zoom(n) => {
            if n > 0 {
                tr("editor.hint.zoom_in")
            } else {
                tr("editor.hint.zoom_out")
            }
        }
        Action::Grid => tr("editor.hint.grid"),
        Action::Guides => tr("editor.hint.guides"),
        Action::Flip(0x8000_0000) => tr("editor.hint.flip_h"),
        Action::Flip(0x4000_0000) => tr("editor.hint.flip_v"),
        Action::Flip(_) => tr("editor.hint.swap"),
        Action::Save => tr("editor.hint.save"),
        Action::SaveLeave => tr("editor.hint.save_leave"),
        Action::Reload => tr("editor.hint.reload"),
        Action::Reset => tr("editor.hint.original"),
        Action::Done | Action::Quit => tr("editor.hint.done"),
        Action::Folder => tr("editor.hint.files"),
        Action::Map(MapKind::Town) => tr("editor.hint.town"),
        Action::Map(_) => tr("editor.next_map"),
        Action::Pan(n) => {
            if n.x < 0 {
                tr("editor.hint.pan_left")
            } else {
                tr("editor.hint.pan_right")
            }
        }
        Action::Palette(n) | Action::LayerPage(n) | Action::Stamp(n) => {
            if n < 0 {
                tr("editor.hint.previous")
            } else {
                tr("editor.hint.next")
            }
        }
        Action::Discard => tr("editor.hint.discard"),
        Action::Cancel => tr("editor.hint.cancel"),
    }
    .into()
}
