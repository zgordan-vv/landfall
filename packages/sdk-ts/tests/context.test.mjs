import test from "node:test";
import assert from "node:assert/strict";
import { createBusinessActionContext, LandfallSdk } from "../dist/index.js";

test("business action context is immutable and links traces", () => {
  const action = createBusinessActionContext("checkout-42", "Checkout");
  const sdk = new LandfallSdk({ collectorUrl: "http://localhost:8080" });
  const trace = sdk.startTrace("0198ef00-0000-7000-8000-000000000300", action);
  assert.equal(trace.businessAction?.businessActionId, "checkout-42");
  assert.throws(() => { action.businessActionId = "changed"; }, TypeError);
});
