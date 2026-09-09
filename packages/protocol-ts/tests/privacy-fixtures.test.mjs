import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repositoryRoot = fileURLToPath(new URL("../../../", import.meta.url));

test("privacy fixtures separate structural rejection from text redaction", () => {
  const result = spawnSync(process.execPath, ["scripts/check-protocol-privacy-fixtures.mjs"], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /8 rejected and 4 redacted privacy fixtures/);
  assert.doesNotMatch(result.stdout, /LF_CANARY_/);
  assert.doesNotMatch(result.stderr, /LF_CANARY_/);
});
