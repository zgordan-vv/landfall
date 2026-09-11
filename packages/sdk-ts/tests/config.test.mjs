import test from "node:test";
import assert from "node:assert/strict";
import { validateConfig } from "../dist/index.js";

test("configuration is validated and frozen", () => {
  const config = validateConfig({ collectorUrl: "https://collector.example", maxBatchEvents: 10, maxBufferEvents: 20 });
  assert.equal(config.maxBatchEvents, 10);
  assert.throws(() => { config.maxBatchEvents = 99; }, TypeError);
  assert.throws(() => validateConfig({ collectorUrl: "http://collector.example" }));
});
