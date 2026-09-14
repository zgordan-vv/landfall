import test from "node:test";
import assert from "node:assert/strict";
import { buildTraceCreated } from "../dist/index.js";

test("manual builder emits a typed envelope without raw transaction data", () => {
  const event = buildTraceCreated(
    {
      eventId: "0198ef00-0000-7000-8000-000000000020",
      occurredAt: "2026-08-29T12:00:00Z",
      projectId: "0198ef00-0000-7000-8000-000000000100",
      environmentId: "0198ef00-0000-7000-8000-000000000200",
      traceId: "0198ef00-0000-7000-8000-000000000300",
      source: { kind: "application", name: "demo", version: "1.0.0" },
    },
    { flow: "checkout", transaction_version: "legacy" },
  );
  assert.equal(event.event_type, "solana.trace.created");
  assert.equal("signed_transaction_bytes" in event, false);
});
