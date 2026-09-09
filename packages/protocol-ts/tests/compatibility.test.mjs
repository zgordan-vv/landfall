import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repositoryRoot = fileURLToPath(new URL("../../../", import.meta.url));

test("TypeScript compatibility decisions match the canonical matrix", () => {
  const result = spawnSync(process.execPath, ["scripts/check-protocol-compatibility.mjs"], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /7 compatibility decisions/);
});
