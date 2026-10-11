import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";
import { readFileSync } from "node:fs";

const limits = JSON.parse(readFileSync(new URL("../club/server.json", import.meta.url), "utf8"));

const base = (process.env.CLUB_URL || "http://127.0.0.1:8788").replace(/\/$/, "");
const wsBase = base.replace(/^http/, "ws");
const clients = [];

async function request(path, options = {}) {
  return fetch(base + path, { ...options, signal: AbortSignal.timeout(30_000) });
}

function client(path) {
  const ws = new WebSocket(wsBase + path);
  const messages = [];
  const heartbeat = setInterval(() => {
    if (ws.readyState === WebSocket.OPEN) ws.send("ping");
  }, 15_000);
  heartbeat.unref();
  let closeReason = "";
  ws.addEventListener("close", ({ code, reason }) => {
    closeReason = `${code}: ${reason}`;
    clearInterval(heartbeat);
  });
  ws.addEventListener("message", ({ data }) => {
    if (data !== "pong") messages.push(JSON.parse(data));
  });
  const closed = new Promise((resolve) => ws.addEventListener("close", resolve, { once: true }));
  const ready = new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("WebSocket connection timed out")), 30_000);
    ws.addEventListener("open", () => { clearTimeout(timer); resolve(); }, { once: true });
    ws.addEventListener("error", () => { clearTimeout(timer); reject(new Error("WebSocket rejected")); }, { once: true });
  });
  const result = {
    ws, ready, closed,
    get closeReason() { return closeReason; },
    send: (value) => ws.send(JSON.stringify(value)),
    async next(type, predicate = () => true) {
      const deadline = Date.now() + 10_000;
      while (Date.now() < deadline) {
        const index = messages.findIndex((message) => message.type === type && predicate(message));
        if (index !== -1) return messages.splice(index, 1)[0];
        assert.notEqual(ws.readyState, WebSocket.CLOSED, `Socket closed waiting for ${type} (${closeReason})`);
        await delay(20);
      }
      throw new Error(`Timed out waiting for ${type}`);
    },
  };
  clients.push(result);
  return result;
}

