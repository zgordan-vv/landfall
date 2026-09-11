import test from "node:test";
import assert from "node:assert/strict";
import { captureLatestBlockhash, measureSigning, normalizeBlockhashSnapshot, normalizeSimulationResult, preserveCustomerOperation } from "../dist/index.js";

test("signing measurement records monotonic delay without signer access", async () => {
  const ticks = [1000n, 1234n];
  const success = await measureSigning(async () => "signed-value", () => ticks.shift());
  assert.deepEqual(success, { result: "completed", durationNs: "234", value: "signed-value" });
  const error = new Error("rejected");
  const failedTicks = [10n, 25n];
  const failure = await measureSigning(async () => { throw error; }, () => failedTicks.shift());
  assert.equal(failure.result, "failed");
  assert.equal(failure.durationNs, "15");
  assert.equal(failure.error, error);
});

test("simulation capture preserves compute units and classifies execution errors", () => {
  assert.deepEqual(normalizeSimulationResult({ err: null, unitsConsumed: 9007199254740993n, logs: ["ok"] }), { rpcResult: "succeeded", unitsConsumed: "9007199254740993", logsPresent: true });
  assert.equal(normalizeSimulationResult({ err: { InstructionError: [0, "Custom"] } }).rpcResult, "execution_error");
  assert.equal(normalizeSimulationResult({ blockhashNotFound: true }).rpcResult, "blockhash_not_found");
  assert.equal(normalizeSimulationResult("bad").rpcResult, "malformed_response");
});

test("blockhash capture normalizes an exact validity height", async () => {
  const client = { getLatestBlockhash: async () => ({ blockhash: "  hash-1 ", lastValidBlockHeight: 123n }), simulate: async () => ({}), sign: async () => ({}), submit: async () => "sig", confirm: async () => ({}) };
  assert.deepEqual(await captureLatestBlockhash(client), { blockhash: "hash-1", lastValidBlockHeight: "123" });
  assert.throws(() => normalizeBlockhashSnapshot("hash", "01"), /canonical unsigned decimal/);
});

test("adapter boundary preserves customer result and error identity", async () => {
  const value = { signature: "sig-1" };
  assert.equal(await preserveCustomerOperation(async () => value), value);
  const error = new Error("customer failure");
  await assert.rejects(preserveCustomerOperation(async () => { throw error; }), (received) => received === error);
});
