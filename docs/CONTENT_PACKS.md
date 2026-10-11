# Content packs v1

[简体中文](CONTENT_PACKS.zh-CN.md)

**Available in v0.7.1.** This format uses network protocol **3**, world
format **2**, and pack format **1**. Run matching game/server builds. Earlier releases
use protocol 2 and cannot join protocol 3 rooms.

A pack owns a finite collection of maps and their assets. The official game loads
`assets/packs/yapshire/pack.json` through the same loader as a community pack.
There is one active pack per world; dependency resolution, script mods, automatic
asset downloads, infinite worlds and a building/mining system are outside v1.

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
objects with **OBJECT STAMP / P**. A stamp click places its top-left cell; select
the target tile layer first. Point/rectangle objects and background layers are
preserved and previewed; author those in Tiled. Gold guides show actual map
objects. Save/reload uses the same files as Tiled and the server.

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
  terrain/ground.tsj + ground.png
  terrain/water.tsj  + water.png
  buildings/tackle.tsj + tackle.png
  objects/harbor.tsj + harbor.png
  backgrounds/street.png
  maps/town.tmj
  maps/tackle-shop.tmj
```

See the [working manifest](../assets/packs/yapshire/pack.json). Its fields are:

| Field | Meaning |
| --- | --- |
| `format`, `id`, `version`, `tile_size` | `1`, pack name, content version, `16` |
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

## Coordinates, rendering and collision

Tiled coordinates start at the top left: x right, y down. Actor positions are
feet positions. World conversion is `world_x = tiled_x`,
`world_y = map_height * 16 - 64 - tiled_y`; on the default 17-row map, Tiled
`y=208` is the original walking surface. Map sizes and layer counts can vary.

| Definition | Behavior |
| --- | --- |
| Tile property `collision` | String: `none` (default), `solid`, or `platform` |
| Tile-layer property `collision` | Boolean: opt the entire layer into physical collision |
| Object type `solid` | Axis-aligned rectangular solid collider, independent of artwork |
| Layer property `z` | Optional numeric depth; default `-30 + layer_index * 6` |
| Layer `visible` / `opacity` | Visual state; hiding artwork does not remove its collision |

Players collide with solid tile cells and rectangles. Platforms only catch
players descending through the top. Jumping works from elevated surfaces;
falling below the map returns to a named spawn. The camera uses map dimensions.
A foreground object uses a layer depth above actors (`z=5`); background furniture
uses a lower depth. Keep collision separate when the art's footprint differs
from its physical shape. No slopes, rotated collision or destructible terrain yet.

Image layers reference registered background PNGs and use pixel offsets. The
original street illustration is now an ordinary pack image layer. The existing
outdoor sky/cloud ambience remains shared game presentation; an authored image
layer can cover it. Layer groups, infinite chunks, compressed arrays, tile
objects with `gid`, Tiled templates, polygons and parallax layers are rejected.

## Interactions

Place these objects in an `objectgroup`. Use Tiled's **Type** field (`class` from
older exports is also accepted). Rotation is zero; each object has a unique
positive numeric object ID. Every map needs at least one named spawn.

| Type | Shape | Properties / action |
| --- | --- | --- |
| `spawn` | Point | Unique `name`; the entry and portals refer to this name |
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
colliders are separate `solid` rectangles. No custom shop economy or fish tables
are implied by these reusable triggers; v1 invokes the existing game behaviors.

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
materials do not blend automatically; mixed-material/corner terrain is outside v1.

## Sharing, migration and limits

The server loads/validates its pack before opening a port. It sends definitions,
map layouts and a world revision. The client compares pack ID, version, JSON
content and SHA-256 image fingerprints with its installed pack **before** sending
`world_ready`. A mismatch names a missing/different resource. PNGs and scripts
are never fetched from the server. Map layouts are held only in memory while
joined; leaving restores local maps. Movement includes the stable map ID, so
players in different locations are hidden from each other. The server rejects
unknown map IDs and bounds positions to that map.

Old `town.tmj` / `tackle-shop.tmj` with `harbor.tsj` still import from legacy editor
or server folders. The exact original harbor assets are required if supplied.
The built-in migration table resolves all 359 old IDs, adds the original physical
layer and interaction objects, and preserves flips/art layout. Loading never
rewrites those files. An explicit editor save writes the new pack layout and
keeps the previous bytes as `.tmj.bak`; the legacy file remains available.

Limits: 32 maps and 32 tilesets per pack, 8192 tiles total, 32 layers per map,
2–256 columns × 4–128 rows, 256 objects per object layer, 1 MiB per map,
8 MiB per world message, 4 MiB per asset file and 32 MiB of loaded pack assets.
Only installed, validated definitions can be used in a room.

The supported file conventions follow Tiled's
[JSON format](https://doc.mapeditor.org/en/stable/reference/json-map-format/),
[GID rules](https://doc.mapeditor.org/en/stable/reference/global-tile-ids/) and
[terrain rules](https://doc.mapeditor.org/en/stable/manual/terrain/).
