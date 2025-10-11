async function resolveHandler() {
  const mod = await import("func:user-code");

  if (typeof mod.handler === "function") {
    return mod.handler;
  }

  if (
    typeof mod.default === "object" &&
    typeof mod.default.handler === "function"
  ) {
    return mod.default.handler;
  }

  if (typeof mod.default === "function") {
    return mod.default;
  }

  throw new Error("Handler Method Not Defined");
}

async function work() {
  let response;
  try {
    const handler = await resolveHandler();
    response = await handler(Func.request);
  } catch (error) {
    const msg =
      error.stack || `Internal Server Error: ${error?.message || error}`;
    response = new Response(msg, {
      status: 500,
    });
  } finally {
    Func.setResponse(response);
  }
}

work().catch(console.error);
