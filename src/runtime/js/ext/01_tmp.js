console.log("testing");

globalThis.log = (msg) => {
  Deno.core.print(`[worker]: ${msg}\n`);
};
