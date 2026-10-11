//! Local, offline map editing. Drafts never change a running multiplayer room.
use crate::{
    game,
    i18n::{Message, tr},
    maps::{Map, MapKind, Maps},
    paths::atomic_write,
    ui::{Menu, Page},
};
use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    window::{ClosingWindow, WindowCloseRequested},
};
use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

pub(crate) const CANVAS: Vec2 = Vec2::new(24.0, 154.0);
pub(crate) const CANVAS_SIZE: Vec2 = Vec2::new(960.0, 544.0);
const HISTORY_LIMIT: usize = 100;

pub(crate) struct Document {
    pub map: Map,
    saved: Map,
    original: Map,
    disk: Option<Vec<u8>>,
    disk_path: Option<PathBuf>,
    undo: VecDeque<Map>,
    redo: Vec<Map>,
    stroke: Option<Map>,
}

impl Document {
    fn new(map: Map) -> Self {
        Self {
            saved: map.clone(),
            original: map.clone(),
            map,
            disk: None,
            disk_path: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            stroke: None,
        }
    }

    pub fn dirty(&self) -> bool {
        self.map != self.saved
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty() || self.stroke.is_some()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn begin(&mut self) {
        if self.stroke.is_none() {
            self.stroke = Some(self.map.clone());
        }
    }

    fn finish(&mut self) {
        if let Some(before) = self.stroke.take() {
            if before != self.map {
                self.undo.push_back(before);
                if self.undo.len() > HISTORY_LIMIT {
                    self.undo.pop_front();
                }
                self.redo.clear();
            }
        }
    }

    fn undo(&mut self) {
        self.finish();
        if let Some(before) = self.undo.pop_back() {
            self.redo.push(std::mem::replace(&mut self.map, before));
        }
    }

    fn redo(&mut self) {
        self.finish();
        if let Some(after) = self.redo.pop() {
            self.undo.push_back(std::mem::replace(&mut self.map, after));
        }
    }

    fn paint(&mut self, layer: usize, point: UVec2, gid: u32) {
        if point.x >= self.map.width
            || point.y >= self.map.height
            || self.map.layers[layer].kind != "tilelayer"
        {
            return;
        }
        let index = (point.y * self.map.width + point.x) as usize;
        if self.map.layers[layer].data[index] != gid {
            self.begin();
            self.map.layers[layer].data[index] = gid;
        }
    }

    fn fill(&mut self, layer: usize, point: UVec2, gid: u32) {
        if self.map.layers[layer].kind != "tilelayer" {
            return;
        }
        let width = self.map.width as usize;
        let start = point.y as usize * width + point.x as usize;
        let old = self.map.layers[layer].data[start];
        if old == gid {
            return;
        }
        self.begin();
        let data = &mut self.map.layers[layer].data;
        let mut stack = vec![start];
        while let Some(index) = stack.pop() {
            if data[index] != old {
                continue;
            }
            data[index] = gid;
            if index % width > 0 {
                stack.push(index - 1);
            }
            if index % width + 1 < width {
                stack.push(index + 1);
            }
            if index >= width {
                stack.push(index - width);
            }
            if index + width < data.len() {
                stack.push(index + width);
            }
        }
    }

    fn connect_terrain(&mut self, layer: usize, maps: &Maps, kind: MapKind) {
        let content = &maps.world.content;
        let map = &self.map;
        let mut changes = Vec::new();
        for (index, &gid) in map.layers[layer].data.iter().enumerate() {
            let Some((_, _, tile)) = content.resolve(map, maps.filename(kind), gid) else {
                continue;
            };
            let Some(terrain) = tile.terrain() else {
                continue;
            };
            let x = index as i32 % map.width as i32;
            let y = index as i32 / map.width as i32;
            let mut mask = 0;
            for (bit, dx, dy) in [(1, 0, -1), (2, 1, 0), (4, 0, 1), (8, -1, 0)] {
                let (x, y) = (x + dx, y + dy);
                if x >= 0 && y >= 0 && x < map.width as i32 && y < map.height as i32 {
                    let other = map.layers[layer].data[(y as u32 * map.width + x as u32) as usize];
                    if content
                        .resolve(map, maps.filename(kind), other)
                        .is_some_and(|(_, _, t)| t.terrain() == Some(terrain))
                    {
                        mask |= bit;
                    }
                }
            }
            let new = content
                .gid(&content.manifest.terrains[terrain][mask])
                .unwrap();
            if new != gid {
                changes.push((UVec2::new(x as u32, y as u32), new));
            }
        }
        for (point, gid) in changes {
            self.paint(layer, point, gid);
        }
    }

    fn stamp(
        &mut self,
        layer: usize,
        point: UVec2,
        prefab: &yapshire_shared::content::Prefab,
        maps: &Maps,
    ) {
        if point.x + prefab.width > self.map.width || point.y + prefab.height > self.map.height {
            return;
        }
        for (index, id) in prefab.tiles.iter().enumerate() {
            if let Some(gid) = maps.world.content.gid(id) {
                self.paint(
                    layer,
                    point + UVec2::new(index as u32 % prefab.width, index as u32 / prefab.width),
                    gid,
                );
            }
        }
    }

    fn discard(&mut self) {
        self.map = self.saved.clone();
        self.undo.clear();
        self.redo.clear();
        self.stroke = None;
    }

    fn save(&mut self, directory: &Path, kind: MapKind, maps: &Maps) -> io::Result<()> {
        self.finish();
        let mut prepared = self.map.clone();
        maps.prepare_map(kind, &mut prepared)?;
        let path = directory.join(maps.filename(kind));
        let previous_path = self.disk_path.as_deref().unwrap_or(&path);
        if read_optional(previous_path)? != self.disk || (previous_path != path && path.exists()) {
            return Err(io::Error::other(
                "File changed outside the game. Reload before saving.",
            ));
        }
        let mut bytes = serde_json::to_vec_pretty(&prepared).map_err(io::Error::other)?;
        bytes.push(b'\n');
        if let Some(previous) = &self.disk {
            atomic_write(&previous_path.with_extension("tmj.bak"), previous)?;
        }
        atomic_write(&path, &bytes)?;
        self.disk = Some(bytes);
        self.disk_path = Some(path);
        self.map = prepared.clone();
        self.saved = prepared;
        Ok(())
    }

    fn reload(&mut self, directory: &Path, kind: MapKind, maps: &Maps) -> io::Result<()> {
        let path = directory.join(maps.filename(kind));
        let legacy = directory.join(Path::new(maps.filename(kind)).file_name().unwrap());
        let path = if !path.exists() && legacy.exists() {
            legacy
        } else {
            path
        };
        let bytes = read_optional(&path)?;
        let mut map = match &bytes {
            Some(bytes) => serde_json::from_slice::<Map>(bytes).map_err(io::Error::other)?,
            None => self.original.clone(),
        };
        if map.tilesets.len() == 1 && map.tilesets[0].source == "harbor.tsj" {
            let folder = path.parent().unwrap();
            if folder.join("harbor.tsj").exists() || folder.join("harbor.png").exists() {
                yapshire_shared::validate_tileset(folder)?;
            }
        }
        maps.prepare_map(kind, &mut map)?;
        self.disk_path = Some(path);
        self.saved = map;
        self.disk = bytes;
        self.discard();
        Ok(())
    }
}

fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match yapshire_shared::content::read_limited(path, yapshire_shared::MAX_MAP_BYTES) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Brush,
    Eraser,
    Pick,
    Fill,
    Stamp,
}

