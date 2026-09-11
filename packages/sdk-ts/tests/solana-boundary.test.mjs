import test from "node:test";
import assert from "node:assert/strict";
import { preserveCustomerOperation } from "../dist/index.js";

test("adapter boundary preserves customer result and error identity", async () => {
  const value = { signature: "sig-1" };
  assert.equal(await preserveCustomerOperation(async () => value), value);
  const error = new Error("customer failure");
  await assert.rejects(preserveCustomerOperation(async () => { throw error; }), (received) => received === error);
});
