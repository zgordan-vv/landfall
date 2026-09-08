import { readFileSync, readdirSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const draft = "https://json-schema.org/draft/2020-12/schema";
const schemaIdBase = "https://schemas.landfall.dev/events/v1/";
const repositoryRoot = fileURLToPath(new URL("../", import.meta.url));
const schemaRoot = join(repositoryRoot, "schemas/events/v1");
const manifestPath = join(schemaRoot, "manifest.json");

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

function resolveJsonPointer(document, fragment, source) {
  if (fragment === "" || fragment === "#") return;
  if (!fragment.startsWith("#/")) fail(`${source} uses unsupported $ref fragment ${fragment}`);

  let current = document;
  for (const encodedSegment of fragment.slice(2).split("/")) {
    const segment = decodeURIComponent(encodedSegment).replaceAll("~1", "/").replaceAll("~0", "~");
    if (current === null || typeof current !== "object" || !(segment in current)) {
      fail(`${source} points to missing JSON Pointer ${fragment}`);
    }
    current = current[segment];
  }
}

const manifest = readJson(manifestPath);
const schemaFiles = listJsonFiles(schemaRoot).filter((path) => path !== manifestPath);
const relativeSchemaPaths = schemaFiles
  .map((path) => relative(schemaRoot, path).split(sep).join("/"))
  .sort();
const declaredResources = [...manifest.resources].sort();

if (JSON.stringify(relativeSchemaPaths) !== JSON.stringify(declaredResources)) {
  fail("manifest resources must exactly match the checked-in schema files");
}

const schemasById = new Map();
const schemasByPath = new Map();
for (const path of schemaFiles) {
  const schema = readJson(path);
  const relativePath = relative(schemaRoot, path).split(sep).join("/");
  const expectedId = `${schemaIdBase}${relativePath}`;

  if (schema.$schema !== draft) fail(`${relativePath} does not declare Draft 2020-12`);
  if (schema.$id !== expectedId) fail(`${relativePath} must use $id ${expectedId}`);
  if (schemasById.has(schema.$id)) fail(`duplicate schema $id ${schema.$id}`);

  schemasById.set(schema.$id, schema);
  schemasByPath.set(relativePath, schema);
}

const validator = new Ajv2020({ allErrors: true, strict: true });
addFormats(validator);
for (const [schemaId, schema] of schemasById) validator.addSchema(schema, schemaId);
for (const [schemaId, schema] of schemasById) {
  if (!validator.validateSchema(schema)) {
    fail(`${schemaId} is not a valid Draft 2020-12 schema: ${validator.errorsText()}`);
  }
}

function parseCanonicalDecimal(value, minimum, maximum, signed) {
  if (typeof value !== "string") return undefined;
  const lexicalPattern = signed ? /^(0|-?[1-9][0-9]*)$/ : /^(0|[1-9][0-9]*)$/;
  if (!lexicalPattern.test(value)) return undefined;

  const parsed = BigInt(value);
  if (parsed < minimum || parsed > maximum) return undefined;
  if (parsed.toString(10) !== value) return undefined;
  return parsed;
}

const valuesSchemaId = `${schemaIdBase}1.0/common/values.schema.json`;
const validateUint64Lexical = validator.getSchema(`${valuesSchemaId}#/$defs/uint64_decimal`);
const validateInt64Lexical = validator.getSchema(`${valuesSchemaId}#/$defs/int64_decimal`);
const uint64Minimum = 0n;
const uint64Maximum = 18446744073709551615n;
const int64Minimum = -9223372036854775808n;
const int64Maximum = 9223372036854775807n;

for (const value of ["0", "1", "9007199254740991", "9007199254740992", "18446744073709551615"]) {
  if (!validateUint64Lexical(value)) fail(`uint64 schema rejects canonical value ${value}`);
  if (parseCanonicalDecimal(value, uint64Minimum, uint64Maximum, false) === undefined) {
    fail(`uint64 semantic parser rejects in-range value ${value}`);
  }
}
for (const value of [0, "", "00", "+1", "-0", "-1", "1.0", "1e3", " 1", "1 "]) {
  if (validateUint64Lexical(value)) fail(`uint64 schema accepts non-canonical value ${value}`);
  if (parseCanonicalDecimal(value, uint64Minimum, uint64Maximum, false) !== undefined) {
    fail(`uint64 semantic parser accepts non-canonical value ${value}`);
  }
}
for (const value of ["18446744073709551616", "99999999999999999999"]) {
  if (parseCanonicalDecimal(value, uint64Minimum, uint64Maximum, false) !== undefined) {
    fail(`uint64 semantic parser accepts overflow value ${value}`);
  }
}

for (const value of ["-9223372036854775808", "-1", "0", "1", "9223372036854775807"]) {
  if (!validateInt64Lexical(value)) fail(`int64 schema rejects canonical value ${value}`);
  if (parseCanonicalDecimal(value, int64Minimum, int64Maximum, true) === undefined) {
    fail(`int64 semantic parser rejects in-range value ${value}`);
  }
}
for (const value of ["-9223372036854775809", "9223372036854775808"]) {
  if (parseCanonicalDecimal(value, int64Minimum, int64Maximum, true) !== undefined) {
    fail(`int64 semantic parser accepts overflow value ${value}`);
  }
}

const uint64Manifest = manifest.numeric_domains?.uint64_decimal;
const int64Manifest = manifest.numeric_domains?.int64_decimal;
if (
  uint64Manifest?.minimum !== uint64Minimum.toString(10) ||
  uint64Manifest?.maximum !== uint64Maximum.toString(10) ||
  int64Manifest?.minimum !== int64Minimum.toString(10) ||
  int64Manifest?.maximum !== int64Maximum.toString(10)
) {
  fail("manifest numeric-domain bounds differ from executable boundary checks");
}

const enumsPath = "1.0/common/enums.schema.json";
const enumsSchema = schemasByPath.get(enumsPath);
if (manifest.supported_versions?.[0]?.enum_schema !== enumsPath) {
  fail("manifest supported version does not declare the canonical enum schema");
}
const enumNames = Object.keys(enumsSchema.$defs);
for (const [enumName, enumSchema] of Object.entries(enumsSchema.$defs)) {
  if (!Array.isArray(enumSchema.enum) || enumSchema.enum.length === 0) {
    fail(`${enumName} is not a non-empty closed enum`);
  }
  if (new Set(enumSchema.enum).size !== enumSchema.enum.length) {
    fail(`${enumName} contains duplicate values`);
  }
  if (
    !enumSchema.enum.every((value) => typeof value === "string" && /^[a-z][a-z0-9_]*$/.test(value))
  ) {
    fail(`${enumName} contains a non-canonical enum value`);
  }

  const validateEnum = validator.getSchema(`${enumsSchema.$id}#/$defs/${enumName}`);
  for (const value of enumSchema.enum) {
    if (!validateEnum(value)) fail(`${enumName} rejects its declared value ${value}`);
  }
  if (validateEnum("unregistered_value")) fail(`${enumName} accepts an unknown value`);
}

const enumBindings = [
  ["1.0/envelope.schema.json", "privacy_mode", "privacy_mode", false],
  ["1.0/common/source.schema.json", "kind", "source_kind", false],
  ["1.0/common/errors.schema.json", "category", "normalized_error_category", false],
  ["1.0/events/trace-created.schema.json", "transaction_version", "transaction_version", true],
  ["1.0/events/blockhash-acquired.schema.json", "result", "blockhash_result", true],
  ["1.0/events/simulation-started.schema.json", "commitment", "commitment", true],
  ["1.0/events/simulation-completed.schema.json", "transport_result", "transport_result", true],
  ["1.0/events/simulation-completed.schema.json", "rpc_result", "simulation_rpc_result", true],
  ["1.0/events/signing-started.schema.json", "transaction_version", "transaction_version", true],
  ["1.0/events/signing-completed.schema.json", "result", "signing_result", true],
  ["1.0/events/submission-started.schema.json", "encoding", "submission_encoding", true],
  ["1.0/events/submission-started.schema.json", "preflight_commitment", "commitment", true],
  ["1.0/events/submission-completed.schema.json", "transport_result", "transport_result", true],
  ["1.0/events/submission-completed.schema.json", "rpc_result", "submission_rpc_result", true],
  ["1.0/events/confirmation-wait-started.schema.json", "commitment", "commitment", true],
  [
    "1.0/events/confirmation-wait-completed.schema.json",
    "result",
    "confirmation_wait_result",
    true,
  ],
  ["1.0/events/confirmation-wait-completed.schema.json", "observed_commitment", "commitment", true],
  ["1.0/events/status-observed.schema.json", "source_result", "status_source_result", true],
  ["1.0/events/status-observed.schema.json", "commitment", "commitment", true],
  ["1.0/events/execution-enriched.schema.json", "commitment", "commitment", true],
  ["1.0/events/execution-enriched.schema.json", "execution_result", "execution_result", true],
  ["1.0/events/business-outcome-observed.schema.json", "outcome", "business_outcome", true],
  ["1.0/events/data-quality-detected.schema.json", "category", "data_quality_category", true],
  ["1.0/events/data-quality-detected.schema.json", "severity", "data_quality_severity", true],
  ["1.0/events/data-quality-detected.schema.json", "impact", "data_quality_impact", true],
];
for (const [relativePath, propertyName, enumName, isEventAttribute] of enumBindings) {
  if (!enumNames.includes(enumName)) fail(`enum binding references unknown definition ${enumName}`);
  const schema = schemasByPath.get(relativePath);
  const properties = isEventAttribute
    ? schema.allOf[1].properties.attributes.properties
    : schema.properties;
  const expectedSuffix = `enums.schema.json#/$defs/${enumName}`;
  if (!properties[propertyName]?.$ref?.endsWith(expectedSuffix)) {
    fail(`${relativePath} does not map ${propertyName} to ${enumName}`);
  }
}

const largeIntegerFields = new Map([
  ["monotonic_ns", "duration_ns_decimal"],
  ["duration_ns", "duration_ns_decimal"],
  ["delay_ns", "duration_ns_decimal"],
  ["timeout_ns", "duration_ns_decimal"],
  ["slot", "slot_decimal"],
  ["context_slot", "slot_decimal"],
  ["min_context_slot", "slot_decimal"],
  ["block_height", "block_height_decimal"],
  ["last_valid_block_height", "block_height_decimal"],
  ["fee_lamports", "lamports_decimal"],
  ["units_consumed", "compute_units_decimal"],
  ["compute_units_consumed", "compute_units_decimal"],
  ["confirmations", "confirmation_count_decimal"],
]);
for (const [schemaId, schema] of schemasById) {
  walk(schema, (node) => {
    if (node === null || typeof node !== "object" || !node.properties) return;
    for (const [propertyName, propertySchema] of Object.entries(node.properties)) {
      const expectedDefinition = largeIntegerFields.get(propertyName);
      if (expectedDefinition) {
        const expectedSuffix = `values.schema.json#/$defs/${expectedDefinition}`;
        if (!propertySchema.$ref?.endsWith(expectedSuffix)) {
          fail(`${schemaId} does not map ${propertyName} to ${expectedDefinition}`);
        }
      }
      if (propertySchema.type === "integer") {
        if (
          !Number.isSafeInteger(propertySchema.minimum) ||
          !Number.isSafeInteger(propertySchema.maximum) ||
          propertySchema.minimum > propertySchema.maximum
        ) {
          fail(`${schemaId} has an unbounded or unsafe JSON integer ${propertyName}`);
        }
      }
    }
  });
}

for (const [sourceId, schema] of schemasById) {
  walk(schema, (node) => {
    if (node === null || typeof node !== "object" || typeof node.$ref !== "string") return;

    const resolved = new URL(node.$ref, sourceId);
    const targetId = `${resolved.origin}${resolved.pathname}`;
    if (!targetId.startsWith(schemaIdBase)) {
      fail(`${sourceId} references an unregistered schema authority: ${node.$ref}`);
    }

    const target = schemasById.get(targetId);
    if (!target) fail(`${sourceId} references missing schema ${targetId}`);
    resolveJsonPointer(target, resolved.hash, sourceId);
  });
}

const eventEntries = Object.entries(manifest.event_types);
if (eventEntries.length !== 15) fail(`expected 15 event types, found ${eventEntries.length}`);

const eventTypes = new Set();
for (const [eventType, relativePath] of eventEntries) {
  const schema = schemasByPath.get(relativePath);
  if (!schema) fail(`${eventType} points to missing resource ${relativePath}`);

  const eventDefinition = schema.allOf?.[1];
  const declaredType = eventDefinition?.properties?.event_type?.const;
  if (declaredType !== eventType) fail(`${relativePath} does not const-discriminate ${eventType}`);
  if (eventDefinition?.properties?.attributes?.additionalProperties !== false) {
    fail(`${relativePath} does not close its attributes object`);
  }
  if (eventTypes.has(eventType)) fail(`duplicate event type ${eventType}`);
  eventTypes.add(eventType);
}

const batch = schemasByPath.get("1.0/batch.schema.json");
validator.getSchema(batch.$id) ?? fail("the complete batch schema graph does not compile");
const batchBranches = batch?.properties?.events?.items?.oneOf ?? [];
const batchEventTypes = new Set(
  batchBranches.map((branch) => {
    const resolved = new URL(branch.$ref, batch.$id);
    const targetId = `${resolved.origin}${resolved.pathname}`;
    return schemasById.get(targetId)?.allOf?.[1]?.properties?.event_type?.const;
  }),
);
if (batchBranches.length !== 15 || batchEventTypes.size !== 15) {
  fail("batch event union must contain each of the 15 event schemas exactly once");
}
for (const eventType of eventTypes) {
  if (!batchEventTypes.has(eventType)) fail(`batch event union omits ${eventType}`);
}

const sampleUuid = "0198ef00-0000-7000-8000-000000000001";
const sampleValues = {
  attempt_id: sampleUuid,
  attempt_sequence: 1,
  category: "observer_gap",
  commitment: "processed",
  delay_ns: "1",
  duration_ns: "1",
  encoding: "base64",
  execution_result: "success",
  flow: "swap",
  impact: "reduced_certainty",
  logs_present: false,
  observer_source_id: sampleUuid,
  outcome: "success",
  previous_attempt_id: sampleUuid,
  reason: "transport_timeout",
  retry_sequence: 1,
  route_id: sampleUuid,
  search_transaction_history: false,
  severity: "warning",
  sig_verify: false,
  signing_id: sampleUuid,
  simulation_id: sampleUuid,
  skip_preflight: false,
  slot: "1",
  source_result: "found",
  timeout_ns: "1",
  transaction_version: "legacy",
  transport_result: "response_received",
  replace_recent_blockhash: false,
  rpc_result: "accepted",
  wait_id: sampleUuid,
};

const eventSampleOverrides = {
  "solana.blockhash.acquired": { result: "acquired" },
  "solana.confirmation_wait.completed": { result: "commitment_reached" },
  "solana.signing.completed": { result: "completed" },
  "solana.simulation.completed": { rpc_result: "succeeded" },
};

const sampleEvents = eventEntries.map(([eventType, relativePath]) => {
  const schema = schemasByPath.get(relativePath);
  const eventDefinition = schema.allOf[1];
  const attributeSchema = eventDefinition.properties.attributes;
  const attributes = Object.fromEntries(
    attributeSchema.required.map((name) => {
      const sampleValue = eventSampleOverrides[eventType]?.[name] ?? sampleValues[name];
      if (sampleValue === undefined) fail(`no smoke-test value is registered for ${name}`);
      return [name, sampleValue];
    }),
  );
  const event = {
    schema_version: "1.0",
    event_id: sampleUuid,
    event_type: eventType,
    occurred_at: "2026-09-07T12:00:00Z",
    project_id: sampleUuid,
    environment_id: sampleUuid,
    source: { kind: "sdk", name: "landfall-js", version: "0.1.0" },
    privacy_mode: "standard",
    privacy_policy_version: "1.0",
    redaction_version: "1.0",
    attributes,
  };
  if (eventType === "solana.business_outcome.observed") {
    event.business_action_id = sampleUuid;
  } else if (eventType !== "landfall.data_quality.detected") {
    event.trace_id = sampleUuid;
  }

  const validateEvent = validator.getSchema(schema.$id);
  if (!validateEvent(event)) {
    fail(`${eventType} rejects its minimal smoke event: ${JSON.stringify(validateEvent.errors)}`);
  }

  const unknownAttribute = structuredClone(event);
  unknownAttribute.attributes.unregistered = true;
  if (validateEvent(unknownAttribute)) fail(`${eventType} accepts an unknown attribute`);

  return event;
});

const validateBatch = validator.getSchema(batch.$id);
const sampleBatch = {
  batch_id: sampleUuid,
  sent_at: "2026-09-07T12:00:01Z",
  events: sampleEvents,
};
if (!validateBatch(sampleBatch)) {
  fail(`batch rejects the complete event union: ${JSON.stringify(validateBatch.errors)}`);
}
const unknownEnvelopeField = structuredClone(sampleBatch);
unknownEnvelopeField.events[0].unregistered = true;
if (validateBatch(unknownEnvelopeField)) fail("batch accepts an unknown event-envelope field");

const missingMonotonicInstance = structuredClone(sampleBatch);
missingMonotonicInstance.events[0].monotonic_ns = "1";
if (validateBatch(missingMonotonicInstance)) {
  fail("batch accepts monotonic_ns without source.instance_id");
}
missingMonotonicInstance.events[0].source.instance_id = sampleUuid;
if (!validateBatch(missingMonotonicInstance)) {
  fail("batch rejects monotonic_ns with a valid source.instance_id");
}

const missingBusinessScope = structuredClone(sampleBatch);
const businessEvent = missingBusinessScope.events.find(
  (event) => event.event_type === "solana.business_outcome.observed",
);
delete businessEvent.business_action_id;
if (validateBatch(missingBusinessScope)) {
  fail("batch accepts a business outcome without trace or business-action scope");
}

const envelope = schemasByPath.get("1.0/envelope.schema.json");
if (envelope.additionalProperties !== false) fail("envelope permits unknown root properties");
const requiredEnvelopeFields = [
  "schema_version",
  "event_id",
  "event_type",
  "occurred_at",
  "project_id",
  "environment_id",
  "source",
  "privacy_mode",
  "privacy_policy_version",
  "redaction_version",
  "attributes",
];
for (const field of requiredEnvelopeFields) {
  if (!envelope.required?.includes(field)) fail(`envelope does not require ${field}`);
}
if (!envelope.dependentSchemas?.monotonic_ns) {
  fail("envelope must require source.instance_id when monotonic_ns is present");
}

const prohibitedPropertyNames = new Set([
  "private_key",
  "seed_phrase",
  "signed_transaction_bytes",
  "raw_transaction",
  "authorization",
  "cookie",
]);
for (const [sourceId, schema] of schemasById) {
  walk(schema, (node) => {
    if (node === null || typeof node !== "object" || !node.properties) return;
    for (const propertyName of Object.keys(node.properties)) {
      if (prohibitedPropertyNames.has(propertyName)) {
        fail(`${sourceId} allowlists prohibited property ${propertyName}`);
      }
    }
  });
}

console.log(
  `Validated ${schemaFiles.length} Draft 2020-12 resources, ${eventEntries.length} closed event types, ${enumNames.length} enums, numeric boundaries, and protocol smoke cases.`,
);
