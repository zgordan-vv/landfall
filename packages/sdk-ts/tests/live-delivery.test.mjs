import test from "node:test";
import assert from "node:assert/strict";
import { LandfallSdk } from "../dist/index.js";

const event = {
  schema_version: "1.0",
  event_id: "0198ef00-0000-7000-8000-00000000c101",
  event_type: "solana.trace.created",
  occurred_at: "2026-09-13T20:20:00.001Z",
  project_id: "0198ef00-0000-7000-8000-000000000100",
  environment_id: "0198ef00-0000-7000-8000-000000000200",
  trace_id: "0198ef00-0000-7000-8000-00000000c100",
  source: { kind: "sdk", name: "landfall-js", version: "0.1.0" },
  privacy_mode: "standard",
  privacy_policy_version: "1.0",
  redaction_version: "1.0",
  attributes: { flow: "checkout", transaction_version: "legacy" },
};

test("SDK posts buffered events to the collector and clears a successful batch", async () => {
  const requests = [];
  const sdk = new LandfallSdk({
    collectorUrl: "http://localhost:8080/",
    fetch: async (url, init) => {
      requests.push({ url, init });
      return { status: 202 };
    },
  });
  assert.equal(sdk.startTrace(event.trace_id).emit(event), true);
  assert.equal(sdk.bufferedEventCount, 1);
  assert.deepEqual(await sdk.flush(), { status: "flushed" });
  assert.equal(sdk.bufferedEventCount, 0);
  assert.equal(requests[0].url, "http://localhost:8080/v1/ingest");
  const body = JSON.parse(requests[0].init.body);
  assert.match(body.batch_id, /^[0-9a-f-]{36}$/);
  assert.deepEqual(body.events, [event]);
});

test("failed delivery restores the exact batch for a later retry", async () => {
  let calls = 0;
  const sdk = new LandfallSdk({
    collectorUrl: "http://localhost:8080",
    fetch: async (_url, init) => {
      calls += 1;
      return { status: calls === 1 ? 400 : 202, body: init.body };
    },
    retry: { maxAttempts: 1 },
  });
  sdk.capture(event);
  assert.equal((await sdk.flush()).status, "failed");
  assert.equal(sdk.bufferedEventCount, 1);
  assert.deepEqual(await sdk.flush(), { status: "flushed" });
  assert.equal(sdk.bufferedEventCount, 0);
});
