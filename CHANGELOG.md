# Changelog

Each release entry supplies the corresponding GitHub Release Notes. Entries
prepared before tagging are also bundled in every platform archive. When an
entry is absent, the release workflow generates it from Conventional Commits
and commits it after all platform archives have been uploaded and verified.

## [Unreleased]

## [v0.7.2] - 2026-10-10

### Pixel scale settings

- Choose 2x, 3x or 4x in Settings. Larger pixels bring the world closer; the
  default remains 2x. Apply changes immediately and remember them after restart.
- Keep terrain and characters crisp with integer scaling in windowed and
  fullscreen modes. Update camera bounds, background coverage and editor pointer
  mapping when the render canvas changes.
- Preserve language, saved Clubs and other preferences when saving the scale.
  Include English and Simplified Chinese labels. Protocol 3 and content pack
  1.1.0 remain compatible with v0.7.1 servers.

## [v0.7.1] - 2026-10-10

This is the first published 0.7 release. The v0.7.0 tag failed the optimized test
build; this release fixes the test-only fishing constant and includes the full
art and content-pack update below.

### Autumn lakeside art

- Render the world at 720 × 405 with crisp 2x pixels in the fixed window. Redraw
  slimmer 20 × 32 characters, resize nameplates and fishing rigs, and keep
  the map collision grid and gameplay coordinates intact.
- Reduce menu, HUD, satchel and fishing panel sizes; keep text readable in both
  languages. Refresh every README banner, screenshot and native gameplay GIF.
- Add an original illustrated mountain-and-lake panorama, cedar buildings,
  conifers, golden birches, mossy paths and reflective blue water. Retain the
  editable 16px tile grid, stable tile IDs and walking routes. Place the
  lighthouse at the new lake horizon.
- Restyle menus, Clubs, settings, the workshop and fishing panels with dark
  timber/forest surfaces, warm ivory text and brass accents. Keep both languages.
- Update the official art pack to 1.1.0. Preserve frozen legacy map fixtures and
  the separately illustrated panorama when regenerating editable assets.

### Content packs and map authoring

- Load official and community packs through one versioned manifest. Split the
  harbor atlas into terrain, water, buildings and objects, with 375 stable tile
  IDs, ten whole-object patterns and a normal background image layer.
- Preserve resource identity across Tiled GID changes and atlas repacking using
  a saved stable-ID palette. Import old harbor maps without rewriting originals;
  explicit editor saves keep backups and detect external-file conflicts.
- Load variable map lists/sizes and map-defined spawns, portals, tackle shops,
  fishing areas, physical tiles, one-way platforms and solid rectangles.
- Browse all maps/layers in the workshop, paint connected terrain and place whole
  object stamps. Include a complete Tiled edge terrain and bilingual authoring docs.
- Verify installed pack definitions and image hashes before entering a room;
  synchronize map IDs in movement and reject unknown destinations. This is
  **protocol 3 / world format 2**: upgrade client and server together. Older
  releases use protocol 2 and cannot join protocol 3 rooms.
- Package all manifest resources for native/server archives and container builds.

## [v0.6.0] - 2026-10-10

### Clubs and Rooms

- Save multiple server subscriptions as Clubs, with optional personal aliases.
  Add, edit or remove a Club without changing the actual server; browse all its
  rooms in a grouped, collapsible and scrollable online lobby.
- Refresh Clubs independently every eight seconds while the lobby is open. Room
  rows show occupancy, configured capacity and a measured WebSocket round trip.
  Rooms on one Club share its latency; older servers show unknown measurements.
  Probes never join a room or occupy a player slot.
- Keep each Club's password isolated in memory, migrate the previous saved server,
  and discard pending results after an address changes or a Club is removed.
  Automatic refreshes preserve the current scroll position.
- Share complete server-and-room invitations. Pasting one previews the destination;
  bare room codes still use the selected Club. Passwords are never included.

### Settings

- Keep an icon-only pixel gear in the window's top-right corner, including when
  the game canvas has centered borders. Preserve a minimum 48-point tap target,
  support touch activation and reserve space in adjacent controls.
