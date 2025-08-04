function handler(request) {
    return {
        status: 200,
        headers: { "content-type": "text/html" },
        body: `<h1>Async JavaScript Handler Response</h1>
                <p>Request method: ${request.method}</p>
                <p>Request path: ${request.url}</p>
                <p>Current time: ${new Date().toISOString()}</p>`
    };
}