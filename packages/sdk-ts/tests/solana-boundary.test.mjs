import test from "node:test";
import assert from "node:assert/strict";
import { captureLatestBlockhash, normalizeBlockhashSnapshot, preserveCustomerOperation } from "../dist/index.js";

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
