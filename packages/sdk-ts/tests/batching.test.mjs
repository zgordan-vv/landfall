import test from "node:test";
import assert from "node:assert/strict";
import { BatchAssembler, EventBuffer } from "../dist/index.js";

test("batch retries preserve stable id and event order", () => {
  const buffer = new EventBuffer(4);
  buffer.push("a");
  buffer.push("b");
  const assembler = new BatchAssembler(2);
  const batch = assembler.assemble(buffer);
  const retry = assembler.retry(batch);
  assert.equal(retry.batchId, batch.batchId);
  assert.deepEqual(retry.events, ["a", "b"]);
});