#[derive(Clone, Copy)]
pub(crate) enum Dialog {
    Leave(bool),
    Reload,
}

#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) enum Action {
    Map(MapKind),
    Layer(usize),
    LayerPage(i32),
    Visibility(usize),
    Tile(u32),
    Palette(i32),
    Stamp(i32),
    Tool(Tool),
    Flip(u32),
    Undo,
    Redo,
    Zoom(i32),
    Pan(IVec2),
    Grid,
    Guides,
    Save,
    Folder,
    Reload,
    Reset,
    Done,
    Quit,
    SaveLeave,
    Discard,
    Cancel,
}

#[derive(Resource)]
pub(crate) struct Editor {
    pub docs: Vec<Document>,
    directory: Result<PathBuf, String>,
    pub kind: MapKind,
    pub layer: usize,
    pub layer_page: usize,
    pub tile: u32,
    pub tool: Tool,
    pub palette: usize,
    pub stamp: usize,
    pub zoom: u32,
    pub offset: UVec2,
    pub grid: bool,
    pub guides: bool,
    pub dialog: Option<Dialog>,
    pub status: Message,
    pub hover: Option<UVec2>,
    pub ui_dirty: bool,
    pub canvas_dirty: bool,
    was_open: bool,
    drawing: Option<(UVec2, bool)>,
    panning: Option<(Vec2, UVec2)>,
}

