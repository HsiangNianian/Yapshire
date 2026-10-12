//! Material cells use Tiled pixel coordinates (Y down). Only the room server steps them.
use crate::{
    World,
    blocks::Block,
    content::{Content, invalid},
    maps::{Layer, Map, Object, bool_property, string_property},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

pub const CHUNK: u32 = 32;
pub const MAX_PIXELS: usize = 512 * 1024;
pub const REACH: f32 = 48.0;
pub const RADIUS: i32 = 5;
pub const TICK_MS: u64 = 50;
const HIDDEN: u32 = 1 << 26;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    Solid = 1,
    Sand = 2,
    Water = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Grid {
        let world = World::bundled();
        let map: Map = serde_json::from_value(serde_json::json!({
            "width":4,"height":4,"tilewidth":16,"tileheight":16,
            "orientation":"orthogonal","infinite":false,"tilesets":[],"layers":[]
        }))
        .unwrap();
        Grid::from_map(&map, &world.content, "maps/empty.tmj").unwrap()
    }
    fn counts(grid: &Grid) -> [usize; 4] {
        let mut result = [0; 4];
        for v in &grid.cells {
            result[(v & 3) as usize] += 1;
        }
        result
    }
    #[test]
    fn png_colors_alpha_and_model_flips_preserve_irregular_pixel_shapes() {
        let mut world = World::bundled();
        let block = world
            .content
            .blocks
            .values()
            .find(|b| b.tile && b.material == Some(Material::Solid))
            .unwrap()
            .clone();
        let model = world.content.models.get_mut(&block.model).unwrap();
        model.anchor = [0, 0];
        model.parts.truncate(1);
        let part = &mut model.parts[0];
        part.offset = [0, 0];
        part.size = [16, 16];
        part.flip_x = false;
        let texture = part.texture.clone();
        let mut colors = vec![0u32; 256];
        colors[3 * 16 + 2] = 0x123456ff;
        colors[5 * 16 + 4] = 0xabcdef80;
        colors[7 * 16 + 6] = 0xffffff7f;
        let raw: Vec<_> = colors.iter().flat_map(|v| v.to_be_bytes()).collect();
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 16, 16);
            encoder.set_color(png::ColorType::Rgba);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&raw)
                .unwrap();
        }
        let source = Source::read(&png).unwrap();
        assert_eq!(source.colors(), colors);
        world.content.pixels.insert(texture.clone(), source);
        let sources = BTreeMap::from([(texture.as_str(), colors)]);
        let mut grid = empty();
        grid.stamp(&block, &world.content, &sources, [0, 0], 0, true);
        grid.stamp(&block, &world.content, &sources, [16, 0], 0x8000_0000, true);
        grid.stamp(
            &block,
            &world.content,
            &sources,
            [0, 16],
            0xe000_0000,
            false,
        );
        assert_eq!(grid.get(2, 3), 0x123456 << 2 | 1);
        assert_eq!(grid.get(4, 5), 0xabcdef << 2 | 1);
        assert_eq!(grid.get(6, 7), 0);
        assert_eq!(grid.get(29, 3), grid.get(2, 3));
        assert_eq!(grid.get(12, 29), grid.get(2, 3) | HIDDEN);
        assert_eq!(counts(&grid)[1], 6);
    }
    #[test]
    fn malformed_sources_and_excessive_simulation_work_are_rejected() {
        assert!(Source::read(b"not a png").is_err());
        for source in [
            Source {
                width: u32::MAX,
                height: u32::MAX,
                runs: vec![],
            },
            Source {
                width: 1,
                height: 1,
                runs: vec![[0, 0]],
            },
            Source {
                width: 1,
                height: 1,
                runs: vec![[2, 0]],
            },
        ] {
            assert!(source.validate().is_err());
        }
        let mut world = World::bundled();
        let mut huge = world.maps["yapshire:town"].clone();
        huge.width = u32::MAX;
        assert!(Grid::from_map(&huge, &world.content, "maps/town.tmj").is_err());
        let town = world.maps.get_mut("yapshire:town").unwrap();
        town.width = 256;
        town.height = 128;
        assert!(validate_world(&world).is_err());
        let mut world = World::bundled();
        for block in world
            .content
            .blocks
            .values()
            .filter(|b| b.material.is_some())
        {
            for part in &mut world.content.models.get_mut(&block.model).unwrap().parts {
                part.size = [512, 512];
            }
        }
        assert!(validate_world(&world).is_err());
    }
    #[test]
    fn particles_cross_chunks_once_per_tick_and_conserve_material_and_color() {
        let mut grid = empty();
        grid.set(31, 31, 0xc4a46c << 2 | 2);
        grid.step();
        assert_eq!(grid.get(31, 32) & 3, 2);
        assert_eq!(grid.get(31, 33), 0);
        grid.paint(32, 12, Tool::Sand);
        grid.paint(31, 54, Tool::Water);
        let before = counts(&grid);
        let mut colors = grid.cells.clone();
        colors.sort_unstable();
        for _ in 0..220 {
            grid.step();
        }
        assert_eq!(counts(&grid), before);
        let mut after = grid.cells.clone();
        after.sort_unstable();
        assert_eq!(after, colors);
        assert!(grid.cells[..32 * 64].iter().all(|v| *v == 0));
        assert!(grid.cells[63 * 64..].iter().any(|v| v & 3 == 2));
    }
    #[test]
    fn water_spreads_over_a_floor_and_sand_sinks_through_water() {
        let mut grid = empty();
        for x in 0..64 {
            grid.set(x, 50, 0x778899 << 2 | 1);
        }
        grid.paint(32, 24, Tool::Water);
        let water = counts(&grid)[3];
        for _ in 0..120 {
            grid.step();
        }
        assert_eq!(counts(&grid)[3], water);
        let bottom: Vec<_> = (0..64).filter(|x| grid.get(*x, 49) & 3 == 3).collect();
        assert!(
            bottom.len() >= 32 && bottom[0] < 10 && *bottom.last().unwrap() > 54,
            "Water must spread across the basin: {bottom:?}"
        );
        grid.set(32, 48, 0xc4a46c << 2 | 2);
        let before = counts(&grid);
        grid.step();
        assert_eq!(grid.get(32, 49) & 3, 2);
        assert_eq!(counts(&grid), before);
        assert!(grid.cells[51 * 64..].iter().all(|v| *v == 0));
    }
    #[test]
    fn digging_changes_collision_and_wakes_settled_grains_across_a_chunk_boundary() {
        let mut grid = empty();
        for x in 0..64 {
            grid.set(x, 32, 0x778899 << 2 | 1);
        }
        grid.set(31, 31, 0xc4a46c << 2 | 2);
        for _ in 0..5 {
            grid.step();
        }
        assert!(grid.active.iter().all(|a| !a));
        let before = grid.surfaces([20.0, -40.0], [40.0, 0.0]);
        assert!(grid.paint(31, 35, Tool::Dig));
        assert_ne!(grid.surfaces([20.0, -40.0], [40.0, 0.0]), before);
        // Leave a grain immediately above the drilled opening.
        grid.set(31, 31, 0xc4a46c << 2 | 2);
        for _ in 0..12 {
            grid.step();
        }
        assert!((40..64).any(|y| grid.get(31, y) & 3 == 2));
        assert!(!grid.solid(31, 32));
    }
    #[test]
    fn snapshots_and_live_chunks_reconstruct_identical_pixels_and_reject_bad_packets_atomically() {
        let mut host = empty();
        let mut peer = host.clone();
        host.clear_dirty();
        peer.clear_dirty();
        host.paint(31, 8, Tool::Sand);
        host.paint(46, 24, Tool::Water);
        for _ in 0..60 {
            host.step();
            if let Some(update) = host.take_update("test:map") {
                peer.apply(&update).unwrap();
            }
            assert_eq!(peer.cells, host.cells);
        }
        let mut late = empty();
        late.apply(&host.snapshot("test:map")).unwrap();
        assert_eq!(late.cells, host.cells);
        let before = late.cells.clone();
        for chunks in [
            vec![Patch {
                index: 0,
                runs: vec![[0, 0]],
            }],
            vec![Patch {
                index: 999,
                runs: vec![[1024, 0]],
            }],
            vec![Patch {
                index: 0,
                runs: vec![[1024, 4]],
            }],
            vec![
                Patch {
                    index: 0,
                    runs: vec![[1024, 0]],
                },
                Patch {
                    index: 0,
                    runs: vec![[1024, 0]],
                },
            ],
            vec![Patch {
                index: 0,
                runs: vec![[1024, HIDDEN * 2 | 1]],
            }],
        ] {
            assert!(
                late.apply(&Update {
                    map: "test:map".into(),
                    tick: host.tick + 1,
                    chunks
                })
                .is_err()
            );
            assert_eq!(late.cells, before);
        }
    }
    #[test]
    fn stock_templates_keep_texture_pixels_and_static_platforms_block_fluid_without_becoming_diggable()
     {
        let world = World::bundled();
        let mut grid = build(&world).unwrap().remove("yapshire:town").unwrap();
        assert!(grid.solid(244, 210));
        let before = grid.get(244, 210);
        assert_ne!(before >> 2, 0);
        grid.paint(244, 211, Tool::Dig);
        assert_eq!(grid.get(244, 210), 0);
        assert!(
            grid.collision
                .tile_surfaces(&world.content, "maps/town.tmj", 15, 13)
                .is_empty()
        );
        let pier = grid.index(1280, 208).unwrap();
        assert!(grid.blocked[pier]);
        grid.paint(1280, 208, Tool::Sand);
        assert_eq!(grid.cells[pier], 0);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    #[default]
    Dig,
    Sand,
    Water,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub width: u32,
    pub height: u32,
    pub runs: Vec<[u32; 2]>,
}

