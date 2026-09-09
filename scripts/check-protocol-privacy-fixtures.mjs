import { readFileSync, readdirSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const repositoryRoot = fileURLToPath(new URL("../", import.meta.url));
const schemaRoot = join(repositoryRoot, "schemas/events/v1");
const fixtureRoot = join(repositoryRoot, "fixtures/protocol/privacy");
const schemaIdBase = "https://schemas.landfall.dev/events/v1/";

function fail(message) {
  throw new Error(message);
}

function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    fail(`${relative(repositoryRoot, path)} is not valid JSON: ${error.message}`);
  }
}

function listJsonFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return listJsonFiles(path);
    return entry.isFile() && entry.name.endsWith(".json") ? [path] : [];
  });
}

function containsProperty(value, propertyName) {
  if (Array.isArray(value)) return value.some((item) => containsProperty(item, propertyName));
  if (value === null || typeof value !== "object") return false;
  return (
    Object.hasOwn(value, propertyName) ||
    Object.values(value).some((child) => containsProperty(child, propertyName))
  );
}

function setJsonPointer(document, pointer, replacement) {
  const segments = pointer
    .slice(1)
    .split("/")
    .map((segment) => segment.replaceAll("~1", "/").replaceAll("~0", "~"));
  let current = document;
  for (const segment of segments.slice(0, -1)) {
    current = current?.[segment];
    if (current === null || typeof current !== "object") fail(`missing redaction path ${pointer}`);
  }
  const finalSegment = segments.at(-1);
  if (finalSegment === undefined || !Object.hasOwn(current, finalSegment)) {
    fail(`missing redaction path ${pointer}`);
  }
  current[finalSegment] = replacement;
}

const schemaFiles = listJsonFiles(schemaRoot).filter((path) => !path.endsWith("manifest.json"));
const validator = new Ajv2020({ allErrors: true, strict: true });
addFormats(validator);
for (const path of schemaFiles) {
  const schema = readJson(path);
  validator.addSchema(schema, schema.$id);
}

const manifestPath = join(fixtureRoot, "manifest.json");
const manifest = readJson(manifestPath);
if (manifest.schema_version !== "1.0" || manifest.redaction_version !== "1.0") {
  fail("privacy fixtures must declare the implemented schema and redaction versions");
}
if (manifest.replacement !== "[REDACTED]") fail("unexpected canonical replacement marker");

const registeredPaths = [
  ...manifest.reject.map(({ input }) => input),
  ...manifest.redact.flatMap(({ input, expected }) => [input, expected]),
].sort();
const actualPaths = listJsonFiles(fixtureRoot)
  .filter((path) => path !== manifestPath)
  .map((path) => relative(fixtureRoot, path).split(sep).join("/"))
  .sort();
if (JSON.stringify(registeredPaths) !== JSON.stringify(actualPaths)) {
  fail("privacy manifest paths must exactly match the checked-in fixture files");
}

const ids = new Set();
for (const fixture of manifest.reject) {
  if (ids.has(fixture.id)) fail(`duplicate privacy fixture id ${fixture.id}`);
  ids.add(fixture.id);
  const input = readJson(join(fixtureRoot, fixture.input));
  const validate = validator.getSchema(`${schemaIdBase}${fixture.schema}`);
  if (!validate) fail(`${fixture.id} references unknown schema ${fixture.schema}`);
  if (validate(input)) fail(`${fixture.id} must be structurally rejected`);
  if (!validate.errors?.some(({ keyword }) => keyword === fixture.expected_keyword)) {
    fail(`${fixture.id} did not fail with schema keyword ${fixture.expected_keyword}`);
  }
  if (fixture.prohibited_key && !containsProperty(input, fixture.prohibited_key)) {
    fail(`${fixture.id} does not contain prohibited key ${fixture.prohibited_key}`);
  }
  if (fixture.minimum_metadata_bytes !== undefined) {
    const byteLength = Buffer.byteLength(JSON.stringify(input.attributes?.metadata), "utf8");
    if (byteLength < fixture.minimum_metadata_bytes) {
      fail(`${fixture.id} metadata is only ${byteLength} bytes`);
    }
  }
}

for (const fixture of manifest.redact) {
  if (ids.has(fixture.id)) fail(`duplicate privacy fixture id ${fixture.id}`);
  ids.add(fixture.id);
  const input = readJson(join(fixtureRoot, fixture.input));
  const expected = readJson(join(fixtureRoot, fixture.expected));
  const validate = validator.getSchema(`${schemaIdBase}${fixture.schema}`);
  if (!validate) fail(`${fixture.id} references unknown schema ${fixture.schema}`);
  if (!validate(input)) fail(`${fixture.id} input must reach privacy validation`);
  if (!validate(expected)) fail(`${fixture.id} expected output must remain schema-valid`);

  const transformed = structuredClone(input);
  for (const pointer of fixture.pointers) {
    setJsonPointer(transformed, pointer, manifest.replacement);
  }
  if (JSON.stringify(transformed) !== JSON.stringify(expected)) {
    fail(`${fixture.id} changes fields outside its declared redaction pointers`);
  }

  const inputText = JSON.stringify(input);
  const expectedText = JSON.stringify(expected);
  for (const canary of fixture.canaries) {
    if (!inputText.includes(canary)) fail(`${fixture.id} input omits canary ${canary}`);
    if (expectedText.includes(canary)) fail(`${fixture.id} output retains a prohibited canary`);
  }
  if (!fixture.pointers.every((pointer) => pointer.startsWith("/attributes/"))) {
    fail(`${fixture.id} attempts redaction outside allowlisted event attributes`);
  }
}

console.log(
  `Validated ${manifest.reject.length} rejected and ${manifest.redact.length} redacted privacy fixtures without exposing canary values.`,
);