try {
  assert.match(await (await request("/")).text(), /Yapshire 小镇/);
  assert.equal((await request("/unknown")).status, 404);
  assert.equal((await request("/room/invalid")).status, 404);
  assert.equal((await request("/rooms", { method: "POST" })).status, 405);
  assert.equal((await request("/rooms", { headers: { Origin: "https://untrusted.example" } })).status, 403);
  assert.equal((await request("/lobby")).status, 426);
  assert.equal((await request("/rooms?" + "x".repeat(1025))).status, 414);

  const health = await (await request("/health")).json();
  assert.equal(health.ok, true);
  assert.equal(health.protocol, 3);
  const listing = await (await request("/rooms")).json();
  const town = listing.rooms.find(({ code }) => code === "NIANNIAN");
  assert.equal(town.name, "Yapshire 小镇");
  const initialPlayers = town.players;
  assert.equal(town.capacity, limits.max_players);
  assert(initialPlayers <= 14, "Two player slots are required for this smoke check");

  const lobby = client("/lobby");
  await lobby.ready;
  const lobbyClose = await lobby.closed;
  assert.equal(lobbyClose.code, 1000);

  const probe = client("/lobby?probe=1");
  await probe.ready;
  const measuredListing = await probe.next(undefined, (message) => message.probe === true);
  assert.equal(measuredListing.rooms.find(({ code }) => code === "NIANNIAN").players, initialPlayers);
  const pong = new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("Lobby ping timed out")), 5000);
    probe.ws.addEventListener("message", ({ data }) => {
      if (data === "pong") { clearTimeout(timer); resolve(); }
    });
  });
  const pingStarted = performance.now();
  probe.ws.send("ping");
  await pong;
  const latencyMs = Math.round(performance.now() - pingStarted);
  assert.equal((await probe.closed).code, 1000);
  const afterProbe = await (await request("/rooms")).json();
  assert.equal(afterProbe.rooms.find(({ code }) => code === "NIANNIAN").players, initialPlayers);

  const joined = [];
  for (const name of ["Club QA A", "Club QA B"]) {
    const peer = client(`/room/NIANNIAN?protocol=3&name=${encodeURIComponent(name)}`);
    await peer.ready;
    const { world } = await peer.next("world");
    assert.equal(world.revision, health.world);
    assert.equal(world.format, 2);
    peer.send({ type: "world_ready", revision: world.revision });
    const welcome = await peer.next("welcome");
    joined.push({ peer, id: welcome.you, world });
  }
  const [a, b] = joined;
  assert.notEqual(a.id, b.id);
  assert.deepEqual(a.world, b.world);
  await a.peer.next("joined", ({ player }) => player.id === b.id);

  a.peer.send({ type: "chat", text: "Yapshire 小镇联机验证" });
  for (const { peer } of joined) {
    const chat = await peer.next("chat", ({ id }) => id === a.id);
    assert.equal(chat.text, "Yapshire 小镇联机验证");
  }
  a.peer.send({ type: "move", map: "yapshire:tackle_shop", x: 400, y: 20, moving: true, facing: false, indoors: true, fishing: false });
  const movement = await b.peer.next("moved", ({ player }) => player.id === a.id);
  assert.equal(movement.player.x, 400);
  assert.equal(movement.player.indoors, true);
  const populated = await (await request("/rooms")).json();
  assert.equal(populated.rooms.find(({ code }) => code === "NIANNIAN").players, initialPlayers + 2);

  a.peer.ws.close(1000, "Smoke test complete");
  await b.peer.next("left", ({ id }) => id === a.id);
  b.peer.ws.close(1000, "Smoke test complete");
  await Promise.all(joined.map(({ peer }) => peer.closed));

  const capacity = process.env.CLUB_CAPACITY_CHECK === "1";
  if (capacity) {
    const before = await (await request("/rooms")).json();
    assert(before.rooms.length === 1 && before.rooms[0].players === 0,
      "Capacity checks require an empty server; never displace real players");
    const codes = ["NIANNIAN", ...Array.from({ length: limits.max_rooms - 1 }, () =>
      crypto.randomUUID().replaceAll("-", "").slice(0, 8).toUpperCase())];
    const population = [];
    for (const code of codes) {
      const peers = [];
      for (let player = 0; player < 16; player++) {
        const create = code !== "NIANNIAN" && player === 0;
        const peer = client(`/room/${code}?protocol=3&name=Capacity${player}&create=${Number(create)}&room_name=Capacity%20QA`);
        await peer.ready;
        const { world } = await peer.next("world");
        peer.send({ type: "world_ready", revision: world.revision });
        const { you } = await peer.next("welcome");
        peers.push({ peer, id: you });
      }
      population.push(peers);
    }
    const full = await (await request("/rooms")).json();
    assert.equal(full.rooms.length, limits.max_rooms);
    assert(full.rooms.every(({ players }) => players === 16));
    await assert.rejects(client("/room/NIANNIAN?protocol=3&name=Overflow").ready);
    const extra = crypto.randomUUID().replaceAll("-", "").slice(0, 8).toUpperCase();
    await assert.rejects(client(`/room/${extra}?protocol=3&create=1&name=Overflow&room_name=Overflow`).ready);

    // Exercise the configured capacity for five seconds at the game's 20 Hz.
    for (let frame = 0; frame < 100; frame++) {
      for (const peers of population) for (const { peer } of peers) {
        peer.send({ type: "move", map: "yapshire:town", x: 400 + frame, y: 0, moving: true, facing: false });
      }
      await delay(50);
    }
    for (const peers of population) {
      await peers[1].peer.next("moved", ({ player }) => player.id === peers[0].id && player.x === 499);
      assert(peers.every(({ peer }) => peer.ws.readyState === WebSocket.OPEN),
        `Capacity connections closed: ${peers.map(({ peer }) => peer.closeReason).filter(Boolean).join(", ")}`);
      for (const { peer } of peers) peer.ws.close(1000, "Capacity check complete");
    }
    await Promise.all(population.flat().map(({ peer }) => peer.closed));
    const after = await (await request("/rooms")).json();
    assert.deepEqual(after.rooms.map(({ code, players }) => ({ code, players })), [{ code: "NIANNIAN", players: 0 }]);
  }
  console.log(JSON.stringify({
    result: "passed", server: base, name: town.name,
    version: health.version, world: health.world,
    latencyMs,
    checks: ["routes", "origin", "query limit", "legacy lobby", "lobby round trip without joining", "room capacity", "shared maps", "two players", "Chinese chat", "movement", "disconnect", ...(capacity ? [`${limits.max_rooms} rooms`, `${limits.max_rooms * 16} players at 20 Hz for five seconds`, "room and player limits", "empty room cleanup"] : [])],
  }, null, 2));
} finally {
  for (const { ws } of clients) {
    if (ws.readyState === WebSocket.OPEN) ws.close(1000, "Smoke test complete");
  }
}
