export async function handler(response) {
    // Get the request data from the runtime
    const request = getRequest();

    // Validate handler existence
    if (typeof handler !== 'function') {
        throw new Error('Handler function not found');
    }

    if (!request) {
        throw new Error('No request data available');
    }

    try {
        // Await the handler so both sync and async handlers work
        // const response = await handler(request);

        // Ensure response has required fields
        // if (!response || typeof response !== 'object') {
        //     throw new Error('Handler must return an object');
        // }

        // Normalize and return the response object
        return {
            status: response.status ?? 200,
            headers: response.headers ?? {},
            body: response.body ?? null,
        };
    } catch (error) {
        return {
            status: 500,
            headers: {},
            body: "Error: " + (error && error.message ? error.message : String(error)),
        };
    }
}