impl Editor {
    pub fn load(maps: &mut Maps) -> Self {
        let directory = std::env::var_os("YAPSHIRE_MAP_DIR")
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(|| {
                crate::paths::data_dir().map(|p| {
                    if maps.world.content.manifest.id == "yapshire" {
                        p.join("maps")
                    } else {
                        p.join("packs").join(&maps.world.content.manifest.id)
                    }
                })
            })
            .map_err(|e| e.to_string());
        Self::from_directory(maps, directory)
    }

    fn from_directory(maps: &mut Maps, directory: Result<PathBuf, String>) -> Self {
        let mut docs: Vec<_> = maps
            .kinds()
            .map(|kind| Document::new(maps.get(kind).clone()))
            .collect();
        let mut status = tr("editor.status.ready");
        for kind in maps.kinds() {
            let doc = &mut docs[kind.index()];
            let result = directory
                .as_ref()
                .map_err(|e| io::Error::other(e.clone()))
                .and_then(|dir| doc.reload(dir, kind, maps));
            match result {
                Ok(()) => maps.replace(kind, doc.map.clone()),
                Err(error) => {
                    // Keep an invalid file available for backup after an explicit repair/save.
                    if let Ok(dir) = &directory {
                        let path = dir.join(maps.filename(kind));
                        let legacy = dir.join(Path::new(maps.filename(kind)).file_name().unwrap());
                        let path = if !path.exists() && legacy.exists() {
                            legacy
                        } else {
                            path
                        };
                        doc.disk = read_optional(&path).ok().flatten();
                        doc.disk_path = Some(path);
                    }
                    warn!("Cannot load local {}: {error}", maps.filename(kind));
                    status = tr("editor.status.unavailable")
                        .arg("map", maps.title(kind))
                        .arg("error", error.to_string());
                }
            }
        }
        Self {
            docs,
            directory,
            kind: MapKind::Town,
            layer: maps
                .get(MapKind(0))
                .layers
                .iter()
                .rposition(|l| l.kind == "tilelayer")
                .unwrap_or(0),
            layer_page: 0,
            tile: 1,
            tool: Tool::Brush,
            palette: 0,
            stamp: 0,
            zoom: 2,
            offset: UVec2::new(48, 0),
            grid: true,
            guides: true,
            dialog: None,
            status,
            hover: None,
            ui_dirty: true,
            canvas_dirty: true,
            was_open: false,
            drawing: None,
            panning: None,
        }
    }

    pub fn doc(&self) -> &Document {
        &self.docs[self.kind.index()]
    }
    fn doc_mut(&mut self) -> &mut Document {
        &mut self.docs[self.kind.index()]
    }
    pub fn dirty(&self) -> bool {
        self.docs.iter().any(Document::dirty)
    }
    pub fn cell_size(&self) -> f32 {
        16.0 * self.zoom as f32
    }
    pub fn directory_label(&self) -> String {
        match &self.directory {
            Ok(path) => path.display().to_string(),
            Err(e) => e.clone(),
        }
    }

    pub fn cell_at(&self, ui: Vec2) -> Option<UVec2> {
        let local = ui - CANVAS;
        if local.cmplt(Vec2::ZERO).any() || local.cmpge(CANVAS_SIZE).any() {
            return None;
        }
        let cell = (local / self.cell_size()).floor().as_uvec2() + self.offset;
        (cell.x < self.doc().map.width && cell.y < self.doc().map.height).then_some(cell)
    }

