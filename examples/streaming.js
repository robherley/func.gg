export async function fetch() {
  const stream = new ReadableStream({
    async start(controller) {
      for (let i = 1; i <= 10; i++) {
        const message = `[${new Date().toISOString()}] Log message ${i}\n`;
        console.log(message.trim());
        controller.enqueue(new TextEncoder().encode(message));
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
      controller.close();
    },
  });

  return new Response(stream);
};