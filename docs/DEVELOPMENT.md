# Development guide

A small, native Rust + Bevy multiplayer pixel town. Walk past the coffee shop,
meet your friends, and press **Enter** to talk. Yapshire supports English
and Simplified Chinese; the bundled pixel font includes both, so no system font
installation is required.

## Run

```sh
cargo run --locked
```

The first build downloads and compiles dependencies. Development builds are
optimized for gameplay. Assets load from an `assets/` folder beside the executable,
or fall back to this source checkout during development. To share a native build,
copy its binary and the entire `assets/` directory into the same folder.

### Main menu

- **Create a room**: choose **Local network** or **Online room**. Online hosting
  asks for a room name (up to 24 characters), server address and optional password.
  Hosting and joining share the saved address; the public server is the default.
- **Join LAN**: automatically discovers nearby rooms. Click a room, or enter an
  IP and port yourself. On the same computer, use `127.0.0.1:4761`.
- **Online lobby**: save multiple server subscriptions called **Clubs**. Add a
  server address and optional local alias; its rooms appear below that Club. Edit
  or remove subscriptions, collapse groups, and scroll through the directory.
  Clicking a room uses that Club's address and its own optional password. Full
  invitations also work, while bare room codes use the selected Club.

While the lobby is open, Clubs refresh independently every eight seconds;
**Refresh all** also schedules every Club immediately. Up to four scans run at
once, with the longest-waiting entries first. Leaving the lobby stops new scans.
Each room displays its name, occupancy/capacity and its Club's measured WebSocket
round trip. The round trip excludes the initial connection and map download; all
rooms on the same server share it. Legacy servers show unknown ping/capacity.
Failed scans retain clearly marked stale rooms and disable joining from them.

Club aliases, addresses and stable IDs save atomically in `settings.json`. At
most 32 Clubs can be saved; duplicate normalized addresses are rejected. Migration
keeps the previous custom address alongside the official town; explicitly empty
lists stay empty. Passwords are isolated per Club in memory, never serialized.
Changing credentials/addresses invalidates pending results.

Online rooms display their chosen name; LAN rooms use
the host's nickname. Empty player-created online rooms disappear from the lobby;
a dedicated server's configured town stays listed. **Copy invite** copies a LAN
address or a complete online URL such as `wss://host.example/room/ABCDEFGH`.
Invitations never include passwords, player names or room-creation parameters.

The default deployed server is:

```text
wss://yap-server.mmstudio.games
```

`wss://yap.meaninglessmeaning.studio` reaches the same official town.

### Controls

| Input | Action |
| --- | --- |
| A / D or Left / Right | Walk |
| Shift | Run |
| Space | Jump / hook a bite / hold to reel |
| E | Enter / leave the tackle shop, browse the counter, cast at the pier |
| I | Open / close the illustrated satchel |
| Enter | Open chat / send |
| Escape | Close a panel / cancel a cast, exit the shop, close chat, unfocus a field, or leave the room |
| Tab | Next input field |
| Ctrl+A / Ctrl+V / Ctrl+C | Select, paste, copy in an input |
| 1 / 2 / 3 | Main menu shortcuts |
| 1 / 2 | LAN / server while choosing where to host |
| F11 | Toggle fixed-size window / fullscreen |
| F12 | Save an actual game screenshot under `artifacts/` |

Chat pauses movement while typing. Messages appear above the speaker for eight
seconds and in the room's recent chat log. Long bubbles are shortened; the log
retains the full message. Text input supports Unicode and IME composition.

Windowed mode is fixed at 1440 × 810 logical pixels; resizing and maximizing are
disabled. F11 switches to borderless fullscreen on the current monitor and
restores the fixed size when leaving. The UI and world share a centered 16:9
viewport and integer pixel scale, including on HiDPI and ultrawide displays.

## Fishing and local progress

Version 0.3.0 includes a walkable tackle-shop interior and a coastal pier.
New nicknames start with 100 coins. The counter sells a rod (45), a reusable hook
(15), and five worms (10); both tackle items equip automatically. Each cast spends
one worm, including missed bites, escaped fish and cancelled casts. A bite gives
1.5 seconds to press Space. Hold Space to reel, release to reduce tension, and
bring the catch meter to full. Fish can be sold at the counter. If a player with
tackle runs out of bait, has no fish, and cannot afford a bait pack, the counter
provides one emergency worm.