    fn clamp_offset(&mut self) {
        let visible = (CANVAS_SIZE / self.cell_size()).floor().as_uvec2();
        self.offset = self
            .offset
            .min(UVec2::new(self.doc().map.width, self.doc().map.height).saturating_sub(visible));
    }

    fn finish(&mut self) {
        self.doc_mut().finish();
        self.drawing = None;
        self.panning = None;
    }

    fn save(&mut self, kind: MapKind, maps: &mut Maps) -> Result<(), String> {
        let dir = self.directory.as_ref().map_err(Clone::clone)?;
        let mut candidate = maps.world.maps.clone();
        candidate.insert(maps.info(kind).id.clone(), self.docs[kind.0].map.clone());
        yapshire_shared::World::from_maps(maps.world.content.clone(), candidate)
            .map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let files = std::iter::once("pack.json")
            .chain(
                maps.world
                    .content
                    .manifest
                    .tilesets
                    .iter()
                    .map(String::as_str),
            )
            .chain(maps.world.content.images.keys().map(String::as_str));
        for name in files {
            let target = dir.join(name);
            let bytes = std::fs::read(maps.pack_path.join(name)).map_err(|e| e.to_string())?;
            if target.exists() {
                let same = std::fs::read(&target).map_err(|e| e.to_string())?;
                let equal = if name.ends_with(".json") || name.ends_with(".tsj") {
                    serde_json::from_slice::<serde_json::Value>(&same).ok()
                        == serde_json::from_slice::<serde_json::Value>(&bytes).ok()
                } else {
                    same == bytes
                };
                if !equal {
                    return Err(format!(
                        "Content differs on disk: {}. Install matching assets before saving.",
                        target.display()
                    ));
                }
            } else {
                atomic_write(&target, &bytes).map_err(|e| e.to_string())?;
            }
        }
        for other in maps.kinds().filter(|k| *k != kind) {
            if !dir.join(maps.filename(other)).exists() {
                // Export saved baselines so the folder is a complete pack, without saving other drafts.
                let doc = &mut self.docs[other.0];
                let mut baseline = Document::new(doc.saved.clone());
                baseline.disk = doc.disk.clone();
                baseline.disk_path = doc.disk_path.clone();
                baseline.save(dir, other, maps).map_err(|e| e.to_string())?;
                doc.disk = baseline.disk;
                doc.disk_path = baseline.disk_path;
            }
        }
        let doc = &mut self.docs[kind.index()];
        doc.save(dir, kind, maps).map_err(|e| e.to_string())?;
        maps.replace(kind, doc.map.clone());
        self.status = tr("editor.status.saved").arg("map", maps.title(kind));
        Ok(())
    }

