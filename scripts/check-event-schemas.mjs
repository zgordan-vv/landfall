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
  category: "capture_gap",
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
  result: "success",
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

const sampleEvents = eventEntries.map(([eventType, relativePath]) => {
  const schema = schemasByPath.get(relativePath);
  const eventDefinition = schema.allOf[1];
  const attributeSchema = eventDefinition.properties.attributes;
  const attributes = Object.fromEntries(
    attributeSchema.required.map((name) => {
      if (!(name in sampleValues)) fail(`no smoke-test value is registered for ${name}`);
      return [name, sampleValues[name]];
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
  `Validated ${schemaFiles.length} Draft 2020-12 resources, ${eventEntries.length} closed event types, and protocol smoke cases.`,
);
