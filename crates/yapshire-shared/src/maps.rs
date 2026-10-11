use crate::content::{Content, GID_MASK, invalid, relative};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Property {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub value: serde_json::Value,
}
pub fn string_property<'a>(properties: &'a [Property], name: &str) -> Option<&'a str> {
    properties.iter().find(|p| p.name == name)?.value.as_str()
}
pub fn bool_property(properties: &[Property], name: &str) -> bool {
    properties
        .iter()
        .find(|p| p.name == name)
        .and_then(|p| p.value.as_bool())
        .unwrap_or(false)
}
pub fn validate_properties(properties: &[Property]) -> io::Result<()> {
    let mut names = BTreeSet::new();
    if properties.len() > 32
        || properties.iter().any(|p| {
            p.name.len() > 80
                || !names.insert(&p.name)
                || match p.kind.as_str() {
                    "string" => p.value.as_str().is_none_or(|v| v.len() > 256),
                    "bool" => !p.value.is_boolean(),
                    "float" | "int" => !p.value.is_number(),
                    _ => true,
                }
        })
    {
        return Err(invalid(
            "Invalid, duplicate or unsupported Tiled properties",
        ));
    }
    Ok(())
}
fn visible() -> bool {
    true
}
fn opaque() -> f32 {
    1.0
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Map {
    pub width: u32,
    pub height: u32,
    pub tilewidth: u32,
    pub tileheight: u32,
    pub orientation: String,
    pub infinite: bool,
    pub tilesets: Vec<TilesetRef>,
    pub layers: Vec<Layer>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TilesetRef {
    pub firstgid: u32,
    pub source: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Layer {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub offsetx: f32,
    #[serde(default)]
    pub offsety: f32,
    #[serde(default = "visible")]
    pub visible: bool,
    #[serde(default = "opaque")]
    pub opacity: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<Object>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<Property>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
impl Layer {
    pub fn z(&self, index: usize) -> f32 {
        self.properties
            .iter()
            .find(|p| p.name == "z")
            .and_then(|p| p.value.as_f64())
            .map_or(-30.0 + index as f32 * 6.0, |n| n as f32)
    }
}
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Object {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type", alias = "class")]
    pub kind: String,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub width: f32,
    #[serde(default)]
    pub height: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default = "visible")]
    pub visible: bool,
    #[serde(default)]
    pub point: bool,
    #[serde(default)]
    pub properties: Vec<Property>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
impl Object {
    pub fn property(&self, name: &str) -> Option<&str> {
        string_property(&self.properties, name)
    }
    pub fn contains(&self, map: &Map, x: f32, y: f32) -> bool {
        let y = map.origin_y() - y;
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

impl Map {
    pub fn origin_y(&self) -> f32 {
        self.height as f32 * 16.0 - 64.0
    }
    pub fn objects(&self) -> impl Iterator<Item = &Object> {
        self.layers
            .iter()
            .filter(|l| l.kind == "objectgroup")
            .flat_map(|l| &l.objects)
    }
    pub fn spawn(&self, name: &str) -> Option<[f32; 2]> {
        self.objects()
            .find(|o| o.kind == "spawn" && o.name == name)
            .map(|o| [o.x, self.origin_y() - o.y])
    }
    pub fn interaction(&self, x: f32, y: f32) -> Option<&Object> {
        self.objects().find(|o| {
            matches!(o.kind.as_str(), "portal" | "shop" | "fishing") && o.contains(self, x, y)
        })
    }
    pub fn validate(&self, content: &Content, path: &str) -> io::Result<()> {
        if !(2..=256).contains(&self.width)
            || !(4..=128).contains(&self.height)
            || self.tilewidth != 16
            || self.tileheight != 16
            || self.orientation != "orthogonal"
            || self.infinite
            || !(1..=32).contains(&self.layers.len())
            || self.tilesets.len() > 32
        {
            return Err(invalid(
                "Expected a finite 16px orthogonal map (2..256 x 4..128, at most 32 layers)",
            ));
        }
        let mut ranges = Vec::new();
        let mut sources = BTreeSet::new();
        for r in &self.tilesets {
            let source = relative(path, &r.source)?;
            let set = content
                .sets
                .get(&source)
                .ok_or_else(|| invalid(format!("Unknown tileset: {}", r.source)))?;
            if !sources.insert(source) || r.firstgid == 0 || r.firstgid > GID_MASK - set.tilecount {
                return Err(invalid("Invalid tileset GID range"));
            }
            ranges.push((r.firstgid, r.firstgid + set.tilecount));
        }
        ranges.sort_unstable();
        if ranges.windows(2).any(|r| r[0].1 > r[1].0) {
            return Err(invalid("Overlapping tileset GID ranges"));
        }
        let mut object_ids = BTreeSet::new();
        let mut spawns = BTreeSet::new();
        for layer in &self.layers {
            validate_properties(&layer.properties)?;
            if !layer.z(0).is_finite()
                || !(-100.0..=100.0).contains(&layer.z(0))
                || layer.properties.iter().any(|p| {
                    (p.name == "collision" && p.kind != "bool")
                        || (p.name == "z" && !matches!(p.kind.as_str(), "int" | "float"))
                })
            {
                return Err(invalid("Invalid layer depth or collision property"));
            }
            if layer.name.len() > 80
                || layer.x != 0.0
                || layer.y != 0.0
                || !layer.offsetx.is_finite()
                || !layer.offsety.is_finite()
                || layer.offsetx.abs() > 4096.0
                || layer.offsety.abs() > 4096.0
                || !layer.opacity.is_finite()
                || !(0.0..=1.0).contains(&layer.opacity)
                || layer.extra.keys().any(|k| {
                    matches!(
                        k.as_str(),
                        "chunks" | "layers" | "tintcolor" | "parallaxx" | "parallaxy"
                    )
                })
            {
                return Err(invalid(format!(
                    "Unsupported layer transform: {}",
                    layer.name
                )));
            }
            match layer.kind.as_str() {
                "tilelayer" => {
                    if layer.image.is_some()
                        || layer.width != self.width
                        || layer.height != self.height
                        || layer.offsetx != 0.0
                        || layer.offsety != 0.0
                        || layer.data.len() != (self.width * self.height) as usize
                        || !layer.objects.is_empty()
                        || layer.data.iter().any(|gid| {
                            *gid != 0
                                && (gid & 0x1000_0000 != 0
                                    || content.resolve(self, path, *gid).is_none())
                        })
                    {
                        return Err(invalid(format!("Invalid tile data: {}", layer.name)));
                    }
                }
                "imagelayer" => {
                    let image = relative(
                        path,
                        layer
                            .image
                            .as_deref()
                            .ok_or_else(|| invalid("Image layer has no image"))?,
                    )?;
                    if !content.manifest.images.contains(&image)
                        || !layer.data.is_empty()
                        || !layer.objects.is_empty()
                    {
                        return Err(invalid(format!("Unregistered background: {image}")));
                    }
                }
                "objectgroup" => {
                    if layer.image.is_some()
                        || layer.offsetx != 0.0
                        || layer.offsety != 0.0
                        || !layer.data.is_empty()
                        || layer.objects.len() > 256
                    {
                        return Err(invalid("Invalid object layer"));
                    }
                    for o in &layer.objects {
                        validate_properties(&o.properties)?;
                        if o.id == 0
                            || !object_ids.insert(o.id)
                            || o.name.len() > 80
                            || !o.x.is_finite()
                            || !o.y.is_finite()
                            || !o.width.is_finite()
                            || !o.height.is_finite()
                            || o.x < 0.0
                            || o.y < 0.0
                            || o.width < 0.0
                            || o.height < 0.0
                            || o.x + o.width > self.width as f32 * 16.0
                            || o.y + o.height > self.height as f32 * 16.0
                            || o.rotation != 0.0
                            || o.extra.keys().any(|k| {
                                matches!(
                                    k.as_str(),
                                    "gid"
                                        | "template"
                                        | "ellipse"
                                        | "polygon"
                                        | "polyline"
                                        | "text"
                                )
                            })
                        {
                            return Err(invalid(format!(
                                "Invalid rectangle/point object: {}",
                                o.name
                            )));
                        }
                        match o.kind.as_str() {
                            "spawn"
                                if o.point
                                    && o.width == 0.0
                                    && o.height == 0.0
                                    && !o.name.is_empty()
                                    && spawns.insert(&o.name) => {}
                            "portal"
                                if o.width > 0.0
                                    && o.height > 0.0
                                    && o.property("target_map").is_some()
                                    && o.property("target_spawn").is_some() => {}
                            "shop" | "fishing" | "solid" if o.width > 0.0 && o.height > 0.0 => {}
                            "prefab" => {
                                let p = o
                                    .property("prefab")
                                    .and_then(|key| content.manifest.prefabs.get(key))
                                    .ok_or_else(|| {
                                        invalid(format!("Unknown prefab: {}", o.name))
                                    })?;
                                let x = o.x - p.anchor[0] as f32;
                                let y = o.y - p.anchor[1] as f32;
                                if x < 0.0
                                    || y < 0.0
                                    || x % 16.0 != 0.0
                                    || y % 16.0 != 0.0
                                    || x + p.width as f32 * 16.0 > self.width as f32 * 16.0
                                    || y + p.height as f32 * 16.0 > self.height as f32 * 16.0
                                {
                                    return Err(invalid(
                                        "Prefab must fit the map and align to the 16px grid",
                                    ));
                                }
                            }
                            _ => {
                                return Err(invalid(format!(
                                    "Unsupported object type or dimensions: {}",
                                    o.kind
                                )));
                            }
                        }
                    }
                }
                _ => return Err(invalid(format!("Unsupported layer type: {}", layer.kind))),
            }
        }
        if spawns.is_empty() {
            return Err(invalid("Every map needs a named spawn point"));
        }
        if serde_json::to_vec(self)?.len() as u64 > crate::MAX_MAP_BYTES {
            return Err(invalid("Map exceeds the size limit"));
        }
        Ok(())
    }

    pub fn collision(&self, content: &Content, path: &str, col: i32, row: i32) -> &str {
        if col < 0 || row < 0 || col >= self.width as i32 || row >= self.height as i32 {
            return "none";
        }
        let mut platform = false;
        for layer in &self.layers {
            if layer.kind == "tilelayer" && bool_property(&layer.properties, "collision") {
                if let Some((_, _, tile)) = content.resolve(
                    self,
                    path,
                    layer.data[(row as u32 * self.width + col as u32) as usize],
                ) {
                    if tile.collision() == "solid" {
                        return "solid";
                    }
                    platform |= tile.collision() == "platform";
                }
            }
        }
        if platform { "platform" } else { "none" }
    }

    pub fn expanded(&self, content: &Content) -> Self {
        let mut map = self.clone();
        for layer in &mut map.layers {
            if layer.kind != "objectgroup" {
                continue;
            }
            layer.data = vec![0; (self.width * self.height) as usize];
            for o in &layer.objects {
                if o.kind != "prefab" || !o.visible {
                    continue;
                }
                let Some(p) = o
                    .property("prefab")
                    .and_then(|id| content.manifest.prefabs.get(id))
                else {
                    continue;
                };
                let x = (o.x as i32 - p.anchor[0]) / 16;
                let y = (o.y as i32 - p.anchor[1]) / 16;
                for (i, id) in p.tiles.iter().enumerate() {
                    let col = x + i as i32 % p.width as i32;
                    let row = y + i as i32 / p.width as i32;
                    if col >= 0 && row >= 0 && col < self.width as i32 && row < self.height as i32 {
                        if let Some(gid) = content.gid(id) {
                            layer.data[(row as u32 * self.width + col as u32) as usize] = gid;
                        }
                    }
                }
            }
        }
        map
    }
}
