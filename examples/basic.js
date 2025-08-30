export async function handler(req) {
  console.log(`Request Method: ${req.method}`);
  console.log(`Request URL: ${req.url}`);
  console.log(`Request ID: ${Func.request_id}`);
  const res = await fetch("https://httpbin.org/get");
  const text = await res.text();
  console.log(text);
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