The satchel shows pixel-art tackle, a worm tin, coins and four fish slots, with
equipped states and stack counts. The shop uses matching illustrated purchase
cards. At the end of the pier, casting animates the rod and line into open water;
the float bobs, a bite splashes, and the fish darts as it pulls against the line.
The small water view shows the fish moving closer as you reel. A landed fish
lifts out of the sea before appearing in the satchel. The chat and controls
panels hide during these activities so the float remains visible.

Purchases, casts, catches and sales save automatically per nickname. Files are
named with the nickname's UTF-8 bytes encoded as hex:

| Platform | Save directory |
| --- | --- |
| Linux | `$XDG_DATA_HOME/yapshire`, or `~/.local/share/yapshire` |
| Windows | `%APPDATA%/Yapshire` |
| macOS | `~/Library/Application Support/Yapshire` |

`YAPSHIRE_SAVE_DIR` overrides the directory for isolated tests. Writes replace
the save through a temporary file. Invalid or unsupported saves disable purchases
and fishing instead of overwriting the file; an on-screen message reports load or
write failures. Keep using the same nickname to resume progress.

Progress is local to each computer. Coins and catches are not traded or synchronized
between players. Movement packets carry the stable map ID and fishing pose;
the server derives indoor/outdoor state from the map definition. Current source
uses protocol 3, so update the client and server together.

## Settings and language

The icon-only pixel gear stays in the top-right corner in the menu, game and map
editor. Its hit area stays at least 48 logical window pixels as the game canvas
shrinks. Adjacent lobby/editor/game controls reserve space for it. Mouse and touch
activation both prevent a held press from clicking through a closing panel.
Choose **English** or **简体中文**; the interface updates immediately, including
existing status messages, without clearing chat fields or map drafts. Closing
settings returns to the same screen. Movement, fishing and editing input pause
while the settings panel is open; the room connection remains active.

Language defaults to English and saves in `settings.json` in the platform's
normal Yapshire data directory. `YAPSHIRE_SETTINGS_DIR` overrides that directory.
Missing Chinese translations fall back to English. See the
[translation guide](TRANSLATING.md) for the modular catalogs, placeholders and
native acceptance checks. Settings and language selection are included in v0.5.0.

## In-game map editor

Yapshire v0.5.0 includes **04 MAP EDITOR** on the main menu. Press **4** or
**F2** when no text field is selected. This is an offline editing screen; return
to the main menu from a room before opening it.

Use **Next map**, select a layer (listed front to back, with page arrows),
and pick a tile from the pack's paginated palette. Each map keeps its own draft
and up to 100 undo steps while switching between them. A drag is one undo step.
The tool strip uses original 16px pixel icons, with B/E/I/F/P shortcut badges and
hover descriptions. The active tool's name stays visible beside the strip;
unavailable undo/redo and zoom controls are dimmed. Eye icons toggle layer visibility.

| Input / control | Action |
| --- | --- |
| Left mouse | Paint, erase, fill or pick with the selected tool |
| Right mouse | Erase from the selected layer |
| B / E / I / F | Brush / eraser / eyedropper / flood fill |
| Alt + left mouse | Pick a tile from the selected layer |
| H / V / Swap button | Horizontal / vertical / diagonal tile flip |
| Ctrl/Cmd+Z | Undo |
| Ctrl/Cmd+Shift+Z or Ctrl/Cmd+Y | Redo |
| Ctrl/Cmd+S / Save | Save and immediately apply the current map |
| - / + | Zoom between 1x, 2x and 3x |
| Arrow keys / middle mouse drag | Pan the canvas |
| Mouse wheel over the canvas | Pan horizontally |
| G / Grid | Toggle the grid |
| Guides | Preview map-defined objects |
| P / Object stamp | Place a complete prefab on the selected tile layer |
| Next map / layer arrows | Browse the manifest and all layers |
| Layer eye icon | Show or hide a layer (saved in the map) |
| Reload | Read the current map from disk; confirm before discarding a draft |
| Original | Restore the bundled layout as an undoable draft; save to keep it |
| Done / Escape / window close | Leave; unsaved drafts offer save, discard or cancel |

Gold guides preview map-defined spawn points, doors, shops and fishing regions.
The manifest lists editable maps and tilesets. Collision comes from tile-layer
properties and solid objects; gameplay regions can be moved in Tiled. The editor
paints tiles, connects terrain, places whole object stamps, and preserves object
and image layers. See [Content packs v1](CONTENT_PACKS.md) / [中文规范](CONTENT_PACKS.zh-CN.md).

