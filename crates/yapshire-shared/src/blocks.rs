//! Authored objects keep their identity, visuals and behavior independent of atlas indices.
use crate::content::{Content, invalid, relative, valid_id};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Behavior {
    #[default]
    Decoration,
    Portal,
    Shop,
    Fishing,
    Toggle,
}
impl Behavior {
    pub fn kind(self) -> &'static str {
        match self {
            Self::Decoration => "block",
            Self::Portal => "portal",
            Self::Shop => "shop",
            Self::Fishing => "fishing",
            Self::Toggle => "toggle",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub texture: String,
    pub offset: [i32; 2],
    pub size: [u32; 2],
    #[serde(default)]
    pub flip_x: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub size: [u32; 2],
    pub anchor: [i32; 2],
    pub parts: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collider {
    /// x, y, width, height in pixels relative to the placement anchor, Y down.
    pub rect: [i32; 4],
    #[serde(default)]
    pub platform: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub model: String,
    #[serde(default)]
    pub colliders: Option<Vec<Collider>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<crate::pixels::Material>,
    #[serde(default)]
    pub tile: bool,
    pub model: String,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
    #[serde(default)]
    pub behavior: Behavior,
    #[serde(default)]
    pub colliders: Vec<Collider>,
    #[serde(default)]
    pub selection: Vec<[i32; 4]>,
    #[serde(default)]
    pub interaction: Option<[i32; 4]>,
    #[serde(default)]
    pub states: BTreeMap<String, Variant>,
}
impl Block {
    pub fn model<'a>(&'a self, state: &str, content: &'a Content) -> &'a Model {
        let id = self.states.get(state).map_or(&self.model, |v| &v.model);
        &content.models[id]
    }
    pub fn colliders(&self, state: &str) -> &[Collider] {
        self.states
            .get(state)
            .and_then(|v| v.colliders.as_deref())
            .unwrap_or(&self.colliders)
    }
    pub fn valid_state(&self, state: &str) -> bool {
        state == "default" || self.states.contains_key(state)
    }
    pub fn hit(&self, state: &str, content: &Content, x: f32, y: f32) -> bool {
        let model = self.model(state, content);
        if self.selection.is_empty() {
            model.parts.iter().any(|p| {
                inside(
                    [
                        p.offset[0] - model.anchor[0],
                        p.offset[1] - model.anchor[1],
                        p.size[0] as i32,
                        p.size[1] as i32,
                    ],
                    x,
                    y,
                )
            })
        } else {
            self.selection.iter().any(|r| inside(*r, x, y))
        }
    }
}
pub fn inside([left, top, width, height]: [i32; 4], x: f32, y: f32) -> bool {
    x >= left as f32 && y >= top as f32 && x < (left + width) as f32 && y < (top + height) as f32
}
fn valid_rect([x, y, w, h]: [i32; 4]) -> bool {
    (-1024..=1024).contains(&x)
        && (-1024..=1024).contains(&y)
        && (1..=1024).contains(&w)
        && (1..=1024).contains(&h)
}
pub fn validate(content: &Content) -> io::Result<()> {
    if content.blocks.len() > 8192
        || content.models.len() > 8192
        || content.blocks.len() != content.manifest.blocks.len()
        || content.models.len() != content.manifest.models.len()
    {
        return Err(invalid("Invalid block/model registry size"));
    }
    let mut paths = std::collections::BTreeSet::new();
    for (id, path) in content
        .manifest
        .blocks
        .iter()
        .chain(&content.manifest.models)
    {
        if !valid_id(id)
            || relative("pack.json", path)? != *path
            || !path.ends_with(".json")
            || !paths.insert(path)
        {
            return Err(invalid(format!(
                "Invalid or duplicate definition path: {id}"
            )));
        }
    }
    for (id, model) in &content.models {
        if !content.manifest.models.contains_key(id)
            || model.size.iter().any(|v| !(1..=1024).contains(v))
            || model.anchor.iter().any(|v| !(-1024..=1024).contains(v))
            || model.parts.is_empty()
            || model.parts.len() > 128
        {
            return Err(invalid(format!("Invalid model: {id}")));
        }
        for part in &model.parts {
            if !content.images.contains_key(&part.texture)
                || part.size.iter().any(|v| !(1..=1024).contains(v))
                || (0..2).any(|i| {
                    part.offset[i] < 0 || part.offset[i] as u32 + part.size[i] > model.size[i]
                })
            {
                return Err(invalid(format!("Invalid model part: {id}")));
            }
        }
    }
    for (id, block) in &content.blocks {
        if !content.manifest.blocks.contains_key(id)
            || !content.models.contains_key(&block.model)
            || block.properties.len() > 16
            || block
                .properties
                .iter()
                .any(|(k, v)| k.len() > 80 || v.len() > 256)
            || block.colliders.len() > 64
            || block.selection.len() > 128
            || block.states.len() > 16
            || block.colliders.iter().any(|c| !valid_rect(c.rect))
            || block.selection.iter().any(|r| !valid_rect(*r))
            || block.interaction.is_some_and(|r| !valid_rect(r))
            || (block.behavior != Behavior::Decoration && block.interaction.is_none())
            || (block.behavior == Behavior::Toggle && !block.states.contains_key("open"))
            || (block.material.is_some()
                && (block.behavior != Behavior::Decoration || !block.states.is_empty()))
        {
            return Err(invalid(format!("Invalid block: {id}")));
        }
        if block.tile
            && (content.models[&block.model].size != [16, 16]
                || content.models[&block.model].anchor != [0, 0]
                || block.behavior != Behavior::Decoration
                || !block.states.is_empty()
                || block.colliders.iter().any(|c| {
                    c.rect[0] < 0
                        || c.rect[1] < 0
                        || c.rect[0] + c.rect[2] > 16
                        || c.rect[1] + c.rect[3] > 16
                }))
        {
            return Err(invalid(format!(
                "Tile blocks need a static 16px top-left model: {id}"
            )));
        }
        for (state, variant) in &block.states {
            if state == "default"
                || state.is_empty()
                || state.len() > 32
                || !state
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                || !content.models.contains_key(&variant.model)
                || variant
                    .colliders
                    .as_ref()
                    .is_some_and(|cs| cs.len() > 64 || cs.iter().any(|c| !valid_rect(c.rect)))
            {
                return Err(invalid(format!("Invalid block state: {id}/{state}")));
            }
        }
    }
    Ok(())
}
