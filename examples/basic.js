export default {
    async fetch(req) {
        console.log("[req]", req);
        try {
            const body = await req.json();
            console.log("[body]", body);
        } catch { }

        const res = new Response("hello world", {
            headers: {
                "X-Foo": "bar",
            },
        });

        console.log("[res]", res);
        return res;
    }
}