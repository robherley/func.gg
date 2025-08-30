export async function handler(req) {
  console.log("Request:", req);
  const who = req.body ? JSON.parse(req.body).name : "World";

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
