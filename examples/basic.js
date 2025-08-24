export async function handler(req) {
  log(`Request Method: ${req.method}`);
  log(`Request URL: ${req.url}`);
  log(`Request ID: ${Func.request_id}`);
  return {
    status: 200,
    headers: {
      "Content-Type": "application/json",
      "X-Foo": "bar",
    },
    body: JSON.stringify({
      msg: `Hello from the worker!`,
    }),
  };
}
