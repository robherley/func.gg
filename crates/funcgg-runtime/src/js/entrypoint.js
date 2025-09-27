async function resolveHandlerMethod() {
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

  throw new Error("func.gg(UserError): Handler Method Not Defined");
}

async function worker() {
  try {
    const handler = await resolveHandlerMethod();
    const res = await handler(Func.request);

    console.log("[response]", res);
    if (!res || typeof res !== "object") {
      throw new Error("invalid response");
    }

    return {
      status: res.status || 200,
      headers: res.headers || {},
      body: res.body || "",
    };
  } catch (error) {
    const msg = error && error.message ? error.message : String(error);
    console.log(`Error: ${msg}`);
    return {
      status: 500,
      headers: {},
      body: `Internal Server Error: ${msg}`,
    };
  }
}

Func.response = await worker();
