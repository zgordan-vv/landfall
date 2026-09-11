import test from "node:test";
import assert from "node:assert/strict";
import { HealthCounters, LandfallSdk } from "../dist/index.js";

test("health counters aggregate drops and transport failures", () => {
  const changes = [];
  const counters = new HealthCounters((health) => changes.push(health));
  counters.recordDropped(2);
  counters.recordTransportFailure();
  assert.deepEqual(counters.snapshot, { droppedEvents: 2, transportFailures: 1 });
  assert.deepEqual(changes, [
    { droppedEvents: 2, transportFailures: 0 },
    { droppedEvents: 2, transportFailures: 1 },
  ]);
});

test("health callback failures are isolated and sdk exposes a snapshot", () => {
  const sdk = new LandfallSdk({ collectorUrl: "http://localhost:8080", onHealthChange: () => { throw new Error("ignored"); } });
  sdk.recordDroppedEvents(3);
  sdk.recordTransportFailure(2);
  assert.deepEqual(sdk.health, { droppedEvents: 3, transportFailures: 2 });
});
