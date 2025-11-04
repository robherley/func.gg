const { FUNCD_SOCKET, FUNCD_SCRIPT } = process.env;

if (!FUNCD_SOCKET) {
  throw new Error("FUNCD_SOCKET environment variable is not set");
}

if (!FUNCD_SCRIPT) {
  throw new Error("FUNC_SCRIPT environment variable is not set");
}

const socket = await Bun.connect({
  unix: FUNCD_SOCKET,
  socket: {
    data(socket, data) {
      console.log("[socket] data", data);
    },
    open(socket) {
      console.log("[socket] opened");
      socket.write(JSON.stringify({ kind: "started" }) + "\n");
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

process.on("uncaughtException", (err) => {
  socket.write(
    JSON.stringify({
      kind: "error",
      payload: { error: `uncaughtException: ${err}` },
    }) + "\n",
  );
});

process.on("unhandledRejection", (reason, promise) => {
  socket.write(
    JSON.stringify({
      kind: "error",
      payload: { error: `unhandledRejection: ${promise}: ${reason}` },
    }) + "\n",
  );
});

const func = await import(FUNCD_SCRIPT);

if (!func || !func.default) {
  throw new Error("Func must have a default export");
}

if (!func.default.fetch) {
  throw new Error("Func must export a fetch function");
}

const server = Bun.serve({
  fetch: func.default.fetch,
  websocket: func.default.websocket,
});

console.log(`Server started on ${server.hostname}:${server.port}`);
socket.write(
  JSON.stringify({ kind: "ready", payload: { port: server.port } }) + "\n",
);
