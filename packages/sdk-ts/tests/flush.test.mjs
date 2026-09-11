import test from "node:test";
import assert from "node:assert/strict";
import { LandfallSdk, boundedFlush } from "../dist/index.js";

test("bounded flush completes before its deadline", async () => {
  let called = false;
  const result = await boundedFlush(async () => { called = true; }, { timeoutMs: 100 });
  assert.equal(called, true);
  assert.deepEqual(result, { status: "flushed" });
});

test("bounded flush returns timeout without waiting forever", async () => {
  const result = await boundedFlush(() => new Promise(() => {}), { timeoutMs: 5 });
  assert.deepEqual(result, { status: "timed_out" });
});

test("sdk flush reports failures and updates transport health", async () => {
  const errors = [];
  const sdk = new LandfallSdk({ collectorUrl: "http://localhost:8080", onTelemetryError: (error) => errors.push(error) });
  const result = await sdk.flush(async () => { throw new Error("collector unavailable"); });
  assert.equal(result.status, "failed");
  assert.equal(sdk.health.transportFailures, 1);
  assert.equal(errors.length, 1);
  assert.equal(errors[0].code, "transport");
});
