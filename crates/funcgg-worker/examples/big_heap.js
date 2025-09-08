function startEatingHeapMemory() {
  const arrays = [];
  const chunkSize = 1024 * 1024;
  let totalAllocated = 0;

  console.log("Starting heap limit test - allocating memory in chunks...");

  while (true) {
    const chunk = new Array(chunkSize / 8);
    for (let i = 0; i < chunk.length; i++) {
      chunk[i] = Math.random();
    }
    arrays.push(chunk);
    totalAllocated += chunkSize;

    console.log(`Allocated: ${(totalAllocated / (1024 * 1024)).toFixed(2)} MB`);

    if (totalAllocated > 100 * 1024 * 1024) {
      arrays.push(JSON.stringify(arrays));
    }
  }
}

function takeABigByte() {
  console.log("Attempting to allocate a single large array...");
  const hugeArray = new Array((1024 * 1024 * 1024) / 8);
  for (let i = 0; i < hugeArray.length; i++) {
    hugeArray[i] = i;
  }
  console.log("Successfully allocated");
}

export async function handler(req) {
  switch (req.uri) {
    case "/gradual":
      startEatingHeapMemory();
      break;
    case "/at-once":
      takeABigByte();
      break;
    default:
      return {
        status: 404,
        body: "Either /gradual or /at-once",
      };
  }

  return {
    status: 200,
  };
}