- Include modular English and Simplified Chinese Club catalogs, retaining English
  fallback for missing translations.

### Official Town and Networking

- Unify the official Cloudflare service with the Rust server as **Yapshire Town
  (Yapshire 小镇)**, with one persistent town and one bounded temporary room.
- Connect at `wss://yap-server.mmstudio.games` or
  `wss://yap.meaninglessmeaning.studio`. Both domains reach the same server;
  previous official addresses remain compatible and saved defaults migrate.
- Coalesce queued movement into each player's latest position so brief transport
  stalls do not fill the reliable chat and membership queue with obsolete frames.
- Wait for the Rust container's initial build before running gateway checks in CI.

### Compatibility and Verification

- Existing v0.5.2 clients can continue using the official service. Older self-hosted
  servers remain usable with the new client, with unknown ping/capacity where
  unsupported. The shared map protocol remains version 2.
- Verified native Club CRUD, password isolation, persistence, automatic player
  counts, scrolling, both languages and joining the selected server. Settings
  also pass a phone-sized desktop window and touch-event check; this release does
  not include Android or iOS builds.
- Verified both public domains with real lobby probes, two-client map transfer,
  chat, movement and disconnect, plus the Rust client's live room lifecycle.

## [v0.5.2] - 2026-10-10

### Self-hosted Towns

- Download standalone `yapshire-server` programs for Windows, Linux, macOS Intel
  and Apple Silicon, with editable maps and configuration included. No GPU or
  game window is required.
- Host with the public AMD64/ARM64 Docker image
  `ghcr.io/hsiangnianian/yapshire-server:v0.5.2` or the included Compose file.
- Connect through a saved custom-server address and optional password. Server,
  client and editor share the same Tiled `.tmj` maps. Joining verifies and loads
  the server's maps; leaving restores local maps without overwriting saved files.
- Configure a persistent town, room/player limits, optional player-created rooms
  and password protection. Custom layouts use the bundled tileset and fixed map
  sizes; collision, interaction points and NPC positions remain fixed.

### Cross-platform Fix

- Fix Windows tileset fingerprints differing from Linux/macOS because Git
  converted embedded metadata to CRLF. Normalize metadata line endings before
  hashing and validation, and keep release resource files identical across OSes.
  Existing Windows map folders remain usable; changed metadata or textures are
  still rejected. The Linux/macOS protocol-2 fingerprint is preserved.
- Add a shared world fingerprint fixture to every platform's CI and cover CRLF
  files in the standalone server integration test. Preserve ARM64 build caches
  separately from single-platform container checks.

Windows v0.5.0/v0.5.1 clients must update for cross-platform games. v0.5.1 is now
marked as a pre-release because of that issue. Use v0.5.2 or later clients for
self-hosted towns; compatibility with the existing public Worker is retained.