Local saves preserve Tiled metadata and flips, use atomic replacement and keep
previous bytes as `.tmj.bak`. External edits must be reloaded before saving.
Loading an old harbor map migrates it in memory; only an explicit save writes the
new layout. Joining never overwrites editor files, and leaving restores local maps.

For native acceptance, use fresh isolated map and settings folders with a debug
build. This keeps the check in English regardless of your saved language:

```sh
cargo build --locked
YAPSHIRE_SETTINGS_DIR="$PWD/artifacts/editor-check/settings" \
YAPSHIRE_MAP_DIR="$PWD/artifacts/editor-check/maps" YAPSHIRE_SMOKE=editor ./target/debug/yapshire
YAPSHIRE_SETTINGS_DIR="$PWD/artifacts/editor-check/settings" \
YAPSHIRE_MAP_DIR="$PWD/artifacts/editor-check/maps" YAPSHIRE_SMOKE=editor-reload ./target/debug/yapshire
```

The opt-in driver exercises real editor systems and UI actions: painting and a
continuous stroke, undo/redo, saving into runtime tilemaps, repeated-save cleanup,
shop editing, unsaved changes, fullscreen pointer mapping, icon hover/click,
layer visibility and startup reload.
Screenshots go to `artifacts/editor-*.png`; input is injected only into this app.

## Tilemaps

The official content pack is `assets/packs/yapshire/pack.json`. Its terrain,
water, buildings, objects and background assets are separate resources with
stable IDs. It uses the same loader as community packs. Open its `maps/*.tmj`
files in Tiled or use the in-game workshop. Read [Content packs v1](CONTENT_PACKS.md)
for the schema, authoring workflow, interactions, collision, migration and limits.
The old `assets/maps/harbor.*` and two maps remain read-only migration fixtures.

Regenerate the official art, pack and both default layouts with:

```sh
uv run --with Pillow tools/draw_maps.py
```

This overwrites shipped layouts; preserve hand-edited copies first. The generator
checks old-to-new resource identities. Shared Rust tests exercise palette remaps,
third-party maps, portals, PNG fingerprints and invalid content. Packaging follows
`pack.json` and includes all referenced maps, tilesets and images on every platform.

## Networking

**LAN:** the game hosts its own Rust WebSocket server on TCP 4761 (configurable in
the host screen). UDP 4762 handles automatic discovery. Permit these ports on the
host's firewall. Discovery is limited to the same broadcast network; on routed
networks, VPNs, or Wi-Fi with client isolation, use the manual address. One LAN
host per computer uses the discovery port; multiple client windows are supported.
The LAN room closes when its host leaves. Its server is the same library used by
the standalone `yapshire-server`, including map transfer and validation.

**Dedicated server:** runs without Bevy or a GUI in `crates/yapshire-server`.
It serves a persistent configured room, optional player-created rooms, password
protection and a validated world snapshot. The client, editor and server share
Tiled map types and wire messages from `crates/yapshire-shared`. Start with the
[self-hosting guide](SELF_HOSTING.md) for downloads, Docker and custom maps.

**Official online:** a Cloudflare Worker and a single container-owning Durable
Object forward connections to the same Rust server used for self-hosting and LAN.
The configured `NIANNIAN` town stays listed; player-created rooms survive their
creator leaving while others remain, and close when empty. Chat history and
positions are not saved between sessions. The old `yapshire-multiplayer` address
is a service-binding gateway to this server, not a separate room backend.

v0.7.2 uses protocol **3** and world format **2**. Clients require a
validated world and matching installed content before acknowledging entry. Update
the game and server together; older releases use incompatible protocol 2. There is no legacy no-world handshake fallback in this build.
Positions are transmitted
at up to 20 Hz only when changed; remote players interpolate between updates.
Connections have timeouts and heartbeats. Each room allows 16 players, names are
limited to 12 characters, and chat to 80 characters. The servers bound message
size and rate, sanitize text, clamp positions, and assign identities themselves.
This is a friendly social toy: movement is client-driven, not competitive anti-cheat.
The official deployment has a bounded room count in `server/club/server.json`;
self-hosted servers can configure up to 40 rooms.

Online connections honor `https_proxy` / `HTTPS_PROXY` and `all_proxy` /
`ALL_PROXY` for unauthenticated HTTP CONNECT proxies, with `no_proxy` / `NO_PROXY`
exceptions. LAN connections always connect directly. TLS certificate validation
remains enabled. Authenticated and SOCKS proxies are not currently supported.
Initial transport failures retry up to three times before showing an error;
room creation requests are never automatically repeated.

