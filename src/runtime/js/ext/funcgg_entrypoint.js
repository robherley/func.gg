import "ext:funcgg_runtime/funcgg_tmp.js";
import "ext:funcgg_runtime/init.deno_webidl.js";
import "ext:funcgg_runtime/init.deno_console.js";
import "ext:funcgg_runtime/init.deno_url.js";
import "ext:funcgg_runtime/init.deno_web.js";
import "ext:funcgg_runtime/init.deno_net.js";
import "ext:funcgg_runtime/init.deno_fetch.js";

const { op_get_request, op_set_response, op_get_request_id } = Deno.core.ops;

Object.defineProperty(globalThis, "Func", {
  value: {},
  writable: false,
  enumerable: true,
  configurable: false,
});

Object.defineProperties(globalThis.Func, {
  request: {
    get: op_get_request,
    set: () => {},
    enumerable: true,
    configurable: false,
  },
  response: {
    get: () => {},
    set: op_set_response,
    enumerable: true,
    configurable: false,
  },
  request_id: {
    get: op_get_request_id,
    set: () => {},
    enumerable: true,
    configurable: false,
  },
});
