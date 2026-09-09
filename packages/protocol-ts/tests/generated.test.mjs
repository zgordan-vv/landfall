import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repositoryRoot = fileURLToPath(new URL("../../../", import.meta.url));
const generatedPath = fileURLToPath(new URL("../src/generated/v1.ts", import.meta.url));

test("committed protocol types are deterministic and closed", () => {
  const check = spawnSync(process.execPath, ["scripts/generate-protocol-ts.mjs", "--check"], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });
  assert.equal(check.status, 0, check.stderr);

  const generated = readFileSync(generatedPath, "utf8");
  assert.match(generated, /^\/\/ DO NOT EDIT:/u);
  assert.match(generated, /export type WireEvent =/u);
  assert.doesNotMatch(generated, /\bany\b/u);
  assert.doesNotMatch(generated, /event_type\?:/u);
  assert.doesNotMatch(generated, /attributes\?:/u);
});
