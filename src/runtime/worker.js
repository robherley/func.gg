const log = (msg) => {
    Deno.core.print(`\n[worker]: ${msg}\n\n`);
}

async function handler(req) {
    log(`Request Method: ${req.method}`);
    log(`Request URL: ${req.url}`);
    return {
        status: 200,
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            msg: "Hello from the worker!",
        })
    }
}

async function worker() {
    try {
        const req = Func.request.get();
        const res = await handler(req);

        if (!res || typeof res !== 'object') {
            throw new Error('invalid response');
        }

        return res
    } catch (error) {
        const msg = error && error.message ? error.message : String(error);
        log(`Error: ${msg}`);
        return {
            status: 500,
            headers: {},
            body: "Internal Server Error"
        };
    }
}

const res = await worker();
log(`Response Status: ${res.status}`);
log(`Response Headers: ${JSON.stringify(res.headers)}`);
log(`Response Body: ${res.body}`);
Func.response.set(res);
