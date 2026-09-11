import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

test("SDK package exposes ESM entrypoint and declarations", async () => {
  const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
  assert.equal(packageJson.type, "module");
  assert.equal(packageJson.module, "./dist/index.js");
  assert.equal(packageJson.types, "./dist/index.d.ts");
  assert.equal(packageJson.exports["."].import, "./dist/index.js");
  await readFile(new URL("../dist/index.js", import.meta.url));
  await readFile(new URL("../dist/index.d.ts", import.meta.url));
});
