function __FUNCGG_main() {
    // Get the request data from the runtime
    const request = getRequest();
    
    // Check if handler function exists
    if (typeof handler !== 'function') {
        throw new Error('Handler function not found');
    }
    
    if (!request) {
        throw new Error('No request data available');
    }
    
    try {
        // Call the handler and await the result if it's a promise
        const response = handler(request);
        
        // Ensure response has required fields
        if (!response || typeof response !== 'object') {
            throw new Error('Handler must return an object');
        }
        
        // Return the response object directly - no JSON serialization needed
        return {
            status: response.status || 200,
            headers: response.headers || {},
            body: response.body || null
        };
    } catch (error) {
        return {
            status: 500,
            headers: {},
            body: "Error: " + error.message
        };
    }
}

__FUNCGG_main();