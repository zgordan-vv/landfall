import test from "node:test";
import assert from "node:assert/strict";
import { allowMetadata, redactEndpoint } from "../dist/index.js";

test("redacts endpoint credentials and filters metadata", () => {
  assert.equal(
    redactEndpoint("https://user:secret@rpc.example/a?token=hidden"),
    "https://rpc.example/a",
  );
  const metadata = allowMetadata({
    service: "checkout",
    seed_phrase: "never",
    huge: "x".repeat(500),
  });
  assert.deepEqual(metadata, { service: "checkout" });
});
