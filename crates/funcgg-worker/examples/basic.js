const readChunk = async () => Deno.core.ops.op_read_request_chunk();

export async function handler(req) {
  const who = req.body ? JSON.parse(req.body).name : "World";

  let idx = 0;
  while (true) {
    const chunk = await readChunk();
    console.log(idx, chunk.length);

    idx++;
    if (chunk.length === 0) {
      break;
    }
  }

  return {
    status: 200,
    headers: {
      "Content-Type": "application/json",
      "X-Foo": "bar",
    },
    body: JSON.stringify({
      msg: `Hello ${who} from the worker!`,
    }),
  };
}
