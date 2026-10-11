//! Content pack v1. Tiled GIDs are layout-local; namespaced IDs identify assets.
use crate::maps::{Map, Property, string_property};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Read},
    path::{Path, PathBuf},
};

pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_PACK_BYTES: usize = 32 * 1024 * 1024;
pub const GID_MASK: u32 = 0x0fff_ffff;

pub(crate) fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

pub fn read_limited(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid(format!("{} exceeds {limit} bytes", path.display())));
    }
    Ok(bytes)
}

/// Resolve Tiled's relative paths without permitting a pack to escape its root.
pub fn relative(base: &str, file: &str) -> io::Result<String> {
    if file.is_empty() || file.contains(['\\', ':', '\0']) || file.starts_with('/') {
        return Err(invalid(format!("Invalid content path: {file}")));
    }
    let mut parts: Vec<_> = base.split('/').filter(|s| !s.is_empty()).collect();
    parts.pop();
    for part in file.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(invalid("Content path escapes the pack"));
                }
            }
            name => parts.push(name),
        }
    }
    if parts.is_empty() {
        return Err(invalid("Empty content path"));
    }
    Ok(parts.join("/"))
}

pub fn pack_file(root: &Path, path: &str) -> io::Result<PathBuf> {
    let path = relative("pack.json", path)?;
    let root = root.canonicalize()?;
    let file = root.join(path).canonicalize()?;
    if !file.starts_with(&root) {
        return Err(invalid("Content symlink escapes the pack"));
    }
    Ok(file)
}

