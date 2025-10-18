import "ext:func_ext/deno_webidl.js";
import "ext:func_ext/deno_console.js";
import "ext:func_ext/deno_url.js";
import "ext:func_ext/deno_web.js";
import "ext:func_ext/deno_net.js";
import "ext:func_ext/deno_fetch.js";

const {
  op_get_request,
  op_set_response,
  op_get_request_id,
  op_read_request_chunk,
  op_write_response_chunk,
} = Deno.core.ops;

async function setResponse(res) {
  if (!(res instanceof Response)) {
    throw new Error("Response must be of class Response");
  }

  op_set_response({
    status: res.status,
    headers: res.headers,
    body: "",
  });

  res.body.pipeTo(newResponseStream());
}

function newResponseStream() {
  let closed = false;
  return new WritableStream({
    async write(chunk) {
      if (closed) throw new Error("Stream is closed");
      op_write_response_chunk(chunk);
    },
    async close() {
      if (closed) return;
      closed = true;
    },
  });
}

function getRequest() {
  let body;

  let { uri, method, headers } = op_get_request();
  if (method !== "GET" && method !== "HEAD") {
    body = new ReadableStream({
      async pull(controller) {
        const chunk = await op_read_request_chunk();
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
  setResponse: {
    value: setResponse,
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
