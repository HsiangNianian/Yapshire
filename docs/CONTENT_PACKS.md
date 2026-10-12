# Content packs v2

[简体中文](CONTENT_PACKS.zh-CN.md)

**v0.9.1:** network protocol **5**, world format **3**, pack format **2**.
Use matching game/server builds. Published v0.7.1–v0.7.2 builds use protocol 3 and
pack format 1; v0.8.0 also uses protocol 3 and cannot join v0.9.1 rooms. This loader still reads
format-1 packs and legacy maps, including the 855 construction tile IDs from v0.8.0.

A pack owns a finite collection of maps and their assets. The official game loads
`assets/packs/yapshire/pack.json` through the same loader as a community pack.
There is one active pack per world; dependency resolution, script mods, automatic
asset downloads and infinite worlds are not implemented. Local solo worlds have
separate tile construction; authored multiplayer maps support the pixel materials below.

## Start with an existing pack

1. Copy `assets/packs/yapshire` to `assets/packs/my_pack` next to the executable.
2. In the copy's `pack.json`, set `id` to `my_pack`, choose a version and remove
   `legacy_tiles` (that table is only for old official saves). Existing tile IDs
   may remain `yapshire:*`; give new assets your own namespace.
3. Open `maps/town.tmj` in [Tiled](https://www.mapeditor.org/). Use finite,
   orthogonal, 16×16 maps and uncompressed JSON tile arrays.
4. Validate with `yapshire-server --maps assets/packs/my_pack --check`.
5. Launch with `YAPSHIRE_PACK=my_pack ./yapshire`. In PowerShell:
   `$env:YAPSHIRE_PACK='my_pack'; .\yapshire.exe`.
6. Share that complete directory with friends. Everyone installs/selects the
   same pack; the server can then distribute different **map layouts** at join.

The in-game editor lists the manifest's maps with **NEXT MAP**, paginates layers,
paints across all tilesets, automatically joins terrain tiles and places whole
objects with **OBJECT STAMP / P**. A model stamp places its **anchor** at the
clicked grid intersection. Select an object layer, or a tile layer to create/use
its companion object layer at the same draw order. Select that object layer to erase
or pick whole instances; undo/redo and saving preserve their identity and state.
Legacy prefab stamps still paint cells from their top-left corner.
Spawn/rectangle objects and background layers are preserved; author those in
Tiled. Gold guides show map objects. Save/reload uses the same files as the server.

Official local overrides remain in the normal Yapshire `maps/` data folder;
other packs use `packs/<pack-id>/`. `YAPSHIRE_MAP_DIR` overrides that location.
Saving copies the pack definitions/images if absent and stores layouts under the
manifest paths, for example `maps/town.tmj` **inside** the override folder.
Missing maps are exported from their last saved layouts, so this directory is a
complete pack; unsaved drafts on other maps stay in the editor.
Existing asset files that differ are never overwritten. Close the game before
updating a pack; keep your map drafts and update their companion assets together.

## Files and identity

```
my_pack/
  pack.json
  blocks/oak.json
  models/oak.json
  textures/oak/trunk.png + leaves.png
  terrain/ground.tsj + ground.png
  terrain/water.tsj  + water.png
  buildings/tackle.tsj + tackle.png
  objects/harbor.tsj + harbor.png
  terrain/natural.tsj + natural.png
  buildings/modules.tsj + modules.png
  nature/garden.tsj + garden.png
  objects/street.tsj + street.png
  backgrounds/street.png  # compatibility for older authored maps
  maps/town.tmj
  maps/tackle-shop.tmj
```

See the [working manifest](../assets/packs/yapshire/pack.json). Its fields are:

| Field | Meaning |
| --- | --- |
| `format`, `id`, `version`, `tile_size` | `2`, pack name, content version, `16` |
| `blocks` | Stable IDs mapped to block-definition JSON paths |
| `models` | Stable IDs mapped to model JSON paths |
| `tilesets` | Ordered pack-relative external `.tsj` paths |
| `images` | Pack-relative PNGs allowed in image layers |
| `maps` | Entries with stable `id`, relative `path`, display `title`, `indoors` |
| `entry` | Starting `map` ID and named `spawn` |
| `prefabs` | Named reusable rectangular tile patterns |
| `terrains` | Named arrays of 16 connected edge variants |

Paths resolve inside the pack, including Tiled references such as
`../terrain/ground.tsj`. Absolute paths, backslashes, drive prefixes and symlinks
escaping the pack are rejected. IDs use `namespace:name`, for example
`my_pack:brick/wall`. Namespaces use lowercase ASCII letters, digits and `_`;
the name also allows `-` and `/`. IDs are unique and case sensitive.

Every tile in a `.tsj` declares a string property `id`. Its numeric `id` and the
map's `firstgid` are Tiled addresses, **not resource identities**. Loading/saving
normalizes GID ranges and preserves horizontal, vertical and diagonal flips.
The map's string property `yapshire:palette` stores the stable ID snapshot used
when it was last saved. Repacking an atlas can change local tile indices while
existing cells continue to resolve by that snapshot. Keep this property.
After changing tile indices, load and save affected maps in Yapshire **before**
continuing to edit them in Tiled; the snapshot and Tiled GIDs are refreshed
together. New Tiled maps without a snapshot use their current tileset definitions.
Deleting/renaming a referenced ID fails explicitly; do not silently reuse it.

Atlas tiles are exactly 16×16 with no spacing/margins. Images can have any
multiple-of-16 dimensions up to 4096px and the declared column count must match.
Animated tiles use consecutive frames with equal positive durations, stored as
normal Tiled `animation` records. The official water has 27 four-frame loops.

## Blocks, models and irregular shapes

A block is an identity and rules; a model supplies its visible parts; a map holds
instances. The stock pack now uses these definitions for terrain, water,
buildings, furniture, lamps, fences and plants in both scenes. Trees and buildings
are assembled from structural components: repeating bark, branch forks and
transparent leaf clusters; repeating siding, shingles, roof slopes, windows and
doors. A component is authored as its own PNG, never as an arbitrary grid cut
from a finished tree or house. The same pine model and full-resolution textures
appear at multiple physical distances. Sky, hills and clouds remain backgrounds.
The old street PNG is retained for older layouts, but the stock town no longer
displays it as one baked foreground image. Old object fragments stay only in the
compatibility atlases; they do not create duplicate block/model/PNG sources.

For example, register `my_pack:oak` in both `blocks` and `models`. The block JSON:

```json
{
  "model": "my_pack:oak",
  "colliders": [{"rect": [-3, -24, 6, 24]}],
  "selection": [[-20, -56, 16, 24], [4, -56, 16, 24], [-3, -32, 6, 32]]
}
```

And the model JSON:

```json
{
  "size": [40, 56],
  "anchor": [20, 56],
  "parts": [
    {"texture": "textures/oak/trunk.png", "offset": [17, 24], "size": [6, 32]},
    {"texture": "textures/oak/leaves.png", "offset": [0, 0], "size": [16, 24]},
    {"texture": "textures/oak/leaves.png", "offset": [24, 0], "size": [16, 24], "flip_x": true}
  ]
}
```

Model offsets start at its top left. Colliders, selection rectangles and the
interaction rectangle are relative to the placement anchor; all use pixels,
x right/y down. Parts render in listed order, at nearest-neighbor scale, with PNG
alpha. A branch can overhang several cells, leave transparent gaps and use a
trunk-only collider. It does not need to fill a square. Each part can be reused
by another model, or registered as a smaller block for independent placement.

`selection` is optional: by default picking uses the part rectangles, **not
per-pixel alpha**. Explicit selection rectangles let authors leave gaps between
branches. `colliders` defaults to empty; set `platform: true` for a one-way
surface. Collision is independent of selection and visible pixels.

Optional block fields are `behavior` (`decoration`, `portal`, `shop`, `fishing`,
`toggle`), `interaction: [x,y,w,h]`, default string `properties`, and `states`.
Each state maps to a `model` ID and optional replacement `colliders`. The initial
state is `default`. A `toggle` must define `open`; E switches between them.
The stock barrel demonstrates a synchronized lid, not a storage inventory.
Portal definitions supply `target_map`/`target_spawn`; instance properties may
override these, so moving a door also moves its entrance area.

In Tiled place a **point**, Type `block`, with string property `block` set to
`my_pack:oak` and optional `state` (default: `default`). Its x/y is the model's
anchor. Yapshire renders the full model; Tiled shows the point marker. Definitions
and individual part artwork are edited as JSON/PNG, not inside the map editor.

For terrain, mark a block `tile: true`. It must have a static 16×16 model anchored
at `[0,0]`, decoration behavior and no states. Each `.tsj` cell's string `block`
property points at that definition. From a source checkout, after editing PNGs
or adding registered tile blocks, run:

```sh
uv run --with Pillow tools/compile_pack.py assets/packs/my_pack
cargo run -p yapshire-server -- --maps assets/packs/my_pack --check
```

The compiler rebuilds atlases from the source models, preserves existing tile
indices and animations, adds new terrain IDs in `generated/blocks.tsj`, and leaves
maps untouched. Restart to load changed assets. For object models, edit their
JSON/PNGs and restart; their parts are loaded directly. Do not run
`tools/draw_maps.py` on community work: that recreates the official stock layouts.

## Coordinates, rendering and collision

Tiled coordinates start at the top left: x right, y down. Actor positions are
feet positions. World conversion is `world_x = tiled_x`,
`world_y = map_height * 16 - 64 - tiled_y`; on the default 17-row map, Tiled
`y=208` is the original walking surface. Map sizes and layer counts can vary.

| Definition | Behavior |
| --- | --- |
| Tile property `build_layer` | Optional string `wall`, `solid` or `decor`: default construction layer |
| Tile property `label` | Optional English display name; bundled translations use the `tiles` domain |
| Tile property `collision` | String: `none` (default), `solid`, or `platform` |
| Tile property `block` | Uses the definition's partial colliders instead of the legacy collision string |
| Object type `block` | Uses model parts, selected state and anchor-relative colliders |
| Tile-layer property `collision` | Boolean: opt the entire layer into physical collision |
| Object type `solid` | Axis-aligned rectangular solid collider, independent of artwork |
| Layer property `z` | Numeric draw order; default `-30 + layer_index * 6` |
| Layer `visible` / `opacity` | Visual state; hiding artwork does not remove its collision |

Players collide with solid tile cells and rectangles. Platforms only catch
players descending through the top. Jumping works from elevated surfaces;
falling below the map returns to a named spawn. The camera uses map dimensions.
A foreground object uses draw order above actors
(`z=5`); background furniture uses a lower order. Keep collision separate when the art's footprint differs
from its physical shape. Authored colliders do not support slopes or rotation.

Image layers reference registered background PNGs and use pixel offsets. The
existing outdoor sky/cloud ambience remains shared game presentation; an authored image
layer can cover it. Layer groups, infinite chunks, compressed arrays, tile
objects with `gid`, Tiled templates, polygons and parallax layers are rejected.

## Pure 2D pixel rendering

The world uses an orthographic camera. One world unit covers one canvas pixel
at every draw order; `z` only controls overlap. Native-size components share
the terrain and character pixel grid. No perspective projection, horizontal
XZ ground or scenery-distance scaling is applied.

Camera edges align to whole world pixels, including odd canvas dimensions.
The 2x/3x/4x setting changes the canvas resolution; nearest-neighbor display
scaling keeps pixels crisp. Tile flips, animation and transparent model parts
remain supported. Use integer coordinates and native PNG sizes when authoring
new parts. Trees use complete reusable bark, branch and leaf components rather
than slices cut from a finished scene.

## Pixel materials

Blocks may add `"material": "solid"`, `"sand"` or `"water"`. Enable them on a
tile or object layer with this Tiled property:

```json
{"name": "simulation", "type": "bool", "value": true}
```

Both are required: without the layer opt-in, the block keeps its normal artwork
and authored colliders. Material blocks use `decoration` behavior and cannot have
states. The stock Terrain and Water layers opt in. The editor also offers
`yapshire:materials/sand` and `yapshire:materials/water` stamps; placing a material
model enables simulation on its object layer in the same undoable edit.

At room creation, model parts supply their PNG colors and alpha masks at the
authored sizes, offsets, anchors and flips. Alpha below 128 is empty; the remaining
pixels become material cells. Use native sizes for a one-to-one texture mapping.
Higher layers overwrite occupied pixels from lower layers. All material layers
merge into one physical plane rendered at `z=-1`; hidden material retains its
physics. Simulation layers require full opacity. Keep scenery and foreground
decoration on ordinary layers. Material colliders come from their solid pixels;
other blocks, doors, shops, platforms and their existing interactions stay intact.

Press **V** in a room to open the tools, **1/2/3** to select dig/sand/water, and
hold the left mouse button to use a 5-pixel-radius brush within 48 pixels of the
character. Typing, menus and fishing panels block the brush. Sand falls and piles
up; water flows into empty space and sand sinks through it. Solid and sand cells
block players. Digging removes cells, so collision changes with the hole. Static
colliders remain obstacles and cannot be dug with this tool.

Only the room's Rust server advances materials, at 20 Hz. It sends changed 32x32
chunks using run-length encoding and includes modified chunks in the join
snapshot. LAN and Cloudflare gateways use the same protocol **5**. Templates stay
editable as tilemaps/models; no entity or sprite is allocated per material pixel.

Limits: at most **524288 pixels** across the bounding areas of all simulated maps
per room; the same budget applies to decoded material textures, and template
rasterization is capped at four times that budget. Room changes stay in memory
until the room is removed or the server restarts. Saving an editor map saves the
starting template, not live particles. Combustion, rigid-body collapse, swimming
and chemical reactions are not implemented. Player movement still uses the
existing client-reported positions; material authority is not movement anti-cheat.

## Interactions

Place these objects in an `objectgroup`. Use Tiled's **Type** field (`class` from
older exports is also accepted). Rotation is zero; each object has a unique
positive numeric object ID. Every map needs at least one named spawn.

