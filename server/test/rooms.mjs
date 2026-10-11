import assert from "node:assert/strict";

const base = (process.env.SERVER_URL || "ws://127.0.0.1:8787").replace(/\/$/, "").replace(/^http/, "ws");
const guestBase = (process.env.GUEST_SERVER_URL || base).replace(/\/$/, "").replace(/^http/, "ws");
const room = () => crypto.randomUUID().replaceAll("-", "").slice(0, 8).toUpperCase();
const sockets = [];
async function join(code, name, create = false, title = "Sunset & Friends", address = base) {
  const ws = new WebSocket(`${address}/room/${code}?protocol=3&name=${encodeURIComponent(name)}&create=${create ? 1 : 0}&room_name=${encodeURIComponent(title)}`);
  sockets.push(ws);
  const messages = [];
  ws.addEventListener("message", (e) => { if (e.data !== "pong") messages.push(JSON.parse(e.data)); });
  const next = async (type) => {
    const deadline = Date.now() + 10000;
    while (Date.now() < deadline) {
      const index = messages.findIndex((m) => m.type === type);
      if (index !== -1) return messages.splice(index, 1)[0];
      await new Promise((r) => setTimeout(r, 20));
    }
    throw new Error(`Timed out waiting for ${type}: ${JSON.stringify(messages)}`);
  };
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("Connection timeout")), 10000);
    ws.addEventListener("open", () => { clearTimeout(timer); resolve(); }, { once: true });
    ws.addEventListener("error", () => { clearTimeout(timer); reject(new Error(`Connection rejected: ${address}, room ${code}, create=${create}`)); }, { once: true });
  });
  const { world } = await next("world");
  assert.equal(world.format, 2);
  ws.send(JSON.stringify({ type: "world_ready", revision: world.revision }));
  return { ws, next, messages, send: (data) => ws.send(JSON.stringify(data)) };
}

try {
  const code = room();
  const a = await join(code, "小风", true);
  const wa = await a.next("welcome");
  assert.equal(wa.players[0].name, "小风");
  const b = await join(code, "小雨", false, "Sunset & Friends", guestBase);
  const wb = await b.next("welcome");
  assert.equal(wb.players.length, 2);
  const listing = await fetch(`${base.replace(/^ws/, "http")}/rooms`).then((r) => r.json());
  assert.equal(listing.rooms.find((r) => r.code === code).players, 2);
  assert.equal(listing.rooms.find((r) => r.code === code).name, "Sunset & Friends");
  assert.notEqual(wa.you, wb.you);
  assert.equal((await a.next("joined")).player.id, wb.you);
  a.send({ type: "move", map: "yapshire:town", x: 600, y: 24, moving: true, facing: true });
  const moved = await b.next("moved");
  assert.equal(moved.player.id, wa.you);
  assert.equal(moved.player.x, 600);
  assert.equal(moved.player.facing, true);
  assert.equal(moved.player.indoors, false, "old movement packets default to outdoors");
  a.send({ type: "move", map: "yapshire:tackle_shop", x: 270, y: 0, moving: false, facing: false, indoors: true, fishing: true });
  const shopping = (await b.next("moved")).player;
  assert.equal(shopping.indoors, true);
  assert.equal(shopping.fishing, false, "indoor players cannot display a fishing line");
  a.send({ type: "move", map: "yapshire:town", x: 1320, y: 0, moving: false, facing: false, indoors: false, fishing: true });
  assert.equal((await b.next("moved")).player.fishing, true);
  a.send({ type: "chat", text: "你好，远方的朋友！", id: wb.you });
  assert.equal((await b.next("chat")).id, wa.you, "identity must come from socket, not payload");
  assert.equal((await a.next("chat")).text, "你好，远方的朋友！");
  a.send({ type: "move", map: "yapshire:town", x: 9999999, y: -2, moving: false, facing: false });
  assert.equal((await b.next("moved")).player.x, 1428);
  const c = await join("NIANNIAN", "隔壁房间");
  assert.equal((await c.next("welcome")).players.length, 1);
  a.send({ type: "move", map: "yapshire:town", x: 800, y: 0, moving: false, facing: false });
  await b.next("moved");
  assert.equal(c.messages.length, 0, "rooms must be isolated");
  a.ws.close();
  assert.equal((await b.next("left")).id, wa.you);
  b.send({ type: "chat", text: "房主离开后房间仍然可用" });
  assert.equal((await b.next("chat")).id, wb.you);
  await assert.rejects(join(room(), "不存在"));
  await assert.rejects(join(room(), "Invalid title", true, " \n\t "));
  await assert.rejects(join(room(), "Oversize title", true, "x".repeat(257)));
  b.send({ type: "chat", text: "x".repeat(3000) });
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("Oversize message was not closed")), 10000);
    b.ws.addEventListener("close", (e) => { clearTimeout(timer); assert.equal(e.code, 1009); resolve(); });
  });
  const after = await fetch(`${base.replace(/^ws/, "http")}/rooms`).then((r) => r.json());
  assert.equal(after.rooms.some((r) => r.code === code), false, "empty rooms disappear from lobby");
  const invalid = await join(room(), "Invalid activity", true);
  await invalid.next("welcome");
  const invalidClosed = new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("Invalid activity was not closed")), 10000);
    invalid.ws.addEventListener("close", (e) => { clearTimeout(timer); assert.equal(e.code, 1007); resolve(); });
  });
  invalid.send({ type: "move", map: "yapshire:tackle_shop", x: 270, y: 0, moving: false, facing: false, indoors: "yes" });
  await invalidClosed;
  console.log(`PASS: ${base} -> ${guestBase}: maps, lobby listing and cleanup, two clients, Unicode chat, movement, bounds, fishing and shop visibility, invalid activity, identity, room isolation, disconnect, missing rooms, oversize messages`);
} finally {
  for (const ws of sockets) if (ws.readyState < 2) ws.close();
}