    /// Returns whether to quit the app (true) or return to its menu (false).
    fn act(&mut self, action: Action, maps: &mut Maps) -> Option<bool> {
        self.finish();
        self.ui_dirty = true;
        self.canvas_dirty = true;
        if self.dialog.is_some()
            && !matches!(action, Action::SaveLeave | Action::Discard | Action::Cancel)
        {
            return None;
        }
        match action {
            Action::Map(kind) => {
                self.kind = kind;
                self.layer_page = 0;
                self.layer = self.layer.min(self.doc().map.layers.len() - 1);
                self.status = tr("editor.status.editing").arg("map", maps.title(kind));
                self.offset = if kind == MapKind::Town {
                    UVec2::new(48, 0)
                } else {
                    UVec2::ZERO
                };
            }
            Action::LayerPage(delta) => {
                self.layer_page = (self.layer_page as i32 + delta)
                    .rem_euclid(self.doc().map.layers.len().div_ceil(5) as i32)
                    as usize;
            }
            Action::Layer(layer) => {
                self.layer = layer;
                self.status = tr("editor.status.layer").arg(
                    "layer",
                    crate::maps::layer_title(&self.doc().map.layers[layer].name),
                );
                if self.doc().map.layers[layer].kind != "tilelayer" {
                    self.status = tr("editor.tiled_layer");
                }
            }
            Action::Visibility(layer) => {
                let doc = self.doc_mut();
                doc.begin();
                doc.map.layers[layer].visible = !doc.map.layers[layer].visible;
                doc.finish();
            }
            Action::Tile(gid) => {
                self.tile = gid;
                self.tool = Tool::Brush;
            }
            Action::Palette(delta) => {
                let pages = maps.tile_count().div_ceil(128) as i32;
                self.palette = (self.palette as i32 + delta).rem_euclid(pages) as usize;
            }
            Action::Stamp(delta) => {
                let count = maps.world.content.manifest.prefabs.len();
                if count != 0 {
                    self.stamp = (self.stamp as i32 + delta).rem_euclid(count as i32) as usize;
                    self.tool = Tool::Stamp;
                }
            }
            Action::Tool(tool) => self.tool = tool,
            Action::Flip(flag) => self.tile ^= flag,
            Action::Undo => self.doc_mut().undo(),
            Action::Redo => self.doc_mut().redo(),
            Action::Zoom(delta) => self.zoom = (self.zoom as i32 + delta).clamp(1, 3) as u32,
            Action::Pan(delta) => {
                self.offset = (self.offset.as_ivec2() + delta).max(IVec2::ZERO).as_uvec2()
            }
            Action::Grid => self.grid = !self.grid,
            Action::Guides => self.guides = !self.guides,
            Action::Save => {
                if let Err(error) = self.save(self.kind, maps) {
                    self.status = tr("editor.status.save_failed").arg("error", error);
                }
            }
            Action::Folder => {
                let result = self
                    .directory
                    .as_ref()
                    .map_err(Clone::clone)
                    .and_then(|dir| {
                        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                        let program = if cfg!(target_os = "macos") {
                            "open"
                        } else if cfg!(target_os = "windows") {
                            "explorer"
                        } else {
                            "xdg-open"
                        };
                        std::process::Command::new(program)
                            .arg(dir)
                            .spawn()
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    });
                self.status = match result {
                    Ok(()) => tr("editor.status.files").arg("path", self.directory_label()),
                    Err(error) => tr("editor.status.open_failed")
                        .arg("path", self.directory_label())
                        .arg("error", error),
                };
            }
            Action::Reload => {
                if self.doc().dirty() {
                    self.dialog = Some(Dialog::Reload);
                } else {
                    self.reload(maps);
                }
            }
            Action::Reset => {
                let doc = self.doc_mut();
                doc.begin();
                doc.map = doc.original.clone();
                doc.finish();
                self.status = tr("editor.status.original");
            }
            Action::Done | Action::Quit => {
                let quit = matches!(action, Action::Quit);
                if self.dirty() {
                    self.dialog = Some(Dialog::Leave(quit));
                    self.status = tr("editor.status.save_all");
                } else {
                    return Some(quit);
                }
            }
            Action::Cancel => self.dialog = None,
            Action::SaveLeave => {
                if let Some(Dialog::Leave(quit)) = self.dialog {
                    for kind in maps.kinds() {
                        if self.docs[kind.index()].dirty() {
                            if let Err(error) = self.save(kind, maps) {
                                self.status = tr("editor.status.save_failed").arg("error", error);
                                return None;
                            }
                        }
                    }
                    self.dialog = None;
                    return Some(quit);
                }
            }
            Action::Discard => match self.dialog.take() {
                Some(Dialog::Leave(quit)) => {
                    for doc in &mut self.docs {
                        doc.discard();
                    }
                    return Some(quit);
                }
                Some(Dialog::Reload) => self.reload(maps),
                None => {}
            },
        }
        self.clamp_offset();
        None
    }

    fn reload(&mut self, maps: &mut Maps) {
        let kind = self.kind;
        let result = self
            .directory
            .as_ref()
            .map_err(|e| io::Error::other(e.clone()))
            .and_then(|dir| self.docs[kind.index()].reload(dir, kind, maps));
        match result {
            Ok(()) => {
                maps.replace(kind, self.doc().map.clone());
                self.status = tr("editor.status.reloaded").arg("map", maps.title(kind));
            }
            Err(error) => {
                self.status = tr("editor.status.reload_failed").arg("error", error.to_string())
            }
        }
    }
}

