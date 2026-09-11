import test from "node:test";
import assert from "node:assert/strict";
import { EventBuffer } from "../dist/index.js";

test("bounded buffer rejects overflow and drains FIFO", () => {
  const buffer = new EventBuffer(2);
  assert.equal(buffer.push("a"), true);
  assert.equal(buffer.push("b"), true);
  assert.equal(buffer.push("c"), false);
  assert.deepEqual(buffer.drain(1), ["a"]);
  assert.deepEqual(buffer.drain(), ["b"]);
});
