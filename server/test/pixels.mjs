import assert from "node:assert/strict";

const base = (process.env.SERVER_URL || "ws://127.0.0.1:8787").replace(/\/$/, "").replace(/^http/, "ws");
const guest = (process.env.GUEST_SERVER_URL || base).replace(/\/$/, "").replace(/^http/, "ws");
const code = crypto.randomUUID().replaceAll("-", "").slice(0, 8).toUpperCase();
const sockets = [];
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
async function join(name, create = false, address = base) {
  const ws = new WebSocket(`${address}/room/${code}?protocol=5&name=${name}&create=${Number(create)}&room_name=Pixel%20QA`);
  sockets.push(ws);
  const messages = [];
  ws.addEventListener("message", ({data}) => { if (data !== "pong") messages.push(JSON.parse(data)); });
  const next = async (type, predicate = () => true) => {
    const until = Date.now() + 10000;
    while (Date.now() < until) {
      const index = messages.findIndex((m) => m.type === type && predicate(m));
      if (index >= 0) return messages.splice(index, 1)[0];
      if (ws.readyState === WebSocket.CLOSED) throw new Error(`Connection closed waiting for ${type}`);
      await delay(10);
    }
    throw new Error(`Timed out waiting for ${type}`);
  };
  const { world } = await next("world");
  assert.equal(world.format, 3);
  assert.ok(Object.keys(world.content.pixels).length > 0);
  const send = (message) => ws.send(JSON.stringify(message));
  send({ type: "world_ready", revision: world.revision });
  return { next, send, welcome: await next("welcome"), world };
}
function chunk(update, x, y) {
  if (update.map !== "yapshire:town") return;
  return update.chunks.find((c) => c.index === Math.floor(y / 32) * 45 + Math.floor(x / 32));
}
function pixel(update, x, y) {
  let offset = y % 32 * 32 + x % 32;
  for (const [length, cell] of chunk(update, x, y)?.runs || []) {
    if (offset < length) return cell;
    offset -= length;
  }
}
try {
  const a = await join("Alice", true);
  const b = await join("Bob", false, guest);
  a.send({ type: "paint", map: "yapshire:town", x: 270, y: 214, tool: "dig" });
  const predicate = (m) => pixel(m.update, 270, 214) === 0;
  assert.deepEqual((await a.next("pixels", predicate)).update, (await b.next("pixels", predicate)).update);
  const late = await join("Late", false, guest);
  assert.equal(pixel(late.welcome.pixels.find((u) => u.map === "yapshire:town"), 270, 214), 0);
  for (const [tool, material] of [["water", 3], ["sand", 2]]) {
    await delay(150);
    a.send({ type: "paint", map: "yapshire:town", x: 270, y: 176, tool });
    const has = (m) => chunk(m.update, 270, 176)?.runs.some((r) => (r[1] & 3) === material);
    assert.deepEqual((await a.next("pixels", has)).update, (await b.next("pixels", has)).update);
  }
  console.log(`PIXELS PASS: ${base} -> ${guest}: digging, sand, water, matching authoritative chunks and late-join restoration`);
} finally {
  for (const ws of sockets) if (ws.readyState < WebSocket.CLOSING) ws.close();
}
