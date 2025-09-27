import "ext:funcgg_runtime/deno_webidl.js";
import "ext:funcgg_runtime/deno_console.js";
import "ext:funcgg_runtime/deno_url.js";
import "ext:funcgg_runtime/deno_web.js";
import "ext:funcgg_runtime/deno_net.js";
import "ext:funcgg_runtime/deno_fetch.js";

const { op_get_request, op_set_response, op_get_request_id } = Deno.core.ops;

function getRequest() {
  let body;

  let { uri, method, headers } = op_get_request();
  if (method !== "GET" && method !== "POST") {
    body = new ReadableStream({
      async pull(controller) {
        const chunk = await Deno.core.ops.op_read_request_chunk();
        if (chunk === null || chunk.length === 0) {
          controller.close();
        } else {
          controller.enqueue(chunk);
        }
      },
    });
  }

  return new Request(uri, {
    method,
    headers,
    body,
  });
}

Object.defineProperty(globalThis, "Func", {
  value: {},
  writable: false,
  enumerable: true,
  configurable: false,
});

Object.defineProperties(globalThis.Func, {
  request: {
    get: getRequest,
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
