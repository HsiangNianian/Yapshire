use crate::{
    content::{Content, GID_MASK, invalid, pack_file, read_limited},
    maps::Map,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io, path::Path};

pub const PROTOCOL_VERSION: u32 = 3;
pub const WORLD_FORMAT: u32 = 2;
pub const MAX_MAP_BYTES: u64 = 1024 * 1024;
pub const MAX_WORLD_BYTES: usize = 8 * 1024 * 1024;
// Read-only compatibility fixtures: old editor saves migrate only when explicitly saved.
pub const TOWN: &str = include_str!("../../../assets/maps/town.tmj");
pub const SHOP: &str = include_str!("../../../assets/maps/tackle-shop.tmj");
pub const TILESET: &[u8] = include_bytes!("../../../assets/maps/harbor.tsj");
pub const TEXTURE: &[u8] = include_bytes!("../../../assets/maps/harbor.png");

macro_rules! bundled_files {
    ($($path:literal),+ $(,)?) => { pub const BUNDLED_FILES: &[(&str, &[u8])] = &[$(($path, include_bytes!(concat!("../../../assets/packs/yapshire/", $path)))),+]; };
}
bundled_files!(
    "pack.json",
    "terrain/ground.tsj",
    "terrain/ground.png",
    "terrain/water.tsj",
    "terrain/water.png",
    "buildings/tackle.tsj",
    "buildings/tackle.png",
    "objects/harbor.tsj",
    "objects/harbor.png",
    "backgrounds/street.png",
    "maps/town.tmj",
    "maps/tackle-shop.tmj"
);
pub fn bundled_file(name: &str) -> io::Result<&'static [u8]> {
    BUNDLED_FILES
        .iter()
        .find(|(path, _)| *path == name)
        .map(|(_, bytes)| *bytes)
        .ok_or_else(|| invalid(format!("Missing bundled file {name}")))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub format: u32,
    pub content: Content,
    pub revision: String,
    pub maps: BTreeMap<String, Map>,
}

pub fn validate_tileset(folder: &Path) -> io::Result<()> {
    for (name, expected) in [("harbor.tsj", TILESET), ("harbor.png", TEXTURE)] {
        let bytes = read_limited(&folder.join(name), expected.len() as u64 * 2)?;
        let matches = if name.ends_with(".tsj") {
            serde_json::from_slice::<serde_json::Value>(&bytes)?
                == serde_json::from_slice::<serde_json::Value>(expected)?
        } else {
            bytes == expected
        };
        if !matches {
            return Err(invalid(
                "Legacy harbor assets differ; restore them before migrating to a content pack",
            ));
        }
    }
    Ok(())
}

impl World {
    pub fn from_maps(content: Content, mut maps: BTreeMap<String, Map>) -> io::Result<Self> {
        content.validate()?;
        for info in &content.manifest.maps {
            let map = maps
                .get_mut(&info.id)
                .ok_or_else(|| invalid(format!("Missing map {}", info.id)))?;
            content.normalize(map, &info.path)?;
            map.validate(&content, &info.path)?;
        }
        let mut world = Self {
            format: WORLD_FORMAT,
            content,
            revision: String::new(),
            maps,
        };
        world.revision = world.content_hash()?;
        world.validate()?;
        Ok(world)
    }

    pub fn bundled() -> Self {
        let content = Content::bundled();
        let maps = content
            .manifest
            .maps
            .iter()
            .map(|m| {
                (
                    m.id.clone(),
                    serde_json::from_slice(bundled_file(&m.path).unwrap()).unwrap(),
                )
            })
            .collect();
        Self::from_maps(content, maps).expect("Valid bundled world")
    }

    pub fn new(mut town: Map, mut shop: Map) -> io::Result<Self> {
        let content = Content::bundled();
        Self::migrate(&content, &mut town, 0)?;
        Self::migrate(&content, &mut shop, 1)?;
        Self::from_maps(
            content,
            BTreeMap::from([
                ("yapshire:town".into(), town),
                ("yapshire:tackle_shop".into(), shop),
            ]),
        )
    }

