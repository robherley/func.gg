export default {
  async fetch(req) {
    return Response.json({
      method: req.method,
      headers: req.headers,
      url: req.url,
      body: await req.text(),
    });
  },
};