Rooms appear in their server's lobby; dedicated servers can protect it with a
shared password. There are no user accounts, moderation tools, matchmaking across unrelated servers, or automatic
reconnection; after a disconnect, return to the menu and join again. Treat LAN
rooms as trusted-network play. Cloudflare account quotas and usage charges apply.

## Cloudflare server

`server/club/` deploys the official **Yapshire Town (Yapshire 小镇)** Rust container.
`server/wrangler.jsonc` preserves the previous address as a thin service-binding
gateway. Both configurations pin the intended Cloudflare account; change the
account, names and custom-domain routes when deploying your own copy.

Run `npm ci` inside `server` to install the project's pinned Wrangler version.
Authenticate with `npx wrangler login` when deploying to your own account.
The project uses the official npm registry. A temporary `sharp` 0.35.5 override
patches Miniflare's pinned image dependency; remove it when Miniflare includes
that fix upstream.

```sh
cd server
npm run dev
```

Docker is required. This starts both Workers and the Rust container locally.
Set the game's server address to `ws://127.0.0.1:8787` to exercise the compatibility
gateway, or use `npm run dev:club` for the direct gateway on port 8788.

```sh
cd server
npm run deploy:club -- --dry-run
npm run deploy:club
npx wrangler deploy --dry-run
npm run deploy
```

The official deployment binds `yap-server.mmstudio.games` and
`yap.meaninglessmeaning.studio` as custom domains on the same Worker. The old
`workers.dev` endpoints stay enabled for existing clients and invitations.
When deploying your own Worker, use your own custom domain in the menu, or its
`wss://...workers.dev` address if you have not configured a domain.
`assets/server-url.txt` supplies the compiled default. `/health` is the health
endpoint, `/rooms` returns the current lobby, `/lobby` serves the same listing
over WebSocket, and `/room/ABCDEFGH` is a room connection. A listing now includes
each room's `capacity`; missing capacity on older servers is treated as unknown.
`/lobby?probe=1` advertises `probe: true`, waits up to three seconds for the text
`ping`, replies `pong`, then closes normally. It uses the same authentication,
origin and connection limits as lobby discovery, never joins a room or changes
player counts. Plain `/lobby` retains its immediate-list-and-close behavior.
Deploy the Rust container first, verify it, and then switch the old gateway while
both room listings have no active players. The retired Room/Lobby namespaces are
kept with inactive exports for rollback; no new traffic reaches them and their
old alarms do not reschedule. See [Official town deployment](../server/club/README.md).

## Checks

The opt-in native Club check starts two isolated real Rust servers and drives the
Bevy UI through adding, editing and removing Clubs, masked IME input, both
languages, scrolling, automatic room/player refresh, measured ping, joining the
correct server and reloading preferences. It stores screenshots and its result
in `artifacts/clubs/` without changing normal saves:

```sh
cargo build --locked
YAPSHIRE_SMOKE=clubs \
YAPSHIRE_SETTINGS_DIR="$PWD/artifacts/clubs/settings" \
YAPSHIRE_MAP_DIR="$PWD/artifacts/clubs/maps" \
YAPSHIRE_SAVE_DIR="$PWD/artifacts/clubs/save" \
  target/debug/yapshire
```

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
# Against a running local Worker:
cd server && npm test
# Against the deployed Worker (Node 24+):
SERVER_URL=wss://yap-server.mmstudio.games \
  GUEST_SERVER_URL=wss://yap.meaninglessmeaning.studio \
  node --use-env-proxy test/rooms.mjs
```

The Rust tests also cover tackle purchases, insufficient funds, repeated purchases,
bait consumption, selling, save replacement and validation, and fishing wins/losses
at 30, 60 and 144 simulation steps per second.
The Rust tests cover movement, jumping, map bounds, actual Bevy keyboard event
delivery, Unicode editing, bubble limits, UDP discovery, two real LAN sockets,
and bounded TLS failure handling. Dedicated-server integration tests cover edited
map transfer, acknowledgement, passwords, origins, admission races, capacity,
room isolation, message size/rate limits, cleanup, shutdown and CLI initialization.
The gateway test covers map synchronization, discovery, lobby cleanup, room isolation, identity, movement,
shop and fishing flags, invalid activity input,
Unicode chat, disconnects, missing rooms, and oversize input.

To exercise the exact Rust client's WSS, proxy, hosting, lobby, joining, movement,
chat, and disconnect path against a real Worker, from the project root:

```sh
YAPSHIRE_TEST_SERVER=wss://yap-server.mmstudio.games \
  cargo test --locked cloud_client -- --ignored --nocapture
