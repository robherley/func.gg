const { FUNCD_SOCKET_PATH = "/tmp/funcd.sock" } = process.env;

const func = await import("../examples/streaming.js");

if (!func || typeof func !== "object") {
  throw new Error("Func must export an object");
}

if (!func.fetch && typeof !func.routes) {
  throw new Error("Func must export a fetch function or routes object");
}

// TODO(robherley): websockets
Bun.serve({
  unix: FUNCD_SOCKET_PATH,
  routes: func.routes,
  fetch: func.fetch,
});