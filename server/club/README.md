# Yapshire Town (Yapshire 小镇)

Cloudflare Containers deployment of the workspace's Yapshire Rust server, shared by
the official domains and its compatibility gateway. Players can join the fixed
**Yapshire 小镇** room with code **NIANNIAN**, or create one temporary room.
Each room allows 16 players; this initial deployment has **two rooms total**,
uses the bundled maps, and has no password. A temporary room disappears when its
last player leaves; the configured town stays listed.

Default address: **`wss://yap-server.mmstudio.games`**.
Alternate address: **`wss://yap.meaninglessmeaning.studio`**.
Both domains connect directly to the same Worker and share its rooms and players.
The deployment is pinned to the `opensource@mmstudio.games` Cloudflare account in
`wrangler.jsonc`; change `account_id`, names and `routes` when deploying a separate
copy elsewhere. Each custom domain requires an active zone in the target account.
Wrangler configures its DNS record and certificate during deployment; see
[Custom Domains](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/).

The Worker name `niannian-club`, Durable Object identity and room code `NIANNIAN`
stay unchanged to preserve the existing instance and invitations. The previous
`wss://niannian-club.opensource-941.workers.dev` endpoint also stays enabled for
existing clients; new clients use the custom domain by default and migrate saved
official addresses without changing custom server choices.

The previous `wss://yapshire-multiplayer.opensource-941.workers.dev` address uses
a [service binding](https://developers.cloudflare.com/workers/runtime-apis/bindings/service-bindings/http/)
to reach the same server. It has no separate lobby or game implementation.
Use v0.7.x clients with protocol 3 and content pack 1.1.0. Older clients use
protocol 2 and must upgrade before joining the updated official server.

The Worker routes every game request to the same Durable Object. The `default`
container scheduling policy enforces `max_instances: 1`, using the `lite` size
(256 MiB RAM, 1/16 vCPU). The container can stop two minutes after its Durable
Object becomes inactive; open game WebSockets keep it active. The next connection
starts it again, and the configured room is recreated automatically.

The Dockerfile builds the workspace's Rust server, reuses the pinned v0.5.2
runtime image, and includes `server.json`. No player
positions or chat history are persisted across restarts. Custom maps can be
included in the image and selected with `maps_dir`; do not rely on runtime edits
to the container filesystem surviving a restart.

The container sees proxied connections from the gateway, so its configured
`max_connections_per_ip: 40` leaves room for 32 players plus lobby/handshake
connections. It is effectively a shared connection ceiling behind this proxy,
not a per-player public-IP quota. The existing 300 requests/minute address limit
and per-socket message limits still apply. Room limits are admission limits,
not a guarantee of sustained performance at maximum load.

From the repository root:

```sh
npm --prefix server ci
npm --prefix server run dev:club
```

In another terminal:

```sh
npm --prefix server run test:club
```

Deploy using the authenticated Cloudflare account, which must have Workers Paid:

```sh
npm --prefix server run deploy:club -- --dry-run
npm --prefix server run deploy:club
```

Use either custom domain under **Online lobby**. The HTTP root shows connection
instructions, `/health`
checks the Rust server, and `/rooms` lists the configured and temporary rooms
with their player capacities. `/lobby?probe=1` supports a single `ping`/`pong`
round trip for the Club directory without joining a room or changing occupancy.
Plain `/lobby` remains compatible with older clients.

For a migration, check `/rooms` at both public addresses first and deploy when
they have no active players. Verify the new container, then run
`npm --prefix server run deploy` to switch the old-address gateway. The old Room
and Lobby namespaces remain inactive so the previous deployment can be restored;
do not delete their namespaces as part of a routine rollback. Check both
addresses after switching, including creating through one and joining through
the other. Changing a container image restarts its in-memory rooms.

To verify a deployed instance, run the following with Node.js 24 or newer when an
environment proxy is needed:

```sh
CLUB_URL=https://yap-server.mmstudio.games \
  node --use-env-proxy server/test/club.mjs
```

The smoke check verifies the measured lobby probe and unchanged player counts,
then temporarily joins two players, sends a chat message, moves one player, and
verifies departure. Run it while two slots are available.

For an empty development instance, set `CLUB_CAPACITY_CHECK=1` as well. This
fills both rooms, verifies the room/player limits, moves all 32 players at 20 Hz
for five seconds, and checks cleanup. It deliberately fails if real players are
already present. Local simulation does not reproduce Cloudflare CPU limits, so
verify the intended deployment too before relying on a higher capacity.

The normal CI check runs both gateways locally using `npm run dev`, then runs
the room protocol tests and this capacity check. `GUEST_SERVER_URL` on
`test/rooms.mjs` can point to the compatibility address to verify shared rooms
across the two addresses.

Cloudflare charges the account's Workers plan plus usage. A single small instance
and idle shutdown control usage, but they do not impose a monetary spending cap.
See [Containers pricing](https://developers.cloudflare.com/containers/platform/pricing/).
