export default {
  async fetch(req) {
    const url = new URL(req.url);

    if (url.pathname === "/events") {
      const stream = new ReadableStream({
        async start(controller) {
          const encoder = new TextEncoder();

          controller.enqueue(
            encoder.encode(
              `data: ${JSON.stringify({ type: "connected", timestamp: new Date().toISOString() })}\n\n`,
            ),
          );

          let count = 0;
          const interval = setInterval(() => {
            if (count >= 10) {
              controller.enqueue(
                encoder.encode(
                  `data: ${JSON.stringify({ type: "complete", count })}\n\n`,
                ),
              );
              controller.close();
              clearInterval(interval);
              return;
            }

            count++;
            const event = {
              type: "update",
              count,
              timestamp: new Date().toISOString(),
              message: `Event ${count} of 10`,
            };

            console.log("[sse] sending:", event);
            controller.enqueue(
              encoder.encode(`data: ${JSON.stringify(event)}\n\n`),
            );
          }, 1000);

          // Cleanup on abort
          req.signal?.addEventListener("abort", () => {
            console.log("[sse] client disconnected");
            clearInterval(interval);
            controller.close();
          });
        },
      });

      return new Response(stream, {
        headers: {
          "Content-Type": "text/event-stream",
          "Cache-Control": "no-cache",
          Connection: "keep-alive",
        },
      });
    }

    // Regular HTTP endpoint with instructions
    return new Response(
      "SSE server running.\n\nConnect to /events to receive server-sent events.\nExample: curl -N http://localhost:PORT/events\n",
      {
        headers: { "Content-Type": "text/plain" },
      },
    );
  },
};
