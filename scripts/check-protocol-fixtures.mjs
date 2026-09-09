import { readFileSync, readdirSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const repositoryRoot = fileURLToPath(new URL("../", import.meta.url));
const schemaRoot = join(repositoryRoot, "schemas/events/v1");
const fixtureRoot = join(repositoryRoot, "fixtures/protocol/v1");
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

function walk(value, visitor) {
  visitor(value);
  if (Array.isArray(value)) {
    for (const item of value) walk(item, visitor);
  } else if (value !== null && typeof value === "object") {
    for (const child of Object.values(value)) walk(child, visitor);
  }
}

function schemaErrorCategories(errors) {
  const categories = new Set();
  for (const error of errors ?? []) {
    switch (error.keyword) {
      case "required":
        categories.add("missing_required_field");
        break;
      case "additionalProperties":
        categories.add("unknown_field");
        break;
      case "const":
        categories.add("wrong_event_type");
        break;
      case "enum":
        categories.add("unsupported_enum_value");
        break;
      case "type":
        categories.add("type_mismatch");
        break;
      case "format":
      case "pattern":
      case "maxLength":
      case "minLength":
        categories.add("invalid_format");
        break;
      case "maxItems":
      case "minItems":
        categories.add("collection_out_of_range");
        break;
    }
  }
  return categories;
}

const uint64Fields = new Set([
  "monotonic_ns",
  "duration_ns",
  "delay_ns",
  "timeout_ns",
  "slot",
  "context_slot",
  "min_context_slot",
  "block_height",
  "last_valid_block_height",
  "fee_lamports",
  "units_consumed",
  "compute_units_consumed",
  "confirmations",
]);
const uint64Maximum = 18_446_744_073_709_551_615n;

function semanticCategory(document) {
  let numericFailure = false;
  function visit(value) {
    if (Array.isArray(value)) {
      for (const item of value) visit(item);
      return;
    }
    if (value === null || typeof value !== "object") return;
    for (const [name, child] of Object.entries(value)) {
      if (uint64Fields.has(name) && typeof child === "string") {
        const parsed = /^(0|[1-9][0-9]*)$/.test(child) ? BigInt(child) : undefined;
        if (parsed === undefined || parsed > uint64Maximum) numericFailure = true;
      }
      visit(child);
    }
  }
  visit(document);
  if (numericFailure) return "numeric_out_of_range";

  const events = Array.isArray(document.events) ? document.events : [document];
  for (const event of events) {
    const attributes = event.attributes ?? {};
    if (
      event.event_type === "solana.simulation.completed" &&
      ((attributes.transport_result === "response_received" &&
        attributes.rpc_result === "not_observed") ||
        (attributes.transport_result !== "response_received" &&
          attributes.rpc_result !== "not_observed") ||
        (attributes.rpc_result === "succeeded" && attributes.error !== undefined))
    ) {
      return "contradictory_evidence";
    }
    if (
      event.event_type === "solana.submission.completed" &&
      ((attributes.transport_result === "response_received" &&
        attributes.rpc_result === "not_observed") ||
        (attributes.transport_result !== "response_received" &&
          attributes.rpc_result !== "not_observed") ||
        (attributes.rpc_result === "accepted" && attributes.error !== undefined))
    ) {
      return "contradictory_evidence";
    }
    if (
      event.event_type === "solana.signing.completed" &&
      attributes.result === "completed" &&
      attributes.error !== undefined
    ) {
      return "contradictory_evidence";
    }
    if (
      event.event_type === "solana.execution.enriched" &&
      attributes.execution_result === "success" &&
      attributes.error !== undefined
    ) {
      return "contradictory_evidence";
    }
    if (
      event.event_type === "solana.confirmation_wait.completed" &&
      attributes.result === "commitment_reached" &&
      attributes.observed_commitment === undefined
    ) {
      return "missing_required_evidence";
    }
  }
  return undefined;
}

const schemaFiles = listJsonFiles(schemaRoot).filter((path) => !path.endsWith("manifest.json"));
const validator = new Ajv2020({ allErrors: true, strict: true });
addFormats(validator);
for (const path of schemaFiles) {
  const schema = readJson(path);
  validator.addSchema(schema, schema.$id);
}

const protocolManifest = readJson(join(schemaRoot, "manifest.json"));
const fixtureManifest = readJson(join(fixtureRoot, "manifest.json"));
if (fixtureManifest.protocol !== protocolManifest.protocol) {
  fail("fixture and protocol manifests name different protocols");
}
if (
  !protocolManifest.supported_versions.some(
    ({ version }) => version === fixtureManifest.schema_version,
  )
) {
  fail(`fixture corpus uses unsupported schema version ${fixtureManifest.schema_version}`);
}

const entries = [...fixtureManifest.valid, ...fixtureManifest.invalid];
const entryIds = new Set(entries.map(({ id }) => id));
if (entryIds.size !== entries.length) fail("fixture ids must be unique");

const registeredPaths = entries.map(({ path }) => path).sort();
const actualPaths = listJsonFiles(fixtureRoot)
  .filter((path) => path !== join(fixtureRoot, "manifest.json"))
  .map((path) => relative(fixtureRoot, path).split(sep).join("/"))
  .sort();
if (JSON.stringify(registeredPaths) !== JSON.stringify(actualPaths)) {
  fail("fixture manifest paths must exactly match the checked-in fixture files");
}

const observedEventTypes = new Set();
for (const fixture of fixtureManifest.valid) {
  const document = readJson(join(fixtureRoot, fixture.path));
  const validate = validator.getSchema(`${schemaIdBase}${fixture.schema}`);
  if (!validate) fail(`${fixture.id} references unknown schema ${fixture.schema}`);
  if (!validate(document)) {
    fail(`${fixture.id} must be valid: ${JSON.stringify(validate.errors)}`);
  }
  const semanticFailure = semanticCategory(document);
  if (semanticFailure) fail(`${fixture.id} fails semantic validation with ${semanticFailure}`);

  const events = Array.isArray(document.events) ? document.events : [document];
  for (const event of events) observedEventTypes.add(event.event_type);
}

const registeredEventTypes = Object.keys(protocolManifest.event_types).sort();
if (JSON.stringify([...observedEventTypes].sort()) !== JSON.stringify(registeredEventTypes)) {
  fail("valid fixtures must cover every registered event discriminator");
}

const stableCategories = new Set([
  "collection_out_of_range",
  "contradictory_evidence",
  "invalid_format",
  "missing_required_evidence",
  "missing_required_field",
  "numeric_out_of_range",
  "type_mismatch",
  "unknown_field",
  "unsupported_enum_value",
  "wrong_event_type",
]);
for (const fixture of fixtureManifest.invalid) {
  if (!stableCategories.has(fixture.expected_category)) {
    fail(`${fixture.id} uses unknown stable category ${fixture.expected_category}`);
  }

  const document = readJson(join(fixtureRoot, fixture.path));
  const validate = validator.getSchema(`${schemaIdBase}${fixture.schema}`);
  if (!validate) fail(`${fixture.id} references unknown schema ${fixture.schema}`);
  const schemaAccepted = validate(document);

  if (fixture.expected_stage === "schema") {
    if (schemaAccepted) fail(`${fixture.id} was unexpectedly accepted by JSON Schema`);
    const categories = schemaErrorCategories(validate.errors);
    if (!categories.has(fixture.expected_category)) {
      fail(
        `${fixture.id} did not produce ${fixture.expected_category}: ${JSON.stringify(validate.errors)}`,
      );
    }
  } else if (fixture.expected_stage === "semantic") {
    if (!schemaAccepted) {
      fail(`${fixture.id} must reach semantic validation: ${JSON.stringify(validate.errors)}`);
    }
    const actualCategory = semanticCategory(document);
    if (actualCategory !== fixture.expected_category) {
      fail(
        `${fixture.id} produced ${actualCategory ?? "no error"}, expected ${fixture.expected_category}`,
      );
    }
  } else {
    fail(`${fixture.id} uses unknown validation stage ${fixture.expected_stage}`);
  }
}

console.log(
  `Validated ${fixtureManifest.valid.length} valid and ${fixtureManifest.invalid.length} invalid shared protocol fixtures across ${observedEventTypes.size} event types.`,
);
