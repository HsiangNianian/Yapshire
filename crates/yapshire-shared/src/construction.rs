//! A pack-derived construction catalog. Saved cells refer to stable IDs, never atlas positions.
use crate::{
    content::{Content, relative},
    maps::string_property,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Wall,
    #[default]
    Solid,
    Decor,
}
impl Layer {
    pub const ALL: [Self; 3] = [Self::Wall, Self::Solid, Self::Decor];
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collision {
    None,
    Solid,
    Platform,
}

pub struct Piece {
    pub id: String,
    pub label: String,
    pub source: String,
    pub image: String,
    pub index: u32,
    pub columns: u32,
    pub collision: Collision,
    pub layer: Layer,
    pub family: Option<String>,
    pub animation: Vec<crate::content::Frame>,
}

pub struct Catalog {
    pub pieces: Vec<Piece>,
    pub groups: Vec<String>,
    pub terrains: BTreeMap<String, Vec<String>>,
    pub prefabs: BTreeMap<String, crate::content::Prefab>,
    indices: BTreeMap<String, usize>,
}
impl Catalog {
    pub fn new(content: &Content) -> Self {
        let mut pieces = Vec::new();
        let mut indices = BTreeMap::new();
        for source in &content.manifest.tilesets {
            let set = &content.sets[source];
            for tile in &set.tiles {
                let collision = match tile.collision() {
                    "solid" => Collision::Solid,
                    "platform" => Collision::Platform,
                    _ => Collision::None,
                };
                let layer = match string_property(&tile.properties, "build_layer") {
                    Some("wall") => Layer::Wall,
                    Some("solid") => Layer::Solid,
                    Some("decor") => Layer::Decor,
                    _ if collision != Collision::None => Layer::Solid,
                    _ => Layer::Decor,
                };
                indices.insert(tile.key().to_owned(), pieces.len());
                pieces.push(Piece {
                    id: tile.key().into(),
                    label: string_property(&tile.properties, "label")
                        .unwrap_or(tile.key())
                        .into(),
                    source: source.clone(),
                    image: relative(source, &set.image).expect("Validated content image"),
                    index: tile.id,
                    columns: set.columns,
                    collision,
                    layer,
                    family: tile.terrain().map(str::to_owned),
                    animation: tile.animation.clone(),
                });
            }
        }
        Self {
            pieces,
            indices,
            groups: content.manifest.tilesets.clone(),
            terrains: content.manifest.terrains.clone(),
            prefabs: content.manifest.prefabs.clone(),
        }
    }
    pub fn index(&self, id: &str) -> Option<usize> {
        self.indices.get(id).copied()
    }
    pub fn get(&self, id: &str) -> Option<&Piece> {
        self.index(id).map(|i| &self.pieces[i])
    }

    pub fn prefab_cell(&self, name: &str, layer: Layer, x: i32, y: i32) -> &str {
        let Some(prefab) = self.prefabs.get(name) else {
            return "";
        };
        if x < 0 || y < 0 || x >= prefab.width as i32 || y >= prefab.height as i32 {
            return "";
        }
        let id = &prefab.tiles[(y as u32 * prefab.width + x as u32) as usize];
        if self.get(id).is_some_and(|piece| piece.layer == layer) {
            id
        } else {
            ""
        }
    }
}
