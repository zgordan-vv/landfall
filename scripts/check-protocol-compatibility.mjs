import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  SUPPORTED_EVENT_TYPES,
  SUPPORTED_SCHEMA_VERSIONS,
  checkEventCompatibility,
} from "../packages/protocol-ts/src/compatibility.ts";

const repositoryRoot = fileURLToPath(new URL("../", import.meta.url));

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function fail(message) {
  throw new Error(message);
}

const manifest = readJson(join(repositoryRoot, "schemas/events/v1/manifest.json"));
const fixtures = readJson(join(repositoryRoot, "fixtures/protocol/compatibility.json"));
const manifestVersions = manifest.supported_versions.map(({ version }) => version);

if (JSON.stringify(SUPPORTED_SCHEMA_VERSIONS) !== JSON.stringify(manifestVersions)) {
  fail("TypeScript supported schema versions differ from the protocol manifest");
}

const versionOne = manifest.supported_versions.find(({ version }) => version === "1.0");
const manifestEventTypes = Object.keys(manifest.event_types).sort();
if (JSON.stringify(versionOne?.event_types) !== JSON.stringify(manifestEventTypes)) {
  fail("the 1.0 capability matrix must contain every registered event type exactly once");
}
if (JSON.stringify(SUPPORTED_EVENT_TYPES) !== JSON.stringify(manifestEventTypes)) {
  fail("TypeScript supported event types differ from the 1.0 capability matrix");
}

const policy = manifest.compatibility;
if (
  policy.selection !== "exact_registered_version" ||
  policy.same_major_implies_support !== false ||
  policy.unknown_schema_version_code !== "LF_UNSUPPORTED_SCHEMA_VERSION" ||
  policy.unknown_event_type_code !== "LF_UNSUPPORTED_EVENT_TYPE" ||
  policy.raw_event_version_mutable !== false ||
  policy.implicit_upcasting !== false ||
  JSON.stringify(policy.rollout_order) !== JSON.stringify(["collector", "cli", "sdk"])
) {
  fail("protocol manifest weakens the accepted exact-version rollout policy");
}

const fixtureIds = new Set();
for (const fixture of fixtures.cases) {
  if (fixtureIds.has(fixture.id)) fail(`duplicate compatibility fixture id ${fixture.id}`);
  fixtureIds.add(fixture.id);
  const actual = checkEventCompatibility(fixture.schema_version, fixture.event_type);
  if (JSON.stringify(actual) !== JSON.stringify(fixture.expected)) {
    fail(
      `${fixture.id} produced ${JSON.stringify(actual)}, expected ${JSON.stringify(fixture.expected)}`,
    );
  }
}

for (const eventType of manifestEventTypes) {
  const actual = checkEventCompatibility("1.0", eventType);
  if (!actual.supported) fail(`TypeScript rejects registered event type ${eventType}`);
}

console.log(
  `Validated ${fixtures.cases.length} compatibility decisions for ${manifestVersions.length} exact schema version and ${manifestEventTypes.length} event types.`,
);
