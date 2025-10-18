// https://github.com/denoland/deno/tree/main/ext/webidl

import * as webidl from "ext:deno_webidl/00_webidl.js";

Object.defineProperty(globalThis, webidl.brand, {
  value: webidl.brand,
  enumerable: false,
  configurable: true,
  writable: true,
});