| Type | Shape | Properties / action |
| --- | --- | --- |
| `spawn` | Point | Unique `name`; the entry and portals refer to this name |
| `block` | Point | String `block` ID, optional `state`; appearance and behavior from its definition |
| `portal` | Rectangle | Strings `target_map`, `target_spawn`; press E inside to travel |
| `shop` | Rectangle | Press E for the existing tackle shop; the clerk follows the area |
| `fishing` | Rectangle | Press E to start the existing fishing game, casting to the right |
| `solid` | Rectangle | Physical collision; no artwork implied |
| `prefab` | Point | String `prefab` selects a manifest pattern |

A prefab contains `width`/`height` in cells, pixel `anchor: [x,y]`, and a row-major
`tiles` array of stable IDs (empty strings are transparent). Its point position
minus its anchor must align to the grid and the whole footprint must fit the map.
These point objects render as complete patterns in Yapshire; Tiled displays their
markers. Use tile stamps instead if a Tiled art preview is essential. Object
colliders are separate `solid` rectangles. Prefer `block` for new objects so that
their artwork and rules travel together. Custom shop economies and fish tables
are not implemented; these triggers invoke the existing game behaviors.

To add a connected location, copy a `.tmj`, add it to `pack.json.maps` under a new
ID, place an `arrival` spawn, and set a portal's `target_map`/`target_spawn` to it.
Add a return portal the same way. Moving a shop/fishing region or changing its
artwork needs no Rust changes. Add/register any new art before validation.

