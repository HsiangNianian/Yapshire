# Host your own Yapshire town

[简体中文](SELF_HOSTING.zh-CN.md) · [README](../README.md)

`yapshire-server` runs without a game window, GPU, Node.js or Cloudflare account.
Use it on a home computer, VPS, or Docker host. In v0.7.x and later clients,
players choose **Online lobby → Add Club**, enter its address and optionally give
it a personal alias. Each saved Club automatically lists its rooms, occupancy and
measured latency. Select a Club to create a room there; editing or removing the
subscription does not rename or stop the actual server. Clients and servers must
use v0.7.x builds with matching content packs (protocol 3); older clients need to update first.

The official **Yapshire Town (Yapshire 小镇)** uses
`wss://yap-server.mmstudio.games`; `wss://yap.meaninglessmeaning.studio` reaches
the same town. Enter either address in a v0.7.x client.

## Start with the standalone download

Download the **yapshire-server** archive for your system from
[Releases](https://github.com/HsiangNianian/Yapshire/releases/latest). Builds are
available for Windows x64, Linux x64 (glibc 2.35+), and macOS Intel / Apple Silicon.
Extract the entire archive. In a terminal inside that folder, run:

```sh
./yapshire-server
```

On Windows, run `./yapshire-server.exe` in PowerShell. The macOS server is a console executable,
not an `.app`. The downloads are unsigned. A server does not need the game's
desktop libraries; Linux needs glibc and the usual C runtime.

The archive includes `server.json` and a `maps/` directory. The server reads
`server.json` in its working directory automatically. Without that file, the
executable runs its embedded original town, so it also works on its own.

The default town is **My Yapshire town**, code **MAIN0001**, listening on
**TCP 4761**. On the same computer, enter `ws://127.0.0.1:4761` in the game's
**Online lobby → Add Club** form, then choose that town from the lobby. On your local
network, use the server computer's LAN address, such as `ws://192.168.1.10:4761`.
The dedicated server uses **Online lobby**; automatic **Join LAN** discovery is
for rooms hosted inside the game.

Ctrl+C stops the server and disconnects its players. The configured town remains
listed while empty, and comes back on restart. Extra rooms created from the game
are removed when their last player leaves. All rooms on one server use its maps.

## Docker

The public image supports **linux/amd64** and **linux/arm64**:

```sh
docker run -d --name yapshire --restart unless-stopped \
  -p 4761:4761 ghcr.io/hsiangnianian/yapshire-server:latest
```

Use a version tag such as `:v0.7.2` to pin a release. The repository also includes
[`compose.yaml`](../compose.yaml): `docker compose up -d` starts the default town.
The image runs as UID/GID **10001**, includes an HTTP health check, and reads
configuration from `/data`. It never needs to write to your maps.

To create editable files without a native executable:

```sh
docker run --name yapshire-init ghcr.io/hsiangnianian/yapshire-server:latest --init /tmp/my-town
docker cp yapshire-init:/tmp/my-town ./my-town
docker rm yapshire-init
```

Start a server with those files mounted read-only:

```sh
docker run -d --name yapshire-custom --restart unless-stopped \
  -p 4761:4761 --read-only --cap-drop ALL \
  --security-opt no-new-privileges:true \
  --mount "type=bind,src=$PWD/my-town,dst=/data,readonly" \
  ghcr.io/hsiangnianian/yapshire-server:v0.7.2
```

Ensure the directory and files are readable by UID 10001. In Compose, enable the
commented `./my-town:/data:ro` mount. Use a different host port if another game
or server already uses 4761. Keep the container's port at 4761 for its built-in
health check; for a custom internal port, override the health check as well.

## Use maps and content packs

The following describes **v0.7.x** (protocol 3). Client and server must use
matching content packs. v0.7.1 and v0.7.2 can play together. Earlier releases use
protocol 2 and cannot join these rooms.

1. Run `./yapshire-server --init ./my-town`. This creates `server.json` and a
   complete official pack under `my-town/maps/`, without overwriting files.
2. Open the game's **Map editor**, edit and **Save**, then choose **Map files**.
3. Copy the pack folder's contents into `my-town/maps/`. The manifest is
   `my-town/maps/pack.json`; layouts are `my-town/maps/maps/*.tmj`.
4. Validate, then start:

```sh
./yapshire-server --config ./my-town/server.json --check
./yapshire-server --config ./my-town/server.json
```

`maps_dir` and `--maps` accept a content-pack root. Old folders containing the
original `town.tmj`, `tackle-shop.tmj` and `harbor.*` still import without changing
their files. The editor, client and server share the same validation module.
See [Content packs v1](CONTENT_PACKS.md) for stable IDs, Tiled authoring, variable
maps, interactions, collisions, terrain, size limits and migration.

The server transfers layouts and pack definitions, and the client checks its
installed pack ID/version, image hashes and world revision **before joining**.
Install/select custom packs on every client first; assets are not downloaded
from room servers. Layout-only edits need no client rebuild. Remote maps stay
in memory; leaving restores local maps and never overwrites editor files.

LAN hosts use the same protocol and checks. Cloudflare gateways forward to the
same Rust server; update their container with the client when moving to protocol
3. This source change does not update the running public service automatically.

## Configuration

All fields are optional. Paths in `server.json` are relative to that config file;
a CLI `--maps` path is relative to the process's working directory.

```json
{
  "bind": "0.0.0.0:4761",
  "name": "My Yapshire town",
  "room_code": "MAIN0001",
  "maps_dir": "maps",
  "max_players": 16,
  "max_rooms": 40,
  "max_connections_per_ip": 32,
  "allow_room_creation": true,
  "allowed_origins": []
}
```

`room_code` is eight uppercase letters or digits, `name` is at most 24 characters,
`max_players` is 1–16, `max_rooms` is 1–40, and `max_connections_per_ip` is 1–256.
Set `allow_room_creation` to `false` to offer only the configured town. Set
`maps_dir` to `null` to use embedded maps. Invalid or unknown options fail early.
`--bind`, `--maps`, `--config`, `--check`, `--init`, `--help`, and `--version` are
available; `--init` is used by itself.

## Optional shared password

Set **YAPSHIRE_SERVER_PASSWORD** to an 8–128 byte password in the server's
environment. Players enter it in the masked **Server password** field. It is
kept only for that game process, never saved in settings or copied with an invite.
Changing an address clears its password. Saved Clubs retain their own passwords
only in memory until the game exits; switching Clubs never copies one server's
password to another. Share passwords separately from room codes.

In Bash or zsh, read the password without echoing it or putting it in history:

```sh
read -rs YAPSHIRE_SERVER_PASSWORD
export YAPSHIRE_SERVER_PASSWORD
./yapshire-server --config ./my-town/server.json
```

For Docker, set the variable in the shell first, then pass
`--env YAPSHIRE_SERVER_PASSWORD` to `docker run`. Compose reads the same variable.
Container administrators can inspect environment variables, so protect access
to the Docker host. A password protects both joining and lobby listing. `/health`
remains public and contains only status, version, protocol and map revision.

This is a shared server password, not per-player accounts or moderation. The
wire header is `Authorization: Bearer <SHA-256(password)>`; that digest is itself
a credential. It is never placed in a URL. Use TLS for public connections.

## Internet access and TLS

Permit the chosen TCP port in the server firewall. A home connection also needs
router port forwarding; carrier-grade NAT may require a VPS or a VPN with peer
connectivity. Friends need your server address **and** a room code, plus the
password if enabled. **Copy invite** copies a complete server-and-room URL;
paste it into **Room code or invitation** to fill both fields. The password
must be shared separately. Previously released clients copy only the room code.

For a public domain, bind the server to `127.0.0.1:4761` and terminate TLS at your
reverse proxy. For example, a [Caddy reverse proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
can forward WebSockets:

```caddyfile
town.example.com {
    reverse_proxy 127.0.0.1:4761
}
```

Players then enter `wss://town.example.com`. Use a valid certificate, preserve
the `/health`, `/rooms`, `/lobby` and `/room/*` paths, and forward the Authorization
header. With Docker, publish `127.0.0.1:4761:4761` when the proxy runs on the host.

The native client sends no browser Origin. Browser WebSocket origins are rejected
unless explicitly listed in `allowed_origins`. The server intentionally ignores
forwarded IP headers: per-address limits use the actual TCP peer. Behind a proxy,
clients share that proxy's IP budget (300 connection/list requests per minute and
the configured connection limit). For larger deployments, apply limits at the
proxy and plan capacity around this shared budget.

Each socket has bounded messages (2 KiB), rate (80 messages/second), an outgoing
queue, a 10-second map acknowledgement deadline and a 45-second heartbeat
deadline. The server assigns player IDs, clamps movement and cleans text. A slow,
malformed or flooding peer is disconnected without taking down other rooms.
This is a social game with client-driven movement, not competitive anti-cheat.
Wallets and fishing saves remain on each player's computer; positions and chat
history are not persisted by the server.

## Build and verify

```sh
cargo build --release --locked -p yapshire-server
cargo test --locked -p yapshire-server -p yapshire-shared
docker build -t yapshire-server:local .
python3 tools/check_container.py yapshire-server:local
```

The server build does not compile Bevy or require GUI libraries. CI tests real
password-protected connections and edited maps, builds all four native server
archives alongside the clients, and checks both container architectures before
publishing the GitHub release. Versioned GHCR images carry the source revision;
release archives have `SHA256SUMS`.
