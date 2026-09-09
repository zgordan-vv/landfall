import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repositoryRoot = fileURLToPath(new URL("../../../", import.meta.url));

test("shared protocol fixtures match JSON Schema and semantic rules", () => {
  const result = spawnSync(process.execPath, ["scripts/check-protocol-fixtures.mjs"], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /15 event types/);
});