## Connected terrain

The official ground tileset contains **Quay**, a complete 16-pattern edge terrain.
Tiled can paint it with its Terrain brush. The in-game brush/fill also recomputes
neighbors when any terrain tile is painted or erased on that layer.

`terrains["my_pack:stone"]` lists stable IDs in mask order `0..15`:
**N=1, E=2, S=4, W=8**, where a bit means a matching terrain neighbor. Each tile
has a string `terrain` property with that same ID. All 16 variants must be in
one tileset; declare matching `wangsets` to use Tiled's Terrain brush. Separate
materials do not blend automatically; mixed-material/corner terrain is not implemented.

## Sharing, migration and limits

The server loads/validates its pack before opening a port. It sends definitions,
map layouts and a world revision. The client compares pack ID, version, JSON
content and SHA-256 image fingerprints with its installed pack **before** sending
`world_ready`. A mismatch names a missing/different resource. PNGs and scripts
are never fetched from the server. Map layouts are held only in memory while
joined; leaving restores local maps. Movement includes the stable map ID, so
players in different locations are hidden from each other. The server rejects
unknown map IDs and bounds positions to that map.

Object interactions refer to map/object IDs. The Rust server checks the registered
behavior and the player's last reported position against the interaction area,
then broadcasts state changes. Late joiners receive the room's current states.
Room states are isolated and held in memory; server restarts reset them. State
changes are not written into local editor files. LAN and Cloudflare gateways use
the same server implementation. Custom executable behaviors still require Rust;
content packs cannot load arbitrary code.

Old `town.tmj` / `tackle-shop.tmj` with `harbor.tsj` still import from legacy editor
or server folders. The exact original harbor assets are required if supplied.
The built-in migration table resolves all 359 old IDs, adds the original physical
layer and interaction objects, and preserves flips/art layout. Loading never
rewrites those files. An explicit editor save writes the new pack layout and
keeps the previous bytes as `.tmj.bak`; the legacy file remains available.

Limits: 32 maps and 32 tilesets per pack, 8192 tiles total, 32 layers per map,
2–256 columns × 4–128 rows, 256 objects per object layer, 1 MiB per map,
8 MiB per world message, 4 MiB per asset file and 32 MiB of loaded pack assets.
There are at most 8192 block/model definitions each, 128 parts per model,
1024px per model dimension, 64 colliders, 128 selection rectangles and 16 states
per block. Definitions and referenced textures participate in content validation.
Only installed, validated definitions can be used in a room.

The supported file conventions follow Tiled's
[JSON format](https://doc.mapeditor.org/en/stable/reference/json-map-format/),
[GID rules](https://doc.mapeditor.org/en/stable/reference/global-tile-ids/) and
[terrain rules](https://doc.mapeditor.org/en/stable/manual/terrain/).
