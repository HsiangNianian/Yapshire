import { DurableObject } from "cloudflare:workers";

const IDLE_TIMEOUT_MS = 2 * 60 * 1000;
const PORT = 4761;

// One Durable Object owns this club's Rust server and all of its rooms. The URL never
// determines the object ID, so visitors cannot create extra container instances.
export class Club extends DurableObject {
  starting;

  constructor(ctx, env) {
    super(ctx, env);
    if (ctx.container.running) {
      ctx.blockConcurrencyWhile(() => ctx.container.setInactivityTimeout(IDLE_TIMEOUT_MS));
    }
  }

  async ready() {
    const container = this.ctx.container;
    if (!container.running) container.start({ enableInternet: false });
    await container.setInactivityTimeout(IDLE_TIMEOUT_MS);

    const port = container.getTcpPort(PORT);
    const deadline = Date.now() + 20_000;
    while (Date.now() < deadline) {
      try {
        const response = await port.fetch("http://container/health", {
          signal: AbortSignal.timeout(1000),
        });
        await response.body?.cancel();
        if (response.ok) return;
      } catch {
        // Container startup finishes asynchronously; retry until the port is ready.
      }
      await scheduler.wait(200);
    }
    throw new Error("Club server did not become ready");
  }

  async fetch(request) {
    this.starting ??= this.ready().finally(() => { this.starting = undefined; });
    await this.starting;

    const url = new URL(request.url);
    url.protocol = "http:";
    url.host = "container";
    const forwarded = new Request(url, request);
    forwarded.headers.delete("host");
    // Return the original response so a WebSocket upgrade remains intact.
    return this.ctx.container.getTcpPort(PORT).fetch(forwarded);
  }
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (request.method !== "GET") {
      return new Response("Method not allowed", { status: 405, headers: { Allow: "GET" } });
    }
    if (url.pathname === "/") {
      return new Response(`Yapshire 小镇\nOfficial Yapshire server\n\nProtocol 3 — use a matching Yapshire client\nServer: wss://yap-server.mmstudio.games\nAlternate: wss://yap.meaninglessmeaning.studio\nRoom: NIANNIAN\n`, {
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    }
    if (!["/health", "/rooms", "/lobby"].includes(url.pathname) &&
        !/^\/room\/[A-Z0-9]{8}$/.test(url.pathname)) {
      return new Response("Not found", { status: 404 });
    }
    if (request.headers.has("Origin")) {
      return new Response("Browser origin is not allowed", { status: 403 });
    }
    if (url.search.length > 1024) {
      return new Response("Query too long", { status: 414 });
    }
    if ((url.pathname === "/lobby" || url.pathname.startsWith("/room/")) &&
        request.headers.get("Upgrade")?.toLowerCase() !== "websocket") {
      return new Response("WebSocket required", { status: 426 });
    }
    try {
      return await env.CLUB.getByName("NIANNIAN").fetch(request);
    } catch (error) {
      console.error(JSON.stringify({ event: "club_unavailable", message: error.message }));
      return new Response("Club is starting. Please reconnect shortly.", {
        status: 503, headers: { "Retry-After": "5" },
      });
    }
  },
};
