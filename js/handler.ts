const { FUNCD_SOCKET } = process.env;

if (!FUNCD_SOCKET) {
  throw new Error("FUNCD_SOCKET environment variable is not set");
}

const socket = await Bun.connect({
  unix: FUNCD_SOCKET,
  socket: {
    data(socket, data) {
      console.log("[socket] data", data);
    },
    open(socket) {
      console.log("[socket] opened");
      socket.write(JSON.stringify({ kind: "ping" }) + "\n");
    },
    close(socket) {
      console.error("[socket] closed");
      process.exit(1);
    },
    error(socket, error) {
      console.error("[socket] error:", error);
      process.exit(1);
    },
  },
});

const func = await import("../examples/streaming.js");

if (!func || !func.default) {
  throw new Error("Func must have a default export");
}

if (!func.default.fetch) {
  throw new Error("Func must export a fetch function");
}

const server = Bun.serve({
  fetch: func.default.fetch,
});

console.log(`Server started on ${server.hostname}:${server.port}`);
socket.write(
  JSON.stringify({ kind: "ready", payload: { port: server.port } }) + "\n",
);