fn encode(values: impl Iterator<Item = u32>) -> Vec<[u32; 2]> {
    let mut runs: Vec<[u32; 2]> = Vec::new();
    for value in values {
        if let Some(last) = runs.last_mut().filter(|r| r[1] == value) {
            last[0] += 1;
        } else {
            runs.push([1, value]);
        }
    }
    runs
}
fn valid_runs(runs: &[[u32; 2]], size: usize) -> bool {
    runs.len() <= size
        && runs.iter().all(|r| r[0] > 0 && r[0] as usize <= size)
        && runs.iter().map(|r| r[0] as u64).sum::<u64>() == size as u64
}
impl Source {
    pub fn read(bytes: &[u8]) -> io::Result<Self> {
        let mut decoder = png::Decoder::new(io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|e| invalid(e.to_string()))?;
        let (width, height) = (reader.info().width, reader.info().height);
        if width == 0 || height == 0 || width as u64 * height as u64 > MAX_PIXELS as u64 {
            return Err(invalid("Material texture is too large"));
        }
        let mut buffer = vec![
            0;
            reader
                .output_buffer_size()
                .ok_or_else(|| invalid("Invalid PNG buffer size"))?
        ];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|e| invalid(e.to_string()))?;
        let channels = info.color_type.samples();
        let mut values = Vec::new();
        for p in buffer[..info.buffer_size()].chunks_exact(channels) {
            let rgba = match info.color_type {
                png::ColorType::Rgba => [p[0], p[1], p[2], p[3]],
                png::ColorType::Rgb => [p[0], p[1], p[2], 255],
                png::ColorType::Grayscale => [p[0], p[0], p[0], 255],
                png::ColorType::GrayscaleAlpha => [p[0], p[0], p[0], p[1]],
                _ => return Err(invalid("Unsupported material PNG")),
            };
            values.push(u32::from_be_bytes(rgba));
        }
        Ok(Self {
            width,
            height,
            runs: encode(values.into_iter()),
        })
    }
    pub fn validate(&self) -> io::Result<()> {
        let size = self.width as u64 * self.height as u64;
        if self.width == 0
            || self.height == 0
            || size > MAX_PIXELS as u64
            || !valid_runs(&self.runs, size as usize)
        {
            return Err(invalid("Invalid material texture pixels"));
        }
        Ok(())
    }
    fn colors(&self) -> Vec<u32> {
        self.runs
            .iter()
            .flat_map(|r| std::iter::repeat_n(r[1], r[0] as usize))
            .collect()
    }
}

pub fn tile<'a>(
    layer: &Layer,
    map: &Map,
    content: &'a Content,
    path: &str,
    gid: u32,
) -> Option<&'a Block> {
    if !bool_property(&layer.properties, "simulation") {
        return None;
    }
    let (_, _, tile) = content.resolve(map, path, gid)?;
    let block = content
        .blocks
        .get(string_property(&tile.properties, "block")?)?;
    block.material.map(|_| block)
}
pub fn object<'a>(layer: &Layer, object: &Object, content: &'a Content) -> Option<&'a Block> {
    if !bool_property(&layer.properties, "simulation") {
        return None;
    }
    let block = object.block(content)?;
    block.material.map(|_| block)
}
pub fn enabled(map: &Map) -> bool {
    map.layers
        .iter()
        .any(|l| bool_property(&l.properties, "simulation"))
}
pub fn validate_world(world: &World) -> io::Result<()> {
    let count: u64 = world
        .maps
        .values()
        .filter(|m| enabled(m))
        .map(|m| m.width as u64 * m.height as u64 * 256)
        .sum();
    if count > MAX_PIXELS as u64 {
        return Err(invalid(
            "Simulated maps exceed the 524288-pixel world budget",
        ));
    }
    let mut work = 0u64;
    let cost = |block: &Block| {
        world.content.models[&block.model]
            .parts
            .iter()
            .map(|p| p.size[0] as u64 * p.size[1] as u64)
            .sum::<u64>()
    };
    for info in &world.content.manifest.maps {
        let Some(source) = world.maps.get(&info.id) else {
            return Err(invalid("Missing pixel map"));
        };
        let map = source.expanded(&world.content);
        for layer in &map.layers {
            for &gid in &layer.data {
                if let Some(block) = tile(layer, &map, &world.content, &info.path, gid) {
                    work += cost(block);
                }
            }
            for item in &layer.objects {
                if let Some(block) = object(layer, item, &world.content) {
                    work += cost(block);
                }
            }
        }
    }
    if work > (MAX_PIXELS * 4) as u64 {
        return Err(invalid(
            "Material templates exceed the rasterization budget",
        ));
    }
    Ok(())
}
pub fn build(world: &World) -> io::Result<BTreeMap<String, Grid>> {
    validate_world(world)?;
    world
        .content
        .manifest
        .maps
        .iter()
        .filter(|m| enabled(&world.maps[&m.id]))
        .map(|m| {
            Ok((
                m.id.clone(),
                Grid::from_map(&world.maps[&m.id], &world.content, &m.path)?,
            ))
        })
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub index: u32,
    pub runs: Vec<[u32; 2]>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub map: String,
    pub tick: u64,
    pub chunks: Vec<Patch>,
}

