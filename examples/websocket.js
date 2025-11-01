export default {
  async fetch(req, server) {
    const url = new URL(req.url);

    if (url.pathname === "/ws") {
      const upgraded = server.upgrade(req);
      if (upgraded) {
        return;
      }
      return new Response("upgrade failed", { status: 500 });
    }

    return new Response("WebSocket server running. Connect to /ws", {
      headers: { "Content-Type": "text/plain" },
    });
  },

  websocket: {
    open(ws) {
      console.log("[websocket] client connected");
      ws.send("Welcome! You are now connected to the WebSocket server.");
    },

    message(ws, message) {
      console.log("[websocket] received:", message);
      const response = {
        type: "echo",
        timestamp: new Date().toISOString(),
        data: message,
      };

      ws.send(JSON.stringify(response));
    },

    close(ws, code, reason) {
      console.log(`[websocket] client disconnected: ${code} ${reason}`);
    },

    error(ws, error) {
      console.error("[websocket] error:", error);
    },
  },
};