```

## GPU acceptance and artwork

On Linux, run the complete fishing loop with a fresh, isolated save directory:

```sh
YAPSHIRE_SMOKE=fishing YAPSHIRE_SAVE_DIR="$(mktemp -d)" cargo run --locked
# With the local Worker running:
YAPSHIRE_SMOKE=fishing-cloud YAPSHIRE_TEST_SERVER=ws://127.0.0.1:8787 \
  YAPSHIRE_SAVE_DIR="$(mktemp -d)" cargo run --locked
```

This debug-only driver uses the real game controls to walk into the shop, buy
tackle and bait, reach the pier, miss one bite, catch a fish by controlling line
tension, return to the counter, sell it, and verify the saved balance. It saves
actual screenshots, including the street-to-shop and quay-to-pier transitions, under `artifacts/fishing-*.png` or
`artifacts/fishing-cloud-*.png`. Use a fresh save directory for every run.
The LAN run uses local port 4777.

`YAPSHIRE_SMOKE=display cargo run --locked` checks a fixed window, F11 fullscreen,
and the restored window size, saving screenshots of all three stages under
`artifacts/`. This uses the real window backend and GPU, with no desktop input injection.

`YAPSHIRE_SMOKE=i18n-scale` exercises the settings buttons, all three pixel scales,
fullscreen, the language switch and editor pointer mapping. Follow it with
`YAPSHIRE_SMOKE=i18n-reload` using the same isolated settings directory to verify
that Chinese and 4x are restored. Set separate `YAPSHIRE_SETTINGS_DIR`,
`YAPSHIRE_MAP_DIR` and `YAPSHIRE_SAVE_DIR` directories for this check.

For a full native GPU acceptance run, launch two debug builds with
`YAPSHIRE_SMOKE=host-lan` and `YAPSHIRE_SMOKE=guest-lan` (or `host-cloud` and
`guest-cloud`). They exercise the actual menu actions, discover and join a room,
inject Bevy keyboard events for walking/jumping/chat, assert movement and both
chat deliveries, save screenshots, then close. This opt-in driver bypasses only
window focus gating; it never sends keys to other desktop applications.
Set `YAPSHIRE_TEST_SERVER=ws://127.0.0.1:4761` on both cloud-mode processes to test
a dedicated server, with `YAPSHIRE_TEST_PASSWORD` if needed. Set
`YAPSHIRE_TEST_MAPS` to that server's map directory to assert that the actual
runtime world matches both transferred maps. The driver also verifies that
leaving restores each client's previous local maps. Use isolated
`YAPSHIRE_SETTINGS_DIR`, `YAPSHIRE_MAP_DIR` and `YAPSHIRE_SAVE_DIR` folders.

To record the same real session for README media, set
`YAPSHIRE_RECORD=artifacts/recording` on the host process. The debug driver saves
15 FPS PNG frames while the two players walk, jump, and chat. Use a new empty
directory for each recording. For example, launch these in separate terminals:

```sh
YAPSHIRE_SMOKE=host-cloud YAPSHIRE_RECORD=artifacts/recording cargo run --locked
YAPSHIRE_SMOKE=guest-cloud cargo run --locked
```

The English and Chinese READMEs are recorded separately from actual connected
clients using their respective interface language. Banner lettering is stored as
SVG paths, so Chinese text needs no installed fonts.

