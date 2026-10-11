use crate::i18n::{Message, tr};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use std::{
    io,
    path::{Path, PathBuf},
};
pub(crate) use yapshire_shared::maps::Map;
use yapshire_shared::{
    World,
    content::{Content, relative},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Component)]
pub(crate) struct MapKind(pub usize);
#[allow(non_upper_case_globals)]
impl MapKind {
    pub const Town: Self = Self(0);
    #[cfg(any(debug_assertions, test))]
    pub const Shop: Self = Self(1);
    pub fn index(self) -> usize {
        self.0
    }
}
#[derive(Component)]
pub(crate) struct MapLayer;

#[derive(Resource)]
pub(crate) struct Maps {
    pub world: World,
    pub pack_path: PathBuf,
    pub revision: u64,
    prefix: String,
}
impl Maps {
    pub fn kinds(&self) -> impl Iterator<Item = MapKind> + use<> {
        (0..self.world.content.manifest.maps.len()).map(MapKind)
    }
    pub fn info(&self, kind: MapKind) -> &yapshire_shared::content::MapInfo {
        &self.world.content.manifest.maps[kind.0]
    }
    pub fn filename(&self, kind: MapKind) -> &str {
        &self.info(kind).path
    }
    pub fn title(&self, kind: MapKind) -> Message {
        match self.info(kind).id.as_str() {
            "yapshire:town" => tr("editor.town_title"),
            "yapshire:tackle_shop" => tr("editor.shop_title"),
            _ => self.info(kind).title.clone().into(),
        }
    }
    pub fn world(&self) -> io::Result<World> {
        self.world
            .content
            .require(&Content::load(&self.pack_path)?)?;
        World::from_maps(self.world.content.clone(), self.world.maps.clone())
    }
    pub fn apply_world(&mut self, world: &World) {
        if self.world != *world {
            self.world = world.clone();
            self.revision += 1;
        }
    }
    pub fn get(&self, kind: MapKind) -> &Map {
        &self.world.maps[&self.info(kind).id]
    }
    pub fn by_id(&self, id: &str) -> Option<&Map> {
        self.world.maps.get(id)
    }
    pub fn indoors(&self, id: &str) -> bool {
        self.world
            .content
            .manifest
            .maps
            .iter()
            .find(|m| m.id == id)
            .is_some_and(|m| m.indoors)
    }
    pub fn replace(&mut self, kind: MapKind, map: Map) {
        if self.get(kind) != &map {
            self.world.maps.insert(self.info(kind).id.clone(), map);
            self.revision += 1;
        }
    }
    pub fn tile_count(&self) -> u32 {
        self.world.content.sets.values().map(|s| s.tilecount).sum()
    }
    pub fn validate_map(&self, kind: MapKind, map: &Map) -> io::Result<()> {
        map.validate(&self.world.content, self.filename(kind))?;
        let entry = &self.world.content.manifest.entry;
        if self.info(kind).id == entry.map && map.spawn(&entry.spawn).is_none() {
            return Err(io::Error::other("Entry spawn is missing"));
        }
        Ok(())
    }
    pub fn prepare_map(&self, kind: MapKind, map: &mut Map) -> io::Result<()> {
        World::migrate(&self.world.content, map, kind.0)?;
        self.world.content.normalize(map, self.filename(kind))?;
        self.validate_map(kind, map)
    }
    pub fn load(assets: &Path) -> io::Result<Self> {
        let pack = std::env::var("YAPSHIRE_PACK").unwrap_or_else(|_| "yapshire".into());
        if !yapshire_shared::content::valid_id(&format!("{pack}:pack")) {
            return Err(io::Error::other("Invalid YAPSHIRE_PACK name"));
        }
        let prefix = format!("packs/{pack}");
        let pack_path = assets.join(&prefix);
        let world = World::load(&pack_path)?;
        Ok(Self {
            world,
            pack_path,
            prefix,
            revision: 0,
        })
    }
    pub fn image_path(&self, path: &str) -> String {
        format!("{}/{path}", self.prefix)
    }
    pub fn tile(
        &self,
        gid: u32,
    ) -> Option<(
        &str,
        &yapshire_shared::content::Tileset,
        &yapshire_shared::content::Tile,
    )> {
        self.world
            .content
            .resolve(self.get(MapKind(0)), self.filename(MapKind(0)), gid)
    }
    pub fn tile_image(&self, gid: u32, assets: &AssetServer) -> (Handle<Image>, Rect) {
        let Some((source, set, tile)) = self.tile(gid) else {
            return (Handle::default(), Rect::default());
        };
        let x = (tile.id % set.columns * 16) as f32;
        let y = (tile.id / set.columns * 16) as f32;
        (
            assets.load(self.image_path(&relative(source, &set.image).unwrap())),
            Rect::new(x, y, x + 16.0, y + 16.0),
        )
    }
    pub fn spawn(
        &self,
        commands: &mut Commands,
        assets: &AssetServer,
        parent: Entity,
        kind: MapKind,
    ) {
        let map = self.get(kind).expanded(&self.world.content);
        let size = TilemapSize {
            x: map.width,
            y: map.height,
        };
        let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
        for (depth, layer) in map.layers.iter().enumerate() {
            let z = layer.z(depth);
            let visibility = if layer.visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if let Some(image) = &layer.image {
                let path = relative(self.filename(kind), image).unwrap();
                commands.spawn((
                    MapLayer,
                    Sprite {
                        image: assets.load(self.image_path(&path)),
                        color: Color::WHITE.with_alpha(layer.opacity),
                        ..default()
                    },
                    bevy::sprite::Anchor::TOP_LEFT,
                    Transform::from_xyz(layer.offsetx, map.origin_y() - layer.offsety, z),
                    visibility,
                    ChildOf(parent),
                ));
                continue;
            }
            if layer.data.is_empty() {
                continue;
            }
            for (source, set) in &self.world.content.sets {
                let entity = commands
                    .spawn((MapLayer, Name::new(layer.name.clone()), ChildOf(parent)))
                    .id();
                let mut storage = TileStorage::empty(size);
                for (i, &gid) in layer.data.iter().enumerate().filter(|(_, g)| **g != 0) {
                    let Some((resolved, _, definition)) =
                        self.world.content.resolve(&map, self.filename(kind), gid)
                    else {
                        continue;
                    };
                    if resolved != source {
                        continue;
                    }
                    let pos = TilePos {
                        x: i as u32 % map.width,
                        y: map.height - 1 - i as u32 / map.width,
                    };
                    let mut tile = commands.spawn((
                        TileBundle {
                            position: pos,
                            tilemap_id: TilemapId(entity),
                            texture_index: TileTextureIndex(definition.id),
                            flip: TileFlip {
                                x: gid & 0x8000_0000 != 0,
                                y: gid & 0x4000_0000 != 0,
                                d: gid & 0x2000_0000 != 0,
                            },
                            color: TileColor(Color::WHITE.with_alpha(layer.opacity)),
                            ..default()
                        },
                        ChildOf(entity),
                    ));
                    if let Some(first) = definition.animation.first() {
                        tile.insert(AnimatedTile {
                            start: first.tileid,
                            end: first.tileid + definition.animation.len() as u32,
                            speed: 1000.0
                                / (first.duration as f32 * definition.animation.len() as f32),
                        });
                    }
                    storage.set(&pos, tile.id());
                }
                commands.entity(entity).insert(TilemapBundle {
                    grid_size: tile_size.into(),
                    map_type: TilemapType::Square,
                    size,
                    storage,
                    texture: TilemapTexture::Single(
                        assets.load(self.image_path(&relative(source, &set.image).unwrap())),
                    ),
                    tile_size,
                    transform: Transform::from_xyz(8.0, -56.0, z),
                    visibility,
                    ..default()
                });
            }
        }
    }
}

pub(crate) fn layer_title(name: &str) -> Message {
    let key = match name {
        "Water" => "editor.layer.water",
        "Shore and pilings" => "editor.layer.shore",
        "Terrain" => "editor.layer.terrain",
        "Buildings" => "editor.layer.buildings",
        "Props" => "editor.layer.props",
        "Backdrop" => "editor.layer.backdrop",
        "Walls" => "editor.layer.walls",
        "Floor" => "editor.layer.floor",
        "Furniture" => "editor.layer.furniture",
        "Counter" => "editor.layer.counter",
        _ => return name.into(),
    };
    tr(key)
}
