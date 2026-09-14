import test from "node:test";
import assert from "node:assert/strict";
import { sendWithRetry } from "../dist/index.js";

test("transport retries server failures with stable batch", async () => {
  let calls = 0;
  const delays = [];
  const response = await sendWithRetry(
    { batchId: "b1", events: ["e"] },
    async (batch, compressed) => {
      calls += 1;
      assert.equal(batch.batchId, "b1");
      assert.equal(compressed, true);
      return { status: calls === 1 ? 503 : 202 };
    },
    {
      baseDelayMs: 10,
      jitter: () => 0,
      sleep: async (delay) => {
        delays.push(delay);
      },
    },
  );
  assert.equal(response.status, 202);
  assert.equal(calls, 2);
  assert.deepEqual(delays, [5]);
});
