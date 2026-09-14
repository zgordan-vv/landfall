import test from "node:test";
import assert from "node:assert/strict";
import { fingerprintSignedBytes } from "../dist/index.js";

test("fingerprints exact signed bytes and returns protocol shape", () => {
  const result = fingerprintSignedBytes(
    "0198ef00-0000-7000-8000-000000000900",
    new Uint8Array([1, 2]),
    (bytes) => new Uint8Array(32).fill(bytes[0]),
  );
  assert.equal(result.algorithm, "lf-hmac-sha256-v1");
  assert.equal(result.value_hex.length, 64);
  assert.throws(() =>
    fingerprintSignedBytes(
      "0198ef00-0000-7000-8000-000000000900",
      new Uint8Array(),
      () => new Uint8Array(32),
    ),
  );
});
