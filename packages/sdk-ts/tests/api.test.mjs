import test from "node:test";
import assert from "node:assert/strict";
import { LandfallSdk } from "../dist/index.js";

test("SDK exposes a non-blocking trace context", () => {
  const sdk = new LandfallSdk({ collectorUrl: "http://localhost:8080" });
  const context = sdk.startTrace("0198ef00-0000-7000-8000-000000000300");
  assert.equal(context.traceId, "0198ef00-0000-7000-8000-000000000300");
  assert.doesNotThrow(() => context.emit({ type: "synthetic" }));
});