pub fn valid_id(id: &str) -> bool {
    let Some((namespace, name)) = id.split_once(':') else {
        return false;
    };
    !namespace.is_empty()
        && !name.is_empty()
        && id.len() <= 96
        && namespace
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-/".contains(&b))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    pub id: String,
    pub version: String,
    pub tile_size: u32,
    pub tilesets: Vec<String>,
    #[serde(default)]
    pub images: Vec<String>,
    pub maps: Vec<MapInfo>,
    pub entry: Entry,
    #[serde(default)]
    pub prefabs: BTreeMap<String, Prefab>,
    #[serde(default)]
    pub terrains: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legacy_tiles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapInfo {
    pub id: String,
    pub path: String,
    pub title: String,
    #[serde(default)]
    pub indoors: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub map: String,
    pub spawn: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prefab {
    pub width: u32,
    pub height: u32,
    pub anchor: [i32; 2],
    pub tiles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tileset {
    pub image: String,
    pub imagewidth: u32,
    pub imageheight: u32,
    pub tilewidth: u32,
    pub tileheight: u32,
    pub tilecount: u32,
    pub columns: u32,
    #[serde(default)]
    pub margin: u32,
    #[serde(default)]
    pub spacing: u32,
    pub tiles: Vec<Tile>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub id: u32,
    pub properties: Vec<Property>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animation: Vec<Frame>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
impl Tile {
    pub fn key(&self) -> &str {
        string_property(&self.properties, "id").unwrap_or("")
    }
    pub fn collision(&self) -> &str {
        string_property(&self.properties, "collision").unwrap_or("none")
    }
    pub fn terrain(&self) -> Option<&str> {
        string_property(&self.properties, "terrain")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub tileid: u32,
    pub duration: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Content {
    pub manifest: Manifest,
    pub sets: BTreeMap<String, Tileset>,
    /// SHA-256 of image bytes. JSON definitions are hashed structurally instead.
    pub images: BTreeMap<String, String>,
}

impl Content {
    pub fn load(root: &Path) -> io::Result<Self> {
        Self::read(|name| read_limited(&pack_file(root, name)?, MAX_FILE_BYTES))
    }

    pub fn bundled() -> Self {
        Self::read(|name| crate::bundled_file(name).map(Vec::from)).expect("Valid bundled content")
    }

    fn read(mut read: impl FnMut(&str) -> io::Result<Vec<u8>>) -> io::Result<Self> {
        let manifest: Manifest = serde_json::from_slice(&read("pack.json")?)?;
        if manifest.tilesets.len() > 32 || manifest.images.len() > 64 {
            return Err(invalid("Too many content files"));
        }
        let mut sets = BTreeMap::new();
        let mut image_paths: BTreeSet<String> = manifest.images.iter().cloned().collect();
        let mut total = 0;
        for path in &manifest.tilesets {
            if relative("pack.json", path)? != *path || !path.ends_with(".tsj") {
                return Err(invalid("Invalid tileset path"));
            }
            let bytes = read(path)?;
            total += bytes.len();
            let mut set: Tileset = serde_json::from_slice(&bytes)?;
            set.tiles.sort_by_key(|t| t.id);
            image_paths.insert(relative(path, &set.image)?);
            sets.insert(path.clone(), set);
        }
        let mut images = BTreeMap::new();
        for path in image_paths {
            let bytes = read(&path)?;
            total += bytes.len();
            if total > MAX_PACK_BYTES {
                return Err(invalid("Content pack exceeds 32 MiB"));
            }
            if !path.ends_with(".png")
                || bytes.get(..8) != Some(b"\x89PNG\r\n\x1a\n")
                || bytes.get(12..16) != Some(b"IHDR")
                || bytes.len() < 24
            {
                return Err(invalid(format!("Expected a PNG image: {path}")));
            }
            let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
            let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
            if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
                return Err(invalid("PNG dimensions exceed 4096px"));
            }
            for (source, set) in &sets {
                if relative(source, &set.image)? == path
                    && (set.imagewidth, set.imageheight) != (width, height)
                {
                    return Err(invalid(format!(
                        "Tileset image dimensions disagree: {source}"
                    )));
                }
            }
            images.insert(path, format!("{:x}", Sha256::digest(bytes)));
        }
        let content = Self {
            manifest,
            sets,
            images,
        };
        content.validate()?;
        Ok(content)
    }

    pub fn hash(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("Serializable content"))
        )
    }

    pub fn require(&self, installed: &Self) -> io::Result<()> {
        if self.hash() != installed.hash() {
            let file = self
                .images
                .iter()
                .find(|(path, hash)| installed.images.get(*path) != Some(*hash))
                .map(|(path, _)| path.as_str())
                .unwrap_or("pack.json / tileset definitions");
            return Err(invalid(format!(
                "Required pack {} v{} ({:.12}); installed {} v{} ({:.12}). Content differs: {file}. Install the server's pack and restart.",
                self.manifest.id,
                self.manifest.version,
                self.hash(),
                installed.manifest.id,
                installed.manifest.version,
                installed.hash()
            )));
        }
        Ok(())
    }

    pub fn validate(&self) -> io::Result<()> {
        let p = &self.manifest;
        if p.format != 1
            || p.tile_size != 16
            || !valid_id(&format!("{}:pack", p.id))
            || p.version.is_empty()
            || p.version.len() > 32
            || !(1..=32).contains(&p.tilesets.len())
            || !(1..=32).contains(&p.maps.len())
            || p.images.len() > 64
            || p.prefabs.len() > 256
            || p.terrains.len() > 64
            || self.sets.len() != p.tilesets.len()
            || self.images.len() > 96
        {
            return Err(invalid("Unsupported or oversized content pack v1"));
        }
        let mut keys = BTreeSet::new();
        let mut paths = BTreeSet::new();
        let mut count = 0;
        for path in &p.tilesets {
            if relative("pack.json", path)? != *path || !paths.insert(path) {
                return Err(invalid("Duplicate or invalid tileset path"));
            }
            let set = self
                .sets
                .get(path)
                .ok_or_else(|| invalid(format!("Missing tileset {path}")))?;
            if set.tilewidth != 16
                || set.tileheight != 16
                || set.columns == 0
                || set.columns > 256
                || set.tilecount == 0
                || set.tilecount > 4096
                || set.imagewidth != set.columns * 16
                || set.imageheight == 0
                || set.imageheight > 4096
                || set.imageheight % 16 != 0
                || set.tilecount > set.columns * (set.imageheight / 16)
                || set.margin != 0
                || set.spacing != 0
                || set.tiles.len() != set.tilecount as usize
            {
                return Err(invalid(format!("Invalid 16px tileset: {path}")));
            }
            count += set.tilecount;
            let image = relative(path, &set.image)?;
            if !self.images.contains_key(&image) {
                return Err(invalid(format!("Missing image {image}")));
            }
            for (index, tile) in set.tiles.iter().enumerate() {
                if tile.id != index as u32
                    || tile.id >= set.tilecount
                    || !valid_id(tile.key())
                    || !keys.insert(tile.key())
                    || !matches!(tile.collision(), "none" | "solid" | "platform")
                {
                    return Err(invalid(format!(
                        "Invalid or duplicate tile ID in {path}: {}",
                        tile.key()
                    )));
                }
                crate::maps::validate_properties(&tile.properties)?;
                if tile.properties.iter().any(|p| {
                    matches!(p.name.as_str(), "id" | "collision" | "terrain") && p.kind != "string"
                }) || tile.extra.contains_key("image")
                    || tile.extra.contains_key("objectgroup")
                {
                    return Err(invalid(
                        "Tile IDs, collisions and terrains must be strings; per-tile images/polygons are unsupported",
                    ));
                }
                if tile.animation.len() > 64
                    || tile.animation.iter().enumerate().any(|(i, f)| {
                        f.tileid >= set.tilecount
                            || f.duration == 0
                            || f.duration > 60_000
                            || f.tileid != tile.animation[0].tileid + i as u32
                            || f.duration != tile.animation[0].duration
                    })
                {
                    return Err(invalid(format!(
                        "Animation must use consecutive, equally timed frames: {}",
                        tile.key()
                    )));
                }
            }
        }
        if count > 8192 {
            return Err(invalid("Pack exceeds 8192 tiles"));
        }
        for (path, hash) in &self.images {
            if relative("pack.json", path)? != *path
                || !path.ends_with(".png")
                || hash.len() != 64
                || !hash.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(invalid("Invalid image path or SHA-256"));
            }
        }
        for image in &p.images {
            if !self.images.contains_key(image) {
                return Err(invalid(format!("Missing image {image}")));
            }
        }
        let mut ids = BTreeSet::new();
        for map in &p.maps {
            if !valid_id(&map.id)
                || !ids.insert(&map.id)
                || !paths.insert(&map.path)
                || relative("pack.json", &map.path)? != map.path
                || !map.path.ends_with(".tmj")
                || map.title.is_empty()
                || map.title.len() > 80
            {
                return Err(invalid("Invalid or duplicate map definition"));
            }
        }
        if !ids.contains(&p.entry.map) {
            return Err(invalid("Entry map is missing"));
        }
        for (key, prefab) in &p.prefabs {
            if !valid_id(key)
                || !(1..=32).contains(&prefab.width)
                || !(1..=32).contains(&prefab.height)
                || prefab.tiles.len() != (prefab.width * prefab.height) as usize
                || prefab.anchor[0] < 0
                || prefab.anchor[0] > prefab.width as i32 * 16
                || prefab.anchor[1] < 0
                || prefab.anchor[1] > prefab.height as i32 * 16
                || prefab
                    .tiles
                    .iter()
                    .any(|id| !id.is_empty() && !keys.contains(id.as_str()))
            {
                return Err(invalid(format!("Invalid prefab {key}")));
            }
        }
        for (key, variants) in &p.terrains {
            if !valid_id(key)
                || variants.len() != 16
                || variants.iter().any(|id| !keys.contains(id.as_str()))
            {
                return Err(invalid(format!("Terrain {key} needs all 16 edge masks")));
            }
            let locations: Vec<_> = variants.iter().filter_map(|id| self.find(id)).collect();
            if locations
                .iter()
                .any(|(source, _, tile)| *source != locations[0].0 || tile.terrain() != Some(key))
            {
                return Err(invalid(
                    "Connecting terrain variants must share one tileset and terrain ID",
                ));
            }
        }
        for tile in self.sets.values().flat_map(|s| &s.tiles) {
            if let Some(terrain) = tile.terrain() {
                if !p
                    .terrains
                    .get(terrain)
                    .is_some_and(|ids| ids.iter().any(|id| id == tile.key()))
                {
                    return Err(invalid("Unregistered terrain tile"));
                }
            }
        }
        if !p.legacy_tiles.is_empty()
            && (p.id != "yapshire"
                || p.legacy_tiles.len() != 359
                || p.legacy_tiles.iter().any(|id| !keys.contains(id.as_str())))
        {
            return Err(invalid("Invalid legacy migration table"));
        }
        Ok(())
    }

    pub fn find(&self, id: &str) -> Option<(&str, &Tileset, &Tile)> {
        self.sets.iter().find_map(|(path, set)| {
            set.tiles
                .iter()
                .find(|t| t.key() == id)
                .map(|t| (path.as_str(), set, t))
        })
    }

    pub fn resolve<'a>(
        &'a self,
        map: &Map,
        path: &str,
        gid: u32,
    ) -> Option<(&'a str, &'a Tileset, &'a Tile)> {
        let index = gid & GID_MASK;
        if index == 0 {
            return None;
        }
        let reference = map
            .tilesets
            .iter()
            .filter(|t| t.firstgid <= index)
            .max_by_key(|t| t.firstgid)?;
        let source = relative(path, &reference.source).ok()?;
        let (source, set) = self.sets.get_key_value(&source)?;
        let local = index - reference.firstgid;
        Some((source, set, set.tiles.get(local as usize)?))
    }

    pub fn gid(&self, id: &str) -> Option<u32> {
        let mut first = 1;
        for source in &self.manifest.tilesets {
            let set = self.sets.get(source)?;
            if let Some(tile) = set.tiles.iter().find(|t| t.key() == id) {
                return Some(first + tile.id);
            }
            first += set.tilecount;
        }
        None
    }

    /// Save ordinary Tiled files, with a deterministic palette rather than external GIDs.
    pub fn normalize(&self, map: &mut Map, path: &str) -> io::Result<()> {
        // Tiled preserves custom properties; this snapshot lets an atlas be repacked
        // without changing what existing map cells mean.
        let properties = map
            .extra
            .entry("properties")
            .or_insert_with(|| serde_json::json!([]));
        let properties = properties
            .as_array_mut()
            .ok_or_else(|| invalid("Map properties must be an array"))?;
        let snapshots: Vec<_> = properties
            .iter()
            .filter(|p| p["name"] == "yapshire:palette")
            .collect();
        if snapshots.len() > 1 {
            return Err(invalid("Duplicate palette snapshot"));
        }
        let previous: BTreeMap<String, Vec<String>> = match snapshots.first() {
            Some(p) => serde_json::from_str(
                p["value"]
                    .as_str()
                    .ok_or_else(|| invalid("Invalid palette snapshot"))?,
            )?,
            None => BTreeMap::new(),
        };
        if previous.len() > 32 || previous.values().map(Vec::len).sum::<usize>() > 8192 {
            return Err(invalid("Palette snapshot exceeds the tile limit"));
        }
        let mut lookup = BTreeMap::new();
        let mut first = 1;
        for source in &self.manifest.tilesets {
            let set = &self.sets[source];
            for tile in &set.tiles {
                lookup.insert(tile.key(), first + tile.id);
            }
            first += set.tilecount;
        }
        let mut mapping = BTreeMap::from([(0, 0)]);
        let mut sources = BTreeSet::new();
        let mut metadata = BTreeMap::new();
        for reference in &map.tilesets {
            let source = relative(path, &reference.source)?;
            if !sources.insert(source.clone()) {
                return Err(invalid("Duplicate tileset reference"));
            }
            metadata.insert(source.clone(), reference.extra.clone());
            let set = self
                .sets
                .get(&source)
                .ok_or_else(|| invalid(format!("Unknown tileset: {source}")))?;
            let ids: Vec<_> = if let Some(ids) = previous.get(&source) {
                ids.iter().map(String::as_str).collect()
            } else {
                (0..set.tilecount)
                    .map(|id| set.tiles.iter().find(|t| t.id == id).unwrap().key())
                    .collect()
            };
            if reference.firstgid == 0 || reference.firstgid > GID_MASK - ids.len() as u32 {
                return Err(invalid("Invalid GID range"));
            }
            for (local, id) in ids.iter().enumerate() {
                let gid = *lookup
                    .get(id)
                    .ok_or_else(|| invalid(format!("Map requires missing tile {id}")))?;
                if mapping
                    .insert(reference.firstgid + local as u32, gid)
                    .is_some()
                {
                    return Err(invalid("Overlapping GID ranges"));
                }
            }
        }
        for layer in &mut map.layers {
            for gid in &mut layer.data {
                let index = *gid & GID_MASK;
                if *gid != 0 && (index == 0 || *gid & 0x1000_0000 != 0) {
                    return Err(invalid("Invalid tile flip flags"));
                }
                let new = mapping
                    .get(&index)
                    .ok_or_else(|| invalid(format!("Unknown GID {index}")))?;
                *gid = *new | (*gid & !GID_MASK);
            }
        }
        let prefix = "../".repeat(path.matches('/').count());
        let mut firstgid = 1;
        map.tilesets = self
            .manifest
            .tilesets
            .iter()
            .map(|source| {
                let r = crate::maps::TilesetRef {
                    firstgid,
                    source: format!("{prefix}{source}"),
                    extra: metadata.remove(source).unwrap_or_default(),
                };
                firstgid += self.sets[source].tilecount;
                r
            })
            .collect();
        let snapshot: BTreeMap<_, Vec<_>> = self
            .sets
            .iter()
            .map(|(path, set)| {
                (
                    path,
                    (0..set.tilecount)
                        .map(|id| set.tiles.iter().find(|t| t.id == id).unwrap().key())
                        .collect(),
                )
            })
            .collect();
        let properties = map
            .extra
            .get_mut("properties")
            .unwrap()
            .as_array_mut()
            .unwrap();
        properties.retain(|p| p["name"] != "yapshire:palette");
        properties.push(serde_json::json!({"name":"yapshire:palette", "type":"string", "value":serde_json::to_string(&snapshot)?}));
        Ok(())
    }
}
