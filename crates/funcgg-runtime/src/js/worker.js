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
  try {
    const handler = await resolveHandler();
    const response = await handler(Func.request);
    return Func.setResponse(response);
  } catch (error) {
    const msg = error && error.message ? error.message : String(error);
    return new Response(`Internal Server Error: ${msg}`, {
      status: 500,
    });
  }
}

work().catch(console.error);