See the [self-hosting guide](https://github.com/HsiangNianian/Yapshire/blob/v0.5.2/docs/SELF_HOSTING.md)
or [中文开服指南](https://github.com/HsiangNianian/Yapshire/blob/v0.5.2/docs/SELF_HOSTING.zh-CN.md).

## [v0.5.1] - 2026-10-10

### Self-hosted Towns

- Download standalone `yapshire-server` programs for Windows, Linux, macOS Intel
  and Apple Silicon. Run without a game window or GPU; editable maps and a
  ready-to-use configuration are included.
- Host with the public AMD64/ARM64 Docker image
  `ghcr.io/hsiangnianian/yapshire-server:v0.5.1` or the included Compose file.
- Connect through the game's saved custom-server address and optional password.
  Server, client and editor share one Tiled `.tmj` format. Joining downloads and
  verifies the server's map; leaving restores the player's local map.
- Configure a persistent town, room and player limits, optional player-created
  rooms and password protection. Custom layouts use the bundled tileset and fixed
  map sizes; collision, interaction points and NPC positions remain fixed.

### Release Fix

- Verify each published Docker architecture using its platform manifest digest.
  This avoids a Docker image-store conflict when checking AMD64 and ARM64 in the
  same CI job. This release completes the game/server archive publication that
  was blocked by that verification error in v0.5.0.

Use v0.5.0 or later clients for dedicated servers and LAN map synchronization.
See the [self-hosting guide](https://github.com/HsiangNianian/Yapshire/blob/v0.5.1/docs/SELF_HOSTING.md)
or [中文开服指南](https://github.com/HsiangNianian/Yapshire/blob/v0.5.1/docs/SELF_HOSTING.zh-CN.md).

## [v0.5.0] - 2026-10-10

### New Features

- Run an independent `yapshire-server` on Windows, Linux, macOS Intel or Apple
  Silicon, without a game window or GPU. Release downloads include editable maps
  and configuration. `--init` creates a town without overwriting existing files;
  `--check` validates it before opening a port.
- Host with Docker or Compose using the AMD64/ARM64 image
  `ghcr.io/hsiangnianian/yapshire-server:v0.5.0`. The image runs as a non-root user,
  supports read-only map mounts and includes a health check.
- Use the same saved server address for creating and joining rooms, with a
  button to restore the official service and an optional masked server password.
  Passwords remain in memory and are excluded from settings and invite codes.
- Share maps automatically from LAN hosts and dedicated servers. The client,
  editor and server use one Tiled `.tmj` format and validation module. Clients
  verify the server's map revision before joining, retain their local maps,
  and restore them on leaving or disconnecting.
- Keep a configured town available even when empty, with optional player-created
  rooms, room/player limits, password protection, origin checks, bounded message
  sizes and queues, rate limits, and graceful shutdown.

### Compatibility and Verification

- Dedicated servers and LAN map synchronization require v0.5.0 or later clients
  (protocol 2). The updated client remains compatible with the existing public
  Cloudflare Worker, using its original map.
- Custom layouts use the bundled harbor palette. Map sizes, collision, interaction
  points and NPC positions remain fixed; custom textures are not transferred.
  Reload map files by restarting the server and reconnecting players.
- Verified edited-map transfer, two-client chat and movement, passwords, room
  isolation and capacity, malformed/flooding peers, CLI setup, and clean shutdown.
  Native English and Chinese two-window checks also cover map restoration and
  address persistence. CI checks all four native platforms and both Docker
  architectures before publishing the release.

See the [self-hosting guide](https://github.com/HsiangNianian/Yapshire/blob/v0.5.0/docs/SELF_HOSTING.md)
or [中文开服指南](https://github.com/HsiangNianian/Yapshire/blob/v0.5.0/docs/SELF_HOSTING.zh-CN.md).

## [v0.4.0] - 2026-10-10

### New Features

- Open the in-game map editor from the main menu to customize the town, coast
  and tackle-shop interior across five layers. Paint, erase, fill and pick tiles;
  undo or redo strokes, flip tiles, toggle layers, zoom and pan.
- Use original pixel icons with shortcut badges and hover descriptions for map
  tools, layer visibility and settings. Grid and landmark guides help align edits.
- Save maps locally and apply them immediately. Reopen them in Tiled through
  **Map files**, restore bundled layouts as drafts, and choose to save or discard
  unsaved changes before leaving. Saves keep backups and detect external edits.
- Open the pixel gear **Settings** button from menus, gameplay or the editor to
  switch between English and Simplified Chinese immediately. The game remembers
  the selected language after restarting.
- Translate 239 interface messages through six JSON modules per language.
  Missing, blank or invalid translations fall back to English; named placeholders
  preserve player text. See `docs/TRANSLATING.md` to contribute translations.

### Fixes and Polish

- Keep nicknames, chat drafts and unsaved map edits intact when changing language.
  Block gameplay and editor input while settings are open and prevent the closing
  click from affecting controls behind the panel.
- Validate local map data before loading, preserve invalid files for recovery,
  and release old runtime tile entities when applying a new layout.

### Notes

- Map edits stay on the local computer. Collision, interaction points and NPC
  positions remain fixed, and custom maps are not synchronized between players.
- The multiplayer protocol is unchanged from v0.3.0; existing v0.3.0 Workers
  remain compatible. This release does not require a server redeployment.
- Native macOS checks cover editing, pixel controls, language switching and
  preference recovery after a restart. Windows and macOS builds remain unsigned;
  macOS builds are not notarized.

## [v0.3.0] - 2026-10-09

### New Features

- Visit Tide & Tackle, buy a bamboo rod, hook and bait with a starting wallet of
  100 coins, and sell your catches at Mara's counter.
- Fish at the seaside pier: time the bite, hold Space to reel, and release it to
  manage line tension. Catch sardines, mackerel, sea bass and golden bream.
- Open an illustrated satchel with pixel-art equipment, bait, coins and fish;
  watch the casting line, float, splashes and swimming fish during the minigame.
- Explore a tiled coast and shop interior with animated water. The shipped
  16 × 16 maps can be edited in Tiled without rebuilding the game.
- Save wallet, tackle, bait and catches locally per nickname. LAN and online
  friends can see each other's shop location and fishing pose.

### Fixes and Polish

- Cast into open water beyond the pier and stop at its edge; keep the float
  visible while activity panels are open.
- Align pixel panel borders, improve shop door and counter proportions, place
  the shopkeeper behind the counter, and clear overlapping signs.
- Validate map data and local saves, preserve invalid saves, and provide an
  emergency worm when a player with tackle cannot afford more bait.
- Bundle the map and fishing artwork on every platform and use this changelog
  entry for the release notes and packaged documentation.

### Notes

- Everyone should use v0.3.0 for the updated maps and activities. Self-hosted
  Workers need the matching server update; the built-in public server supports it.
- Progress stays on the local computer; items and money are not shared between
  players. Edited maps retain fixed collision and interaction positions and are
  not synchronized between clients.
- Windows and macOS builds are unsigned; macOS builds are not notarized.

## [v0.2.0] - 2026-10-09
### New Features
- [`c1f9e13`](https://github.com/HsiangNianian/Yapshire/commit/c1f9e1388f9604d80cc15808974bb38295464847) - rename game to Yapshire and add bilingual gameplay previews *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*

### Documentation Changes
- [`036b49d`](https://github.com/HsiangNianian/Yapshire/commit/036b49d72835c30973e022ee70d40dcf20501309) - update CHANGELOG.md for v0.1.0 [skip ci] *(commit by [@github-actions[bot]](https://github.com/apps/github-actions))*

## [v0.1.0] - 2026-10-08
### New Features
- [`3390f15`](https://github.com/HsiangNianian/Yapshire/commit/3390f15c60056c5aa51bc0b7d2f50761586d5573) - add Yapshire multiplayer game and cross-platform releases *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*

### Bug Fixes
- [`6c60c3d`](https://github.com/HsiangNianian/Yapshire/commit/6c60c3d1cc7e01937dc6b77921708a65d81b7ba4) - discover local rooms when broadcast routing is unavailable *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*
- [`9327b22`](https://github.com/HsiangNianian/Yapshire/commit/9327b22fe628fbd230848f2e6a45eccf39d5c7d6) - update Worker tooling to patched dependencies *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*
- [`d8e317b`](https://github.com/HsiangNianian/Yapshire/commit/d8e317b7c53c758d58420b5543cbd86572aa46a1) - keep display modes to a fixed window or fullscreen *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*

### Documentation Changes
- [`4efb55f`](https://github.com/HsiangNianian/Yapshire/commit/4efb55f097a1a937cdfa02af952cbacc82b4bb26) - redesign README around game previews and release downloads *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*

[v0.1.0]: https://github.com/HsiangNianian/Yapshire/compare/a21eb24767b62f3b8c9b82bf199d41d1cde36044...v0.1.0

[v0.2.0]: https://github.com/HsiangNianian/Yapshire/compare/v0.1.0...v0.2.0

[v0.3.0]: https://github.com/HsiangNianian/Yapshire/compare/v0.2.0...v0.3.0

[v0.4.0]: https://github.com/HsiangNianian/Yapshire/compare/v0.3.0...v0.4.0
