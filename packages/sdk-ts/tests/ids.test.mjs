import test from "node:test";
import assert from "node:assert/strict";
import { generateUuidV7, isCanonicalUuidV7 } from "../dist/index.js";

test("generates canonical UUIDv7", () => {
  const id = generateUuidV7(1724932800123);
  assert.equal(isCanonicalUuidV7(id), true);
  assert.equal(id[14], "7");
});
