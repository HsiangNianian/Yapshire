# Explore and build

[简体中文](OPEN_WORLD.zh-CN.md)

Yapshire v0.8.0 includes the first **local, solo** open-world mode.
Launch the game (or run `cargo run --locked` from source), then choose
**[5] Explore & Build** on the main menu.

Create a named world with an optional numeric seed (0–4294967295). The same seed
produces the same hills, soil, sand, trees, shrubs and underground caves. Each world is a finite
2048 × 256 tile area, with 16px tiles. Create multiple independent saves; click a
saved world to resume it. A world name never becomes a file path.

| Control | Action |
| --- | --- |
| A / D, arrows | Walk |
| Shift | Run |
| Space | Jump |
| Left mouse, hold | Remove a cell on the selected layer |
| Right mouse, hold | Place the selected material or catalog tile |
| 1 / 2 / 3, material buttons | Select dirt, stone or sand |
| B / Build | Open the tile catalog (unlimited supply) |
| E / layer buttons | Select wall, solid or decoration layer |
| Middle mouse | Pick a tile from the selected layer |
| H | Return to a safe surface at the spawn column |
| F5 / Save | Save immediately |
| Esc | Close the catalog, or save and return to the world list |
| Save & Exit | Save and return to the world list |

The three material slots collect mined terrain and consume it when placing.
The building catalog supplies every installed pack tile without material costs.
Use its category arrows to choose building parts, trees/plants, street/garden,
shop pieces, harbor furniture, natural terrain, paving or water. Tiles include
repeating walls, roof slopes, windows, door halves, trunks, connected leaves,
shrubs, flowers, fences, benches and lanterns. Water and doors are decorative;
this does not add fluid simulation or new portal interactions.

Wall and decoration layers do not block movement. The solid layer uses each
tile’s shared collision definition, including one-way platforms. Three layers
can occupy one coordinate and are edited independently. New trees and shrubs
are generated from the same individual tiles; removing a leaf/trunk is saved.
Connected foliage updates its edge shape across chunk boundaries.

Edits require line of sight and a reach of six tiles. Collected terrain must
touch existing terrain or a colliding construction tile; catalog tiles may be placed freely within
reach. Colliding tiles cannot overlap the player. The bottom and outer edge are protected. Settings block
world input; interface and scene scales remain independent.

Only visible 32 × 32 chunks are rendered, and chunks outside the camera unload.
Terrain generation is deterministic. Save format 2 stores the seed, ground
generator version, vegetation version, terrain edits, per-layer stable tile IDs,
inventory and position. Version 1 saves still load with their original terrain
and no newly generated vegetation. Their first catalog edit upgrades them to
format 2. Vegetation version 2 uses the detailed, versioned tree/shrub recipes;
vegetation version 1 retains its previous cell layout, including erased cells.
Missing recipe footprints or tile IDs are reported before opening; they are never silently
substituted. Keep a backup when upgrading: older clients cannot read format 2.
Worlds autosave every five seconds and save before leaving or closing the window.
A failed save keeps the world open and displays an error so you can retry.
Corrupt or newer-format files are not overwritten, and external file changes
are detected before saving. Use only one game instance per save; this is not a
shared-file locking or multiplayer storage system.

Worlds live in `worlds/` under the normal Yapshire application data directory:

- macOS: `~/Library/Application Support/Yapshire/worlds`
- Windows: `%APPDATA%/Yapshire/worlds`
- Linux: `$XDG_DATA_HOME/yapshire/worlds`, or `~/.local/share/yapshire/worlds`

`YAPSHIRE_WORLD_DIR` overrides the directory for development/tests. Back up the
JSON files with the game closed. Editor packs and fishing profiles are separate.

## Current boundary and next stage

v0.9.0 Club/LAN rooms use authored maps, protocol **5** and pack **2.0.0**.
They do not host generated solo worlds or synchronize their tile construction.
The 855 construction tile IDs and older local saves remain supported. Authored
rooms now simulate pixel sand, water and excavation; generated solo worlds keep
their saved tile terrain. Upgrade multiplayer clients and servers together.
Crafting, enemies, survival, fluids in solo worlds, infinite terrain and
community-pack terrain generation are not implemented.

The next multiplayer stage should keep **Club → Room → world**: a Club lists
servers' rooms, each room owns a persistent world, and surface/caves are regions
within that world. That requires independent per-room state, stable player
identity, server validation of edits/materials, revisioned chunk snapshots/deltas
and durable saves before exposing shared building. The current server instead
shares one immutable pack world across rooms and drops temporary empty rooms.

## Implementation and native acceptance

`yapshire-shared::construction` derives the build catalog from the content pack.
`yapshire-shared::terrain` owns deterministic generation and edit rules without
Bevy/network dependencies. `src/wilds/storage.rs` handles versioned local saves;
`src/wilds.rs` handles movement, picking and visible chunks; `src/wilds_ui.rs`
owns the translated world list, layer controls and catalog. Tile names live in
the separate `tiles.json` translation domain with English fallback. The existing movement collision
step and pixel assets are reused. Generator 1 must remain stable for existing
saves; a terrain algorithm change needs an explicit migration/version.

After `cargo build --locked`, use fresh isolated directories:

```sh
export YAPSHIRE_WORLD_DIR="$PWD/artifacts/world-check/worlds"
export YAPSHIRE_SETTINGS_DIR="$PWD/artifacts/world-check/settings"
export YAPSHIRE_MAP_DIR="$PWD/artifacts/world-check/maps"
export YAPSHIRE_SAVE_DIR="$PWD/artifacts/world-check/fishing"
YAPSHIRE_SMOKE=wilds ./target/debug/yapshire
YAPSHIRE_SMOKE=wilds-reload ./target/debug/yapshire
```

The native check mines/places using physical pointer coordinates, walks/jumps,
visits distant chunks, checks cleanup, changes UI/scene scales, checks modal input
isolation and resumes saved edits/materials. The second process checks restart
persistence. Screenshots are written to `artifacts/wilds-*.png`.

The additional native modes `YAPSHIRE_SMOKE=wilds-tiles` and
`YAPSHIRE_SMOKE=wilds-tiles-reload` use the same isolated directory variables.
They test catalog touches, per-layer pointer placement/removal, constructed
collisions, compact/scaled bilingual UI and disk persistence.
`YAPSHIRE_SMOKE=wilds-tiles-art` captures the village and harbor in the native
renderer for art review; it uses a local test actor and does not open a server.