#[derive(Clone, Debug)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub collision: Map,
    cells: Vec<u32>,
    blocked: Vec<bool>,
    moved: Vec<u64>,
    active: Vec<bool>,
    dirty: Vec<bool>,
    modified: Vec<bool>,
    pub tick: u64,
}
impl Grid {
    pub fn from_map(map: &Map, content: &Content, path: &str) -> io::Result<Self> {
        if !(2..=256).contains(&map.width) || !(4..=128).contains(&map.height) {
            return Err(invalid("Invalid pixel map dimensions"));
        }
        let size = map.width as usize * map.height as usize * 256;
        if size > MAX_PIXELS {
            return Err(invalid("Simulated map exceeds pixel budget"));
        }
        let (width, height) = (map.width * 16, map.height * 16);
        let chunks = (width.div_ceil(CHUNK) * height.div_ceil(CHUNK)) as usize;
        let mut grid = Self {
            width,
            height,
            collision: map.clone(),
            cells: vec![0; size],
            blocked: vec![false; size],
            moved: vec![0; size],
            active: vec![false; chunks],
            dirty: vec![true; chunks],
            modified: vec![false; chunks],
            tick: 0,
        };
        let sources: BTreeMap<_, _> = content
            .pixels
            .iter()
            .map(|(k, v)| (k.as_str(), v.colors()))
            .collect();
        let expanded = map.expanded(content);
        let mut layers: Vec<_> = expanded.layers.iter().enumerate().collect();
        layers.sort_by(|(i, a), (j, b)| a.z(*i).total_cmp(&b.z(*j)));
        for (_, layer) in layers {
            for (i, &gid) in layer.data.iter().enumerate() {
                if let Some(block) = tile(layer, &expanded, content, path, gid) {
                    grid.stamp(
                        block,
                        content,
                        &sources,
                        [
                            (i as u32 % (width / 16) * 16) as i32,
                            (i as u32 / (width / 16) * 16) as i32,
                        ],
                        gid,
                        layer.visible,
                    );
                }
            }
            for o in &layer.objects {
                if let Some(block) = object(layer, o, content) {
                    grid.stamp(
                        block,
                        content,
                        &sources,
                        [o.x.round() as i32, o.y.round() as i32],
                        0,
                        layer.visible && o.visible,
                    );
                }
            }
        }
        grid.rebuild_obstacles(map, content, path);
        for (cell, blocked) in grid.cells.iter_mut().zip(&grid.blocked) {
            if *blocked {
                *cell = 0;
            }
        }
        grid.modified.fill(false);
        Ok(grid)
    }
    fn stamp(
        &mut self,
        block: &Block,
        content: &Content,
        sources: &BTreeMap<&str, Vec<u32>>,
        anchor: [i32; 2],
        gid: u32,
        visible: bool,
    ) {
        let model = block.model("default", content);
        for part in &model.parts {
            let source = &content.pixels[&part.texture];
            let colors = &sources[part.texture.as_str()];
            for y in 0..part.size[1] {
                for x in 0..part.size[0] {
                    let sx = if part.flip_x { part.size[0] - 1 - x } else { x } * source.width
                        / part.size[0];
                    let sy = y * source.height / part.size[1];
                    let rgba = colors[(sy * source.width + sx) as usize];
                    if rgba & 255 < 128 {
                        continue;
                    }
                    let mut px = part.offset[0] + x as i32;
                    let mut py = part.offset[1] + y as i32;
                    if gid & 0x2000_0000 != 0 {
                        (px, py) = (py, px);
                    }
                    if gid & 0x8000_0000 != 0 {
                        px = 15 - px;
                    }
                    if gid & 0x4000_0000 != 0 {
                        py = 15 - py;
                    }
                    self.set(
                        anchor[0] - model.anchor[0] + px,
                        anchor[1] - model.anchor[1] + py,
                        ((rgba >> 8) << 2)
                            | block.material.unwrap() as u32
                            | if visible { 0 } else { HIDDEN },
                    );
                }
            }
        }
    }
    pub fn rebuild_obstacles(&mut self, map: &Map, content: &Content, path: &str) {
        self.collision = map.expanded(content);
        for layer in &mut self.collision.layers {
            let original = layer.clone();
            for gid in &mut layer.data {
                if tile(&original, map, content, path, *gid).is_some() {
                    *gid = 0;
                }
            }
            layer
                .objects
                .retain(|o| object(&original, o, content).is_none());
        }
        self.blocked.fill(false);
        let mut surfaces = self.collision.block_surfaces(content);
        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                surfaces.extend(self.collision.tile_surfaces(content, path, x, y));
            }
        }
        for o in self.collision.objects().filter(|o| o.kind == "solid") {
            surfaces.push((
                [
                    o.x,
                    map.origin_y() - o.y - o.height,
                    o.x + o.width,
                    map.origin_y() - o.y,
                ],
                true,
            ));
        }
        for ([left, bottom, right, top], solid) in surfaces {
            let y1 = (map.origin_y() - top).floor().max(0.0) as u32;
            let y2 = if solid {
                (map.origin_y() - bottom).ceil().max(0.0) as u32
            } else {
                y1 + 1
            };
            for y in y1..y2.min(self.height) {
                for x in
                    left.floor().max(0.0) as u32..(right.ceil().max(0.0) as u32).min(self.width)
                {
                    self.blocked[(y * self.width + x) as usize] = true;
                }
            }
        }
        self.active.fill(true);
    }
    pub fn chunks(&self) -> usize {
        self.dirty.len()
    }
    pub fn chunk_origin(&self, chunk: usize) -> [u32; 2] {
        let cols = self.width.div_ceil(CHUNK);
        [chunk as u32 % cols * CHUNK, chunk as u32 / cols * CHUNK]
    }
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32)
            .then(|| (y as u32 * self.width + x as u32) as usize)
    }
    pub fn get(&self, x: i32, y: i32) -> u32 {
        self.index(x, y).map_or(0, |i| self.cells[i])
    }
    pub fn solid(&self, x: i32, y: i32) -> bool {
        matches!(self.get(x, y) & 3, 1 | 2)
    }
    fn set(&mut self, x: i32, y: i32, value: u32) {
        let Some(i) = self.index(x, y) else {
            return;
        };
        if self.cells[i] == value {
            return;
        }
        self.cells[i] = value;
        let cx = x as u32 / CHUNK;
        let cy = y as u32 / CHUNK;
        let cols = self.width.div_ceil(CHUNK);
        let chunk = (cy * cols + cx) as usize;
        self.dirty[chunk] = true;
        self.modified[chunk] = true;
        for ny in cy.saturating_sub(1)..=(cy + 1).min(self.height.div_ceil(CHUNK) - 1) {
            for nx in cx.saturating_sub(1)..=(cx + 1).min(cols - 1) {
                self.active[(ny * cols + nx) as usize] = true;
            }
        }
    }
    pub fn paint(&mut self, x: u32, y: u32, tool: Tool) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let mut changed = false;
        for dy in -RADIUS..=RADIUS {
            for dx in -RADIUS..=RADIUS {
                if dx * dx + dy * dy > RADIUS * RADIUS {
                    continue;
                }
                let (px, py) = (x as i32 + dx, y as i32 + dy);
                let Some(i) = self.index(px, py) else {
                    continue;
                };
                if self.blocked[i] {
                    continue;
                }
                let shade =
                    ((px.wrapping_mul(73856093) ^ py.wrapping_mul(19349663)) as u32 % 3) * 0x060503;
                let value = match tool {
                    Tool::Dig => 0,
                    Tool::Sand if self.cells[i] == 0 => ((0xc4a46c + shade) << 2) | 2,
                    Tool::Water if self.cells[i] == 0 => ((0x477c91 + shade) << 2) | 3,
                    _ => continue,
                };
                changed |= self.cells[i] != value;
                self.set(px, py, value);
            }
        }
        changed
    }
    fn move_to(&mut self, x: i32, y: i32, nx: i32, ny: i32) -> bool {
        let Some(to) = self.index(nx, ny) else {
            return false;
        };
        let from = self.index(x, y).unwrap();
        let a = self.cells[from];
        let b = self.cells[to];
        if self.blocked[to] || !(b == 0 || (a & 3 == 2 && b & 3 == 3)) {
            return false;
        }
        self.set(x, y, b);
        self.set(nx, ny, a);
        self.moved[from] = self.tick;
        self.moved[to] = self.tick;
        true
    }
    pub fn step(&mut self) {
        self.tick += 1;
        let chunks = self.chunks();
        let active = std::mem::replace(&mut self.active, vec![false; chunks]);
        for chunk in (0..chunks).rev().filter(|i| active[*i]) {
            let [left, top] = self.chunk_origin(chunk);
            for local in (0..CHUNK * CHUNK).rev() {
                let y = top + local / CHUNK;
                let x = left
                    + if self.tick % 2 == 0 {
                        local % CHUNK
                    } else {
                        CHUNK - 1 - local % CHUNK
                    };
                if x >= self.width || y >= self.height {
                    continue;
                }
                let i = (y * self.width + x) as usize;
                let mat = self.cells[i] & 3;
                if mat < 2 || self.moved[i] == self.tick || self.blocked[i] {
                    continue;
                }
                let (x, y) = (x as i32, y as i32);
                let noise = self.tick.wrapping_mul(0x9e3779b97f4a7c15)
                    ^ (x as u64).wrapping_mul(0xbf58476d1ce4e5b9)
                    ^ (y as u64).wrapping_mul(0x94d049bb133111eb);
                let side = if (noise ^ (noise >> 27)).count_ones() % 2 == 0 {
                    1
                } else {
                    -1
                };
                if self.move_to(x, y, x, y + 1)
                    || self.move_to(x, y, x + side, y + 1)
                    || self.move_to(x, y, x - side, y + 1)
                {
                    continue;
                }
                if mat == 3 {
                    for direction in [side, -side] {
                        let mut target = x;
                        for distance in 1..=4 {
                            let nx = x + direction * distance;
                            if self
                                .index(nx, y)
                                .is_none_or(|j| self.blocked[j] || self.cells[j] != 0)
                            {
                                break;
                            }
                            target = nx;
                            if self
                                .index(nx, y + 1)
                                .is_some_and(|j| !self.blocked[j] && self.cells[j] == 0)
                            {
                                break;
                            }
                        }
                        if target != x && self.move_to(x, y, target, y) {
                            break;
                        }
                    }
                }
            }
        }
    }
    fn patch(&self, index: usize) -> Patch {
        let [x, y] = self.chunk_origin(index);
        Patch {
            index: index as u32,
            runs: encode(
                (0..CHUNK * CHUNK)
                    .map(|i| self.get((x + i % CHUNK) as i32, (y + i / CHUNK) as i32)),
            ),
        }
    }
    pub fn snapshot(&self, map: &str) -> Update {
        Update {
            map: map.into(),
            tick: self.tick,
            chunks: self
                .modified
                .iter()
                .enumerate()
                .filter(|(_, v)| **v)
                .map(|(i, _)| self.patch(i))
                .collect(),
        }
    }
    pub fn take_update(&mut self, map: &str) -> Option<Update> {
        let chunks = self
            .dirty
            .iter()
            .enumerate()
            .filter(|(_, v)| **v)
            .map(|(i, _)| self.patch(i))
            .collect::<Vec<_>>();
        self.dirty.fill(false);
        (!chunks.is_empty()).then(|| Update {
            map: map.into(),
            tick: self.tick,
            chunks,
        })
    }
    pub fn validate_update(&self, update: &Update) -> io::Result<()> {
        let mut indices = BTreeSet::new();
        if update.tick < self.tick || update.chunks.len() > self.chunks() {
            return Err(invalid("Out-of-order or oversized pixel update"));
        }
        for patch in &update.chunks {
            if patch.index as usize >= self.chunks()
                || !indices.insert(patch.index)
                || !valid_runs(&patch.runs, (CHUNK * CHUNK) as usize)
                || patch
                    .runs
                    .iter()
                    .any(|r| r[1] >= HIDDEN * 2 || (r[1] != 0 && r[1] & 3 == 0))
            {
                return Err(invalid("Invalid pixel chunk"));
            }
            let [x, y] = self.chunk_origin(patch.index as usize);
            for (i, value) in patch
                .runs
                .iter()
                .flat_map(|r| std::iter::repeat_n(r[1], r[0] as usize))
                .enumerate()
            {
                if self
                    .index((x + i as u32 % CHUNK) as i32, (y + i as u32 / CHUNK) as i32)
                    .is_none()
                    && value != 0
                {
                    return Err(invalid("Pixel update extends outside map"));
                }
            }
        }
        Ok(())
    }
    pub fn apply(&mut self, update: &Update) -> io::Result<()> {
        self.validate_update(update)?;
        for patch in &update.chunks {
            let [x, y] = self.chunk_origin(patch.index as usize);
            for (i, value) in patch
                .runs
                .iter()
                .flat_map(|r| std::iter::repeat_n(r[1], r[0] as usize))
                .enumerate()
            {
                if let Some(j) =
                    self.index((x + i as u32 % CHUNK) as i32, (y + i as u32 / CHUNK) as i32)
                {
                    self.cells[j] = value;
                }
            }
            self.dirty[patch.index as usize] = true;
        }
        self.tick = update.tick;
        Ok(())
    }
    pub fn dirty(&self, chunk: usize) -> bool {
        self.dirty[chunk]
    }
    pub fn clear_dirty(&mut self) {
        self.dirty.fill(false);
    }
    pub fn rgba(&self, chunk: usize) -> Vec<u8> {
        let [x, y] = self.chunk_origin(chunk);
        (0..CHUNK * CHUNK)
            .flat_map(|i| {
                let cell = self.get((x + i % CHUNK) as i32, (y + i / CHUNK) as i32);
                if cell == 0 || cell & HIDDEN != 0 {
                    [0; 4]
                } else {
                    let c = (cell >> 2).to_be_bytes();
                    [c[1], c[2], c[3], if cell & 3 == 3 { 210 } else { 255 }]
                }
            })
            .collect()
    }
    pub fn surfaces(&self, min: [f32; 2], max: [f32; 2]) -> Vec<[f32; 4]> {
        let mut result = Vec::new();
        let origin = self.collision.origin_y();
        for y in ((origin - max[1]).floor() as i32).max(0)
            ..=((origin - min[1]).ceil() as i32).min(self.height as i32 - 1)
        {
            let mut x = (min[0].floor() as i32).max(0);
            let end = (max[0].ceil() as i32).min(self.width as i32 - 1);
            while x <= end {
                if !self.solid(x, y) {
                    x += 1;
                    continue;
                }
                let start = x;
                while x <= end && self.solid(x, y) {
                    x += 1;
                }
                result.push([
                    start as f32,
                    origin - y as f32 - 1.0,
                    x as f32,
                    origin - y as f32,
                ]);
            }
        }
        result
    }
}