    pub fn migrate(content: &Content, map: &mut Map, index: usize) -> io::Result<bool> {
        if map.tilesets.len() != 1 || map.tilesets[0].source != "harbor.tsj" {
            return Ok(false);
        }
        let info = &content.manifest.maps[index];
        if content.manifest.legacy_tiles.len() != 359
            || map.tilesets[0].firstgid != 1
            || map.layers.len() != 5
            || map.layers.iter().any(|l| l.kind != "tilelayer")
            || !matches!(info.id.as_str(), "yapshire:town" | "yapshire:tackle_shop")
        {
            return Err(invalid("This legacy map cannot be migrated automatically"));
        }
        for layer in &mut map.layers {
            for gid in &mut layer.data {
                if *gid == 0 {
                    continue;
                }
                let index = (*gid & GID_MASK)
                    .checked_sub(1)
                    .ok_or_else(|| invalid("Invalid legacy GID"))?
                    as usize;
                let id = content
                    .manifest
                    .legacy_tiles
                    .get(index)
                    .ok_or_else(|| invalid("Unknown legacy tile"))?;
                *gid = content.gid(id).unwrap() | (*gid & !GID_MASK);
            }
        }
        let bundled: Map = serde_json::from_slice(bundled_file(&info.path)?)?;
        map.tilesets = bundled.tilesets;
        map.layers[2].properties = bundled.layers[2].properties.clone();
        map.layers.extend(bundled.layers.into_iter().skip(5));
        map.extra.insert("nextlayerid".into(), 8.into());
        map.extra.insert("nextobjectid".into(), 5.into());
        Ok(true)
    }

    pub fn load(folder: &Path) -> io::Result<Self> {
        if !folder.join("pack.json").exists() {
            if folder.join("harbor.tsj").exists() || folder.join("harbor.png").exists() {
                validate_tileset(folder)?;
            }
            return Self::new(
                serde_json::from_slice(&read_limited(&folder.join("town.tmj"), MAX_MAP_BYTES)?)?,
                serde_json::from_slice(&read_limited(
                    &folder.join("tackle-shop.tmj"),
                    MAX_MAP_BYTES,
                )?)?,
            );
        }
        let content = Content::load(folder)?;
        let maps = content
            .manifest
            .maps
            .iter()
            .map(|m| {
                Ok((
                    m.id.clone(),
                    serde_json::from_slice(&read_limited(
                        &pack_file(folder, &m.path)?,
                        MAX_MAP_BYTES,
                    )?)?,
                ))
            })
            .collect::<io::Result<_>>()?;
        Self::from_maps(content, maps)
    }

    fn content_hash(&self) -> io::Result<String> {
        let bytes = serde_json::to_vec(&(self.format, &self.content, &self.maps))?;
        if bytes.len() > MAX_WORLD_BYTES - 1024 {
            return Err(invalid("World exceeds 8 MiB"));
        }
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    pub fn entry(&self) -> (&str, [f32; 2]) {
        let entry = &self.content.manifest.entry;
        (
            &entry.map,
            self.maps[&entry.map]
                .spawn(&entry.spawn)
                .expect("Validated entry spawn"),
        )
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.format != WORLD_FORMAT {
            return Err(invalid(
                "Unsupported world format; update client and server together",
            ));
        }
        self.content.validate()?;
        if self.maps.len() != self.content.manifest.maps.len() {
            return Err(invalid("World map list differs from its manifest"));
        }
        for info in &self.content.manifest.maps {
            let map = self
                .maps
                .get(&info.id)
                .ok_or_else(|| invalid(format!("Missing map {}", info.id)))?;
            map.validate(&self.content, &info.path)?;
            for object in map.objects().filter(|o| o.kind == "portal") {
                let target = object
                    .property("target_map")
                    .and_then(|id| self.maps.get(id));
                if target
                    .and_then(|m| m.spawn(object.property("target_spawn")?))
                    .is_none()
                {
                    return Err(invalid(format!(
                        "Broken portal {} in {}",
                        object.name, info.id
                    )));
                }
            }
        }
        let entry = &self.content.manifest.entry;
        if self
            .maps
            .get(&entry.map)
            .and_then(|m| m.spawn(&entry.spawn))
            .is_none()
        {
            return Err(invalid("Entry spawn is missing"));
        }
        if self.revision != self.content_hash()? {
            return Err(invalid("World revision does not match its content"));
        }
        Ok(())
    }
}
