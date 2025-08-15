
// async function fetch(request) {
//     return {
//         status: 200,
//         headers: { "content-type": "text/html" },
//         body: `<h1>Async JavaScript Handler Response</h1>
//             <p>Request method: ${request.method}</p>
//             <p>Request path: ${request.url}</p>
//             <p>Current time: ${new Date().toISOString()}</p>`
//     }
// }

// import { handler } from "func:http";

// // sleep function
function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function main() {
    await sleep(1000); // Simulate some async work
    // await handler({ body: "hello world!"})
}

main().catch((error) => {
    console.error("Error in main:", error);
    process.exit(1);
});