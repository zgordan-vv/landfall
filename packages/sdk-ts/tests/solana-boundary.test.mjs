import test from "node:test";
import assert from "node:assert/strict";
import { captureLatestBlockhash, fingerprintSignedBytesInMemory, measureConfirmationWait, measureSigning, normalizeBlockhashSnapshot, normalizeSimulationResult, preserveCustomerOperation, submitWithRoute } from "../dist/index.js";

test("confirmation wait capture records duration and timeout classification", async () => {
  const successTicks = [50n, 80n];
  const success = await measureConfirmationWait(async () => "confirmed", () => successTicks.shift());
  assert.deepEqual(success, { result: "commitment_reached", durationNs: "30", value: "confirmed" });
  const timeoutTicks = [100n, 145n];
  const timeout = await measureConfirmationWait(async () => { throw new Error("deadline"); }, () => timeoutTicks.shift(), () => "timeout");
  assert.equal(timeout.result, "timeout");
  assert.equal(timeout.durationNs, "45");
});

test("submission wrapper preserves route configuration and original outcome", async () => {
  const config = { attemptId: "0198ef00-0000-7000-8000-000000000601", route: { routeId: "rpc-primary" }, attemptSequence: 1, encoding: "base64", skipPreflight: false };
  const accepted = await submitWithRoute(config, async () => "signature-1");
  assert.equal(accepted.result, "accepted");
  assert.equal(accepted.value, "signature-1");
  assert.equal(accepted.config.route.routeId, "rpc-primary");
  const error = new Error("timeout");
  const failed = await submitWithRoute(config, async () => { throw error; });
  assert.equal(failed.result, "failed");
  assert.equal(failed.error, error);
});

test("signed-byte fingerprint uses exact bytes and wipes its transient copy", () => {
  const input = new Uint8Array([1, 2, 3]);
  let observed;
  const result = fingerprintSignedBytesInMemory("0198ef00-0000-7000-8000-000000000900", input, (bytes) => { observed = Array.from(bytes); return new Uint8Array(32).fill(bytes[0]); });
  assert.deepEqual(observed, [1, 2, 3]);
  assert.deepEqual(Array.from(input), [1, 2, 3]);
  assert.equal(result.value_hex.slice(0, 2), "01");
});

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
