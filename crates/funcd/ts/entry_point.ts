const { FUNCD_MSG_SOCKET, FUNCD_HTTP_SOCKET, FUNCD_USER_SCRIPT } = process.env;

if (!FUNCD_MSG_SOCKET) {
  throw new Error("FUNCD_MSG_SOCKET is not defined");
}

if (!FUNCD_HTTP_SOCKET) {
  throw new Error("FUNCD_HTTP_SOCKET is not defined");
}

if (!FUNCD_USER_SCRIPT) {
  throw new Error("FUNCD_USER_SCRIPT is not defined");
}

const socket = await Bun.connect({
  unix: FUNCD_MSG_SOCKET,
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

const func = await import(FUNCD_USER_SCRIPT);

if (!func || !func.default) {
  throw new Error("Func must have a default export");
}

if (!func.default.fetch) {
  throw new Error("Func must export a fetch function");
}

Bun.serve({
  unix: FUNCD_HTTP_SOCKET,
  fetch: func.default.fetch,
});

socket.write(JSON.stringify({ kind: "ready" }) + "\n");