- Bevy **0.18.1** and [bevy_ecs_tilemap **0.18.1**](https://github.com/StarArawn/bevy_ecs_tilemap).
- A world render texture selected in Settings: 720 × 405 at 2x (default),
  480 × 270 at 3x, or 360 × 202 at 4x. Use nearest-neighbor sampling and integer
  pixel scaling; high-resolution UI still uses the bundled pixel font. The 4x
  canvas leaves a one-pixel border above and below in the fixed window.
- Original 16-pixel terrain tiles, four 20 × 32 characters with six animation
  frames each, a layered autumn landscape, drifting clouds and falling leaves.
- The illustrated lake panorama uses nearest-neighbor sampling through the same
  world canvas. See [art direction and source ownership](ART_DIRECTION.md).
- [Fusion Pixel Font](https://github.com/TakWolf/fusion-pixel-font), 12px monospaced
  Latin variant with CJK coverage, release 2026.09.25. Font and upstream licenses
  are in `assets/fonts/`.
- WebSocket architecture follows Cloudflare's
  [Durable Object hibernation API](https://developers.cloudflare.com/durable-objects/examples/websocket-hibernation-server/).

The checked-in PNGs are ready to run. To regenerate the editable foreground,
official content pack and UI art (this replaces official pack layouts):

```sh
uv run --with Pillow tools/draw_assets.py
uv run --with Pillow tools/draw_maps.py
uv run --with Pillow tools/draw_fishing.py
uv run --with Pillow tools/draw_editor_icons.py
uv run --with Pillow tools/draw_readme.py
```

The panorama `assets/hills.png` is a separately generated illustration and is
preserved by these commands. `assets/maps/` contains frozen legacy migration
fixtures; the pack generator never rewrites them. The official art pack is now
version 1.1.0; peers need matching definitions and images. Tile IDs, walking routes
and interactions are unchanged from pack 1.0.0; the decorative lighthouse now
sits at the lake horizon.

This release's native GPU gameplay, editor and language UI are verified on Linux.
GitHub Actions builds and tests Windows, Linux, and both macOS architectures.
GUI play on Windows/macOS and sessions between two separate
physical LAN machines still need manual verification.

## Cross-platform CI

The workflow layout follows [IntelligentMixVideo](https://github.com/HsiangNianian/IntelligentMixVideo):
a reusable build matrix, separate checks, and a tag-triggered release workflow.

| Event | Result |
| --- | --- |
| Main branch push / pull request | Project checks, native tests, eight game/server archives and Docker checks |
| Documentation-only push / PR | Build skipped |
| Manual **CI** | Full checks, eight archives and Docker checks |
| Manual **Build game and server** | Eight platform archives |
| `vX.Y.Z` tag | Checks, eight archives, AMD64/ARM64 GHCR image, verified Release and CHANGELOG update |

All builds use lockfiles. Actions artifacts are retained for 14 days. Both CI and
releases call the same build workflow; release binaries come from the tagged
commit. Normal jobs use read permissions; only the release publishing job can
write repository contents. The container publishing job separately receives
`packages: write`; ordinary container checks need only read access.

Local checks:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
node --test .github/scripts/*.test.mjs
python3 -m unittest discover -s tools -p 'test_*.py'
# With a local Worker already running:
npm test --prefix server
```

## Releases and changelog

Game, shared crate, dedicated server and Worker versions move together. Prepare a version from the repository
root, update both README download tables and add the version entry to
`CHANGELOG.md`, then review and commit the changes before tagging:

```sh
RELEASE_TAG=v0.7.2 node .github/scripts/validate-release.mjs --write
node .github/scripts/validate-release.mjs
git add Cargo.toml Cargo.lock crates/*/Cargo.toml server/package.json server/package-lock.json
git add README.md README.zh-CN.md CHANGELOG.md
git commit -m "chore: release v0.7.2"
# Once the release commit is on main:
git tag -a v0.7.2 -m "Release v0.7.2"
git push --atomic origin main v0.7.2
```

For an unchanged first version, skip the empty version commit. PowerShell users
can set `$env:RELEASE_TAG = "v0.7.2"` before running the same Node command.
Only stable `vX.Y.Z` tags are accepted. CI rejects source/tag version mismatches;
it never changes source versions during a release.

[`release.yml`](../.github/workflows/release.yml) reads Release Notes directly
from the tagged changelog entry. Preparing the entry before tagging keeps the
notes, uploaded changelog and documentation inside every archive identical.
If the entry is absent, the workflow generates it from Conventional Commits
using the same changelog action as IntelligentMixVideo; that fallback entry
cannot appear in archives already built from the tag. The generated range runs
from the preceding ancestor version tag to the new tag. The first release uses
the empty repository bootstrap commit as its baseline.

After every build and both published container architecture checks succeed,
the workflow uploads eight game/server archives,
`SHA256SUMS`, and `CHANGELOG.md` to a **draft**. It verifies the uploaded names,
sizes and available digests before publishing. Release Notes and the changelog
entry use the same text. Then a bot merges any missing entry into
the latest default branch, preserving concurrent edits and existing releases.

Failed drafts can be retried. Published releases remain unchanged; a retry can
repair changelog writeback using the already published attachment. No personal
token is required: the built-in `GITHUB_TOKEN` handles publishing. Branch rules
must allow its changelog commit. Tagged releases do not deploy the Worker.

## License

Project code and original artwork: [AGPL-3.0-only](../LICENSE.md).
The bundled font and its upstream notices retain their licenses in `assets/fonts/`.
