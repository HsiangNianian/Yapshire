//! Seeded, bounded sandbox terrain. Coordinates are 16px cells, with Y pointing up.
//! Generator 1 is part of the save format: changing it requires a migration.
use crate::construction::{Catalog, Collision, Layer};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

pub const WIDTH: i32 = 2048;
pub const HEIGHT: i32 = 256;
pub const CHUNK: i32 = 32;
pub const TILE: f32 = 16.0;

fn no_vegetation(version: &u32) -> bool {
    *version == 0
}
fn no_tiles(layers: &[BTreeMap<u32, String>; 3]) -> bool {
    layers.iter().all(BTreeMap::is_empty)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Block {
    #[default]
    Air,
    Dirt,
    Stone,
    Sand,
}

impl Block {
    pub const MATERIALS: [Self; 3] = [Self::Dirt, Self::Stone, Self::Sand];
    pub fn slot(self) -> Option<usize> {
        Self::MATERIALS.iter().position(|&b| b == self)
    }
    pub fn solid(self) -> bool {
        self != Self::Air
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terrain {
    pub format: u32,
    pub generator: u32,
    pub name: String,
    pub seed: u32,
    pub position: [f32; 2],
    pub inventory: [u32; 3],
    pub changes: BTreeMap<u32, Block>,
    /// Version 0 preserves existing worlds without adding vegetation over their builds.
    #[serde(default, skip_serializing_if = "no_vegetation")]
    pub vegetation: u32,
    /// Wall, solid and decoration overrides. Empty IDs erase generated vegetation.
    #[serde(default, skip_serializing_if = "no_tiles")]
    pub tiles: [BTreeMap<u32, String>; 3],
}

impl Terrain {
    pub fn new(name: String, seed: u32) -> Self {
        let mut world = Self {
            format: 2,
            generator: 1,
            name,
            seed,
            position: [0.0; 2],
            inventory: [0; 3],
            changes: BTreeMap::new(),
            vegetation: 2,
            tiles: Default::default(),
        };
        world.position = world.spawn();
        world
    }

    pub fn validate(&self) -> io::Result<()> {
        if !(1..=2).contains(&self.format)
            || self.generator != 1
            || self.vegetation > 2
            || (self.format == 1
                && (self.vegetation != 0 || self.tiles.iter().any(|l| !l.is_empty())))
        {
            return Err(io::Error::other(
                "Unsupported world or terrain generator version",
            ));
        }
        if self.name.trim().is_empty()
            || self.name.chars().count() > 32
            || self.name.chars().any(char::is_control)
            || !self.position.iter().all(|p| p.is_finite())
            || !(6.0..=WIDTH as f32 * TILE - 6.0).contains(&self.position[0])
            || !(0.0..=HEIGHT as f32 * TILE).contains(&self.position[1])
            || self.inventory.iter().any(|&n| n > (WIDTH * HEIGHT) as u32)
            || self.changes.len() > (WIDTH * HEIGHT) as usize
            || self.changes.keys().any(|&i| {
                i >= (WIDTH * HEIGHT) as u32
                    || !Self::editable((i % WIDTH as u32) as i32, (i / WIDTH as u32) as i32)
            })
        {
            return Err(io::Error::other("Invalid world save"));
        }
        for layer in &self.tiles {
            if layer.len() > (WIDTH * HEIGHT) as usize
                || layer.iter().any(|(&key, id)| {
                    key >= (WIDTH * HEIGHT) as u32
                        || !Self::editable(key as i32 % WIDTH, key as i32 / WIDTH)
                        || (!id.is_empty() && !crate::content::valid_id(id))
                })
            {
                return Err(io::Error::other("Invalid construction cells"));
            }
        }
        Ok(())
    }

    pub fn surface(&self, x: i32) -> i32 {
        let rolling = 148.0
            + self.noise(x as f32 / 96.0, 0.0) * 28.0
            + self.noise(x as f32 / 23.0, 3.0) * 8.0;
        let blend = ((x - WIDTH / 2).abs() as f32 - 8.0).clamp(0.0, 24.0) / 24.0;
        (160.0 + (rolling - 160.0) * blend).round() as i32
    }

    pub fn original(&self, x: i32, y: i32) -> Block {
        if !(0..WIDTH).contains(&x) || y < 2 {
            return Block::Stone;
        }
        let surface = self.surface(x);
        if y > surface {
            return Block::Air;
        }
        if y < surface - 6
            && y > 4
            && (self.noise(x as f32 / 18.0, y as f32 / 11.0) > 0.64
                || self.noise(x as f32 / 9.0 + 57.0, y as f32 / 7.0) > 0.79)
        {
            return Block::Air;
        }
        if y < surface - 8 {
            Block::Stone
        } else if self.noise(x as f32 / 160.0, 19.0) > 0.68 {
            Block::Sand
        } else {
            Block::Dirt
        }
    }

    pub fn block(&self, x: i32, y: i32) -> Block {
        if !(0..WIDTH).contains(&x) || !(0..HEIGHT).contains(&y) {
            return if y >= HEIGHT {
                Block::Air
            } else {
                Block::Stone
            };
        }
        self.changes
            .get(&((y * WIDTH + x) as u32))
            .copied()
            .unwrap_or_else(|| self.original(x, y))
    }

    pub fn spawn(&self) -> [f32; 2] {
        let x = WIDTH / 2;
        let y = (2..HEIGHT - 3)
            .rev()
            .find(|&y| {
                self.block(x, y).solid()
                    && !self.block(x, y + 1).solid()
                    && !self.block(x, y + 2).solid()
            })
            .unwrap_or(HEIGHT - 4);
        [(x as f32 + 0.5) * TILE, (y + 1) as f32 * TILE]
    }

    pub fn clear_for_player(&self, position: [f32; 2]) -> bool {
        let [x, y] = position;
        ((x - 5.9) as i32 / 16..=(x + 5.9) as i32 / 16).all(|cx| {
            ((y + 0.1) as i32 / 16..=(y + 25.9) as i32 / 16).all(|cy| !self.block(cx, cy).solid())
        })
    }

    pub fn editable(x: i32, y: i32) -> bool {
        (1..WIDTH - 1).contains(&x) && (2..HEIGHT - 3).contains(&y)
    }

    /// Reach and line of sight are shared by the highlight and edit operation.
    pub fn reachable(&self, x: i32, y: i32, position: [f32; 2]) -> bool {
        if !Self::editable(x, y) || !position.iter().all(|v| v.is_finite()) {
            return false;
        }
        let start = [position[0] / TILE, position[1] / TILE + 1.0];
        let delta = [x as f32 + 0.5 - start[0], y as f32 + 0.5 - start[1]];
        let distance = delta[0].hypot(delta[1]);
        if distance > 6.0 {
            return false;
        }
        // Aim at the nearest exposed face; aiming at a ground tile's centre
        // would incorrectly hit its neighbour when viewed from the side.
        let delta = [
            start[0].clamp(x as f32 - 0.001, x as f32 + 1.001) - start[0],
            start[1].clamp(y as f32 - 0.001, y as f32 + 1.001) - start[1],
        ];
        let steps = (distance * 8.0).ceil().max(1.0) as i32;
        (0..steps).all(|i| {
            let t = i as f32 / steps as f32;
            let cell = (
                (start[0] + delta[0] * t).floor() as i32,
                (start[1] + delta[1] * t).floor() as i32,
            );
            cell == (x, y) || !self.block(cell.0, cell.1).solid()
        })
    }

    fn set(&mut self, x: i32, y: i32, block: Block) {
        let key = (y * WIDTH + x) as u32;
        if block == self.original(x, y) {
            self.changes.remove(&key);
        } else {
            self.changes.insert(key, block);
        }
    }

    pub fn dig(&mut self, x: i32, y: i32, position: [f32; 2]) -> bool {
        let Some(slot) = self.block(x, y).slot() else {
            return false;
        };
        if !self.reachable(x, y, position) {
            return false;
        }
        self.set(x, y, Block::Air);
        self.inventory[slot] += 1;
        true
    }

    pub fn place(
        &mut self,
        x: i32,
        y: i32,
        block: Block,
        position: [f32; 2],
        catalog: &Catalog,
    ) -> bool {
        let Some(slot) = block.slot() else {
            return false;
        };
        if self.inventory[slot] == 0
            || !self.cell(Layer::Solid, x, y, catalog).is_empty()
            || !self.construction_reachable(x, y, position, catalog)
            || ![(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                .iter()
                .any(|&(cx, cy)| self.collision(cx, cy, catalog) != Collision::None)
            || (position[0] + 6.0 > x as f32 * TILE
                && position[0] - 6.0 < (x + 1) as f32 * TILE
                && position[1] + 26.0 > y as f32 * TILE
                && position[1] < (y + 1) as f32 * TILE)
        {
            return false;
        }
        self.set(x, y, block);
        self.inventory[slot] -= 1;
        true
    }

    pub fn validate_catalog(&self, catalog: &Catalog) -> io::Result<()> {
        self.validate()?;
        let required = [
            "yapshire:earth",
            "yapshire:grass",
            "yapshire:rock",
            "yapshire:sand",
        ];
        for id in required.into_iter().chain(
            self.tiles
                .iter()
                .flat_map(|l| l.values().map(String::as_str)),
        ) {
            if !id.is_empty() && catalog.get(id).is_none() {
                return Err(io::Error::other(format!(
                    "World requires missing tile: {id}"
                )));
            }
        }
        if self.vegetation == 1 {
            for id in [
                "yapshire:trunk",
                "yapshire:birch",
                "yapshire:leaves/15",
                "yapshire:gold_leaves/15",
                "yapshire:bush/15",
                "yapshire:flowers",
                "yapshire:tuft",
            ] {
                if catalog.get(id).is_none() {
                    return Err(io::Error::other(format!(
                        "World requires missing tile: {id}"
                    )));
                }
            }
        }
        if self.vegetation == 2 {
            for name in ["yapshire:tuft", "yapshire:flowers"] {
                if catalog.get(name).is_none() {
                    return Err(io::Error::other(format!(
                        "World requires missing tile: {name}"
                    )));
                }
            }
            for (name, width, height) in [
                ("yapshire:cedar_tree/v2", 7, 12),
                ("yapshire:birch_tree/v2", 7, 12),
                ("yapshire:shrub/v2", 4, 2),
                ("yapshire:autumn_shrub/v2", 4, 2),
            ] {
                if !catalog.prefabs.get(name).is_some_and(|p| {
                    p.width == width
                        && p.height == height
                        && p.tiles.iter().any(|id| !id.is_empty())
                        && p.tiles
                            .iter()
                            .all(|id| id.is_empty() || catalog.get(id).is_some())
                }) {
                    return Err(io::Error::other(format!(
                        "World requires woodland recipe: {name}"
                    )));
                }
            }
        }
        if self.tiles[Layer::Solid.index()].iter().any(|(&key, id)| {
            id.is_empty() || self.block(key as i32 % WIDTH, key as i32 / WIDTH).solid()
        }) {
            return Err(io::Error::other("Construction overlaps solid terrain"));
        }
        Ok(())
    }

    fn vegetation_tile<'a>(&self, layer: Layer, x: i32, y: i32, catalog: &'a Catalog) -> &'a str {
        if self.vegetation == 0 || layer == Layer::Solid || !(1..WIDTH - 1).contains(&x) {
            return "";
        }
        let region = x.div_euclid(18);
        let hash = (region as u32)
            .wrapping_mul(2654435761)
            .wrapping_add(self.seed.wrapping_mul(2246822519));
        let centre = region * 18 + 5 + (hash % 7) as i32;
        let base = self.surface(centre);
        let birch = hash & 1 == 0;
        if self.vegetation == 2 {
            if y <= self.surface(x) {
                return "";
            }
            if (centre - WIDTH / 2).abs() > 9 {
                let id = catalog.prefab_cell(
                    if birch {
                        "yapshire:birch_tree/v2"
                    } else {
                        "yapshire:cedar_tree/v2"
                    },
                    layer,
                    x - centre + 3,
                    base + 12 - y,
                );
                if !id.is_empty() {
                    return id;
                }
            }
            let shrub = x.div_euclid(13) * 13 + 5;
            if (shrub - WIDTH / 2).abs() > 7 {
                let id = catalog.prefab_cell(
                    if (shrub as u32 + self.seed % 3).is_multiple_of(3) {
                        "yapshire:autumn_shrub/v2"
                    } else {
                        "yapshire:shrub/v2"
                    },
                    layer,
                    x - shrub + 2,
                    self.surface(shrub) + 2 - y,
                );
                if !id.is_empty() {
                    return id;
                }
            }
            if layer == Layer::Decor && (x - WIDTH / 2).abs() > 6 && y == self.surface(x) + 1 {
                return if (x + (self.seed % 11) as i32).rem_euclid(11) == 0 {
                    "yapshire:flowers"
                } else if x.rem_euclid(3) == 0 {
                    "yapshire:tuft"
                } else {
                    ""
                };
            }
            return "";
        }
        // Keep version 1 geometry for worlds saved before the detailed artwork.
        let widths: &[i32] = if birch {
            &[3, 5, 7, 7, 5, 3]
        } else {
            &[1, 3, 3, 5, 3, 5, 7, 5, 7]
        };
        if (centre - WIDTH / 2).abs() > 9 && x.abs_diff(centre) <= 3 {
            if layer == Layer::Wall && x == centre && (base + 1..=base + 3).contains(&y) {
                return if birch {
                    "yapshire:birch"
                } else {
                    "yapshire:trunk"
                };
            }
            let row = base + 3 + widths.len() as i32 - y;
            if layer == Layer::Decor
                && row >= 0
                && row < widths.len() as i32
                && (x - centre).abs() <= widths[row as usize] / 2
                && y > self.surface(x)
            {
                return if birch {
                    "yapshire:gold_leaves/15"
                } else {
                    "yapshire:leaves/15"
                };
            }
        }
        if layer == Layer::Decor && (x - WIDTH / 2).abs() > 6 {
            let base = self.surface(x);
            if (x + (self.seed % 9) as i32).rem_euclid(13) < 3 && (base + 1..=base + 2).contains(&y)
            {
                return "yapshire:bush/15";
            }
            if y == base + 1 {
                if (x + (self.seed % 11) as i32).rem_euclid(11) == 0 {
                    return "yapshire:flowers";
                }
                if x.rem_euclid(3) == 0 {
                    return "yapshire:tuft";
                }
            }
        }
        ""
    }

    pub fn cell<'a>(&'a self, layer: Layer, x: i32, y: i32, catalog: &'a Catalog) -> &'a str {
        if !(0..WIDTH).contains(&x) || !(0..HEIGHT).contains(&y) {
            return "";
        }
        if let Some(id) = self.tiles[layer.index()].get(&((y * WIDTH + x) as u32)) {
            return id;
        }
        if layer == Layer::Solid {
            return match self.block(x, y) {
                Block::Dirt if self.block(x, y + 1) == Block::Air => "yapshire:grass",
                Block::Dirt => "yapshire:earth",
                Block::Stone => "yapshire:rock",
                Block::Sand => "yapshire:sand",
                Block::Air => "",
            };
        }
        self.vegetation_tile(layer, x, y, catalog)
    }

    pub fn collision(&self, x: i32, y: i32, catalog: &Catalog) -> Collision {
        if self.block(x, y).solid() {
            return Collision::Solid;
        }
        catalog
            .get(self.cell(Layer::Solid, x, y, catalog))
            .map_or(Collision::None, |piece| piece.collision)
    }

    pub fn construction_spawn(&self, catalog: &Catalog) -> [f32; 2] {
        let x = WIDTH / 2;
        let y = (2..HEIGHT - 3)
            .rev()
            .find(|&y| {
                self.collision(x, y, catalog) != Collision::None
                    && self.collision(x, y + 1, catalog) == Collision::None
                    && self.collision(x, y + 2, catalog) == Collision::None
            })
            .unwrap_or(HEIGHT - 4);
        [(x as f32 + 0.5) * TILE, (y + 1) as f32 * TILE]
    }

    pub fn construction_clear(&self, position: [f32; 2], catalog: &Catalog) -> bool {
        let [x, y] = position;
        ((x - 5.9) as i32 / 16..=(x + 5.9) as i32 / 16).all(|cx| {
            ((y + 0.1) as i32 / 16..=(y + 25.9) as i32 / 16)
                .all(|cy| self.collision(cx, cy, catalog) != Collision::Solid)
        })
    }

    pub fn construction_reachable(
        &self,
        x: i32,
        y: i32,
        position: [f32; 2],
        catalog: &Catalog,
    ) -> bool {
        if !self.reachable(x, y, position) {
            return false;
        }
        let start = [position[0] / TILE, position[1] / TILE + 1.0];
        let end = [
            start[0].clamp(x as f32 - 0.001, x as f32 + 1.001),
            start[1].clamp(y as f32 - 0.001, y as f32 + 1.001),
        ];
        (0..48).all(|step| {
            let t = step as f32 / 48.0;
            let (cx, cy) = (
                (start[0] + (end[0] - start[0]) * t).floor() as i32,
                (start[1] + (end[1] - start[1]) * t).floor() as i32,
            );
            (cx, cy) == (x, y) || self.collision(cx, cy, catalog) != Collision::Solid
        })
    }

    /// Construction is creative: pack tiles are reusable without an economy dependency.
    pub fn place_tile(
        &mut self,
        layer: Layer,
        x: i32,
        y: i32,
        id: &str,
        position: [f32; 2],
        catalog: &Catalog,
    ) -> bool {
        if catalog.get(id).is_none()
            || !self.cell(layer, x, y, catalog).is_empty()
            || !self.construction_reachable(x, y, position, catalog)
            || (layer == Layer::Solid
                && catalog
                    .get(id)
                    .is_some_and(|piece| piece.collision != Collision::None)
                && position[0] + 6.0 > x as f32 * TILE
                && position[0] - 6.0 < (x + 1) as f32 * TILE
                && position[1] + 26.0 > y as f32 * TILE
                && position[1] < (y + 1) as f32 * TILE)
        {
            return false;
        }
        self.format = 2;
        self.tiles[layer.index()].insert((y * WIDTH + x) as u32, id.into());
        true
    }

    pub fn dig_tile(
        &mut self,
        layer: Layer,
        x: i32,
        y: i32,
        position: [f32; 2],
        catalog: &Catalog,
    ) -> bool {
        if self.cell(layer, x, y, catalog).is_empty()
            || !self.construction_reachable(x, y, position, catalog)
        {
            return false;
        }
        if layer == Layer::Solid && self.block(x, y).solid() {
            return self.dig(x, y, position);
        }
        self.format = 2;
        let key = (y * WIDTH + x) as u32;
        if self.vegetation_tile(layer, x, y, catalog).is_empty() {
            self.tiles[layer.index()].remove(&key);
        } else {
            self.tiles[layer.index()].insert(key, String::new());
        }
        true
    }

    /// Connected foliage uses neighbouring cells across chunk boundaries, including edits.
    pub fn visual_cell<'a>(
        &'a self,
        layer: Layer,
        x: i32,
        y: i32,
        catalog: &'a Catalog,
    ) -> &'a str {
        let id = self.cell(layer, x, y, catalog);
        let Some(family) = catalog.get(id).and_then(|p| p.family.as_ref()) else {
            return id;
        };
        let Some(variants) = catalog.terrains.get(family) else {
            return id;
        };
        let mut mask = 0;
        for (dx, dy, bit) in [(0, 1, 1), (1, 0, 2), (0, -1, 4), (-1, 0, 8)] {
            if catalog
                .get(self.cell(layer, x + dx, y + dy, catalog))
                .and_then(|p| p.family.as_ref())
                == Some(family)
            {
                mask |= bit;
            }
        }
        &variants[mask]
    }

    fn noise(&self, x: f32, y: f32) -> f32 {
        let (ix, iy) = (x.floor() as i32, y.floor() as i32);
        let smooth = |n: f32| n * n * (3.0 - 2.0 * n);
        let (tx, ty) = (smooth(x - x.floor()), smooth(y - y.floor()));
        let sample = |x: i32, y: i32| {
            let mut n = (self.seed as u64)
                ^ (x as u64).wrapping_mul(0x9e3779b97f4a7c15)
                ^ (y as u64).wrapping_mul(0xbf58476d1ce4e5b9);
            n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
            ((n ^ (n >> 31)) >> 40) as f32 / 16777215.0
        };
        let a = sample(ix, iy) * (1.0 - tx) + sample(ix + 1, iy) * tx;
        let b = sample(ix, iy + 1) * (1.0 - tx) + sample(ix + 1, iy + 1) * tx;
        a * (1.0 - ty) + b * ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generator_one_keeps_existing_saves_geographically_stable() {
        use sha2::{Digest, Sha256};
        let world = Terrain::new("World".into(), 42);
        let mut hash = Sha256::new();
        for x in (0..WIDTH).step_by(13) {
            for y in (0..HEIGHT).step_by(7) {
                hash.update([world.block(x, y) as u8]);
            }
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "688be7609a7d7ada6dcce812264a312c4f8b96fd445f086e899876afa94148cf",
            "Generator 1 is persisted in existing saves; changing it requires a migration"
        );
    }
    #[test]
    fn seed_is_repeatable_and_spawn_is_clear_above_solid_ground() {
        for seed in [0, 42, u32::MAX] {
            let a = Terrain::new("World".into(), seed);
            let b = Terrain::new("World".into(), seed);
            a.validate().unwrap();
            assert!(a.clear_for_player(a.position));
            assert!(
                a.block(WIDTH / 2, (a.position[1] / TILE) as i32 - 1)
                    .solid()
            );
            for x in (0..WIDTH).step_by(17) {
                for y in (0..HEIGHT).step_by(13) {
                    assert_eq!(a.block(x, y), b.block(x, y));
                }
            }
        }
        let a = Terrain::new("World".into(), 41);
        let b = Terrain::new("World".into(), 42);
        assert!((0..WIDTH).any(|x| a.surface(x) != b.surface(x)));
        assert!((5..140).any(|y| (0..WIDTH).step_by(11).any(|x| a.block(x, y) == Block::Air)));
    }
    #[test]
    fn edits_conserve_materials_and_survive_serialization() {
        let catalog = Catalog::new(&crate::content::Content::bundled());
        let mut world = Terrain::new("World".into(), 42);
        let p = world.position;
        let (x, y) = (WIDTH / 2 + 2, 160);
        let block = world.block(x, y);
        assert!(world.dig(x, y, p));
        assert!(!world.dig(x, y, p));
        let bytes = serde_json::to_vec(&world).unwrap();
        let mut restored: Terrain = serde_json::from_slice(&bytes).unwrap();
        restored.validate().unwrap();
        assert_eq!(world, restored);
        assert!(restored.place(x, y, block, p, &catalog));
        assert_eq!(restored.inventory, [0; 3]);
        assert!(restored.changes.is_empty());
    }
    #[test]
    fn edits_reject_walls_distance_bedrock_and_player_overlap() {
        let catalog = Catalog::new(&crate::content::Content::bundled());
        let mut world = Terrain::new("World".into(), 42);
        let p = world.position;
        assert!(!world.dig(WIDTH / 2 + 30, 160, p));
        assert!(!world.dig(WIDTH / 2 + 2, 155, p));
        assert!(!world.dig(WIDTH / 2, 1, [p[0], 32.0]));
        assert!(!world.place(WIDTH / 2 + 2, 161, Block::Dirt, p, &catalog));
        assert!(world.dig(WIDTH / 2 + 2, 160, p));
        let block = Block::MATERIALS[world.inventory.iter().position(|&n| n > 0).unwrap()];
        assert!(!world.place(WIDTH / 2, 161, block, p, &catalog));
        assert!(!world.place(WIDTH / 2 + 2, 164, block, p, &catalog));
        assert_eq!(world.inventory.iter().sum::<u32>(), 1);
        world.format = 3;
        assert!(world.validate().is_err());
    }
}
