export async function handler(req) {
  console.log("[req]", req);
  try {
    const body = await req.json();
    console.log("[body]", body);
  } catch {}

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
