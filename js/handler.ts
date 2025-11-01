const func = await import("../examples/basic.js");

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
