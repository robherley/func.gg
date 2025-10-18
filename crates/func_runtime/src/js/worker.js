async function resolveHandler() {
  const mod = await import("func:user-code");

  const methods = ["fetch", "handler"];

  for (const method of methods) {
    if (typeof mod[method] === "function") {
      return mod[method];
    }

    if (
      typeof mod.default === "object" &&
      typeof mod.default[method] === "function"
    ) {
      return mod.default[method];
    }
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