// Use the uniform UI scale; 4x world pixels leave a two-pixel vertical border.
pub(crate) fn ui_point(cursor: Vec2, viewport: URect) -> Option<Vec2> {
    let point = cursor - viewport.min.as_vec2();
    let size = viewport.size().as_vec2();
    if size.min_element() <= 0.0 || point.cmplt(Vec2::ZERO).any() || point.cmpge(size).any() {
        return None;
    }
    Some(point * (game::WINDOW_SIZE.x as f32 / size.x))
}

fn line(from: UVec2, to: UVec2) -> Vec<UVec2> {
    let a = from.as_ivec2();
    let b = to.as_ivec2();
    let distance = (b - a).abs().max_element();
    (0..=distance)
        .map(|i| {
            if distance == 0 {
                from
            } else {
                a.as_vec2()
                    .lerp(b.as_vec2(), i as f32 / distance as f32)
                    .round()
                    .as_uvec2()
            }
        })
        .collect()
}

pub(crate) fn update(
    settings: Res<crate::settings::Settings>,
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    mut menu: ResMut<Menu>,
    mut maps: ResMut<Maps>,
    window: Single<(Entity, &Window)>,
    camera: Single<&Camera, With<game::OuterCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut close: MessageReader<WindowCloseRequested>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
    closing: Query<Entity, With<ClosingWindow>>,
) {
    let (window_entity, window) = *window;
    if closing.contains(window_entity) {
        // Match Bevy's two-frame close: let the renderer release the window first.
        commands.entity(window_entity).despawn();
        return;
    }
    let open = menu.page == Page::Editor;
    let close_requested = close.read().next().is_some();
    if !open {
        wheel.clear();
        editor.was_open = false;
        if close_requested {
            commands.entity(window_entity).insert(ClosingWindow);
        }
        return;
    }
    if !editor.was_open {
        editor.was_open = true;
        editor.ui_dirty = true;
        editor.canvas_dirty = true;
    }
    let focused = !settings.blocks_input()
        && (window.focused
            || (cfg!(debug_assertions) && std::env::var_os("YAPSHIRE_SMOKE").is_some()));
    let mut actions = Vec::new();
    if close_requested {
        actions.push(Action::Quit);
    }
    if focused {
        for (interaction, action) in &buttons {
            if *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left) {
                actions.push(*action);
            }
        }
        let control = keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ]);
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        for (key, action) in [
            (
                KeyCode::Escape,
                if editor.dialog.is_some() {
                    Action::Cancel
                } else {
                    Action::Done
                },
            ),
            (KeyCode::KeyS, Action::Save),
            (
                KeyCode::KeyZ,
                if shift { Action::Redo } else { Action::Undo },
            ),
            (KeyCode::KeyY, Action::Redo),
            (KeyCode::KeyB, Action::Tool(Tool::Brush)),
            (KeyCode::KeyE, Action::Tool(Tool::Eraser)),
            (KeyCode::KeyI, Action::Tool(Tool::Pick)),
            (KeyCode::KeyF, Action::Tool(Tool::Fill)),
            (KeyCode::KeyP, Action::Tool(Tool::Stamp)),
            (KeyCode::KeyG, Action::Grid),
            (KeyCode::KeyH, Action::Flip(0x8000_0000)),
            (KeyCode::KeyV, Action::Flip(0x4000_0000)),
            (KeyCode::Minus, Action::Zoom(-1)),
            (KeyCode::Equal, Action::Zoom(1)),
            (KeyCode::ArrowLeft, Action::Pan(IVec2::new(-3, 0))),
            (KeyCode::ArrowRight, Action::Pan(IVec2::new(3, 0))),
            (KeyCode::ArrowUp, Action::Pan(IVec2::new(0, -3))),
            (KeyCode::ArrowDown, Action::Pan(IVec2::new(0, 3))),
        ] {
            let needs_control = matches!(key, KeyCode::KeyS | KeyCode::KeyZ | KeyCode::KeyY);
            if keys.just_pressed(key) && (needs_control == control || key == KeyCode::Escape) {
                actions.push(action);
            }
        }
    }
    for action in actions {
        if let Some(quit) = editor.act(action, &mut maps) {
            if quit {
                commands.entity(window_entity).insert(ClosingWindow);
            } else {
                menu.go(Page::Home);
            }
            return;
        }
    }
    let point = window
        .physical_cursor_position()
        .and_then(|p| camera.physical_viewport_rect().and_then(|v| ui_point(p, v)));
    editor.hover = point.and_then(|p| editor.cell_at(p));
    if settings.blocks_input() {
        editor.hover = None;
    }
    if !focused || editor.dialog.is_some() {
        editor.finish();
        wheel.clear();
        return;
    }
    if editor.hover.is_some() {
        for event in wheel.read() {
            if event.y != 0.0 || event.x != 0.0 {
                let step = if event.y + event.x > 0.0 { -3 } else { 3 };
                editor.act(Action::Pan(IVec2::new(step, 0)), &mut maps);
            }
        }
    } else {
        wheel.clear();
    }
    if mouse.just_pressed(MouseButton::Middle) && editor.hover.is_some() {
        editor.finish();
        editor.panning = point.map(|p| (p, editor.offset));
    }
    if let Some((start, offset)) = editor.panning {
        if mouse.pressed(MouseButton::Middle) {
            if let Some(point) = point {
                let before = editor.offset;
                editor.offset = (offset.as_vec2() + (start - point) / editor.cell_size())
                    .round()
                    .max(Vec2::ZERO)
                    .as_uvec2();
                editor.clamp_offset();
                editor.ui_dirty |= editor.offset != before;
            }
        } else {
            editor.panning = None;
        }
        return;
    }
    if let Some(cell) = editor.hover {
        let left = mouse.just_pressed(MouseButton::Left);
        let right = mouse.just_pressed(MouseButton::Right);
        let layer = editor.layer;
        let mut painted = false;
        if editor.doc().map.layers[layer].kind != "tilelayer" {
            editor.finish();
            return;
        }
        if left || right {
            if left
                && (editor.tool == Tool::Pick
                    || keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]))
            {
                let gid = editor.doc().map.layers[layer].data
                    [(cell.y * editor.doc().map.width + cell.x) as usize];
                if gid != 0 {
                    editor.tile = gid;
                    editor.palette = ((gid & 0x0fff_ffff) - 1) as usize / 128;
                    editor.tool = Tool::Brush;
                }
                editor.ui_dirty = true;
            } else if !editor.doc().map.layers[layer].visible {
                editor.status = tr("editor.status.hidden");
            } else {
                let erase = right || editor.tool == Tool::Eraser;
                let gid = if erase { 0 } else { editor.tile };
                if left && editor.tool == Tool::Stamp {
                    if let Some((_, prefab)) =
                        maps.world.content.manifest.prefabs.iter().nth(editor.stamp)
                    {
                        editor.doc_mut().stamp(layer, cell, prefab, &maps);
                    }
                } else if left && editor.tool == Tool::Fill {
                    editor.doc_mut().fill(layer, cell, gid);
                } else {
                    editor.doc_mut().paint(layer, cell, gid);
                    editor.drawing = Some((cell, erase));
                }
                painted = true;
                editor.canvas_dirty = true;
            }
        } else if let Some((last, erase)) = editor.drawing {
            if cell != last
                && (mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right))
            {
                let gid = if erase { 0 } else { editor.tile };
                for point in line(last, cell) {
                    editor.doc_mut().paint(layer, point, gid);
                }
                editor.drawing = Some((cell, erase));
                painted = true;
                editor.canvas_dirty = true;
            }
        }
        if painted {
            let kind = editor.kind;
            editor.doc_mut().connect_terrain(layer, &maps, kind);
            if left && matches!(editor.tool, Tool::Fill | Tool::Stamp) {
                editor.finish();
            }
        }
    } else if editor.drawing.is_some() {
        // Re-entering the canvas starts a new stroke; never bridge across a toolbar.
        editor.finish();
    }
    if !mouse.pressed(MouseButton::Left)
        && !mouse.pressed(MouseButton::Right)
        && editor.drawing.is_some()
    {
        editor.finish();
    }
}
