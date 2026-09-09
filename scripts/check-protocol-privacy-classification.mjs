import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const schemaRoot = path.join(repositoryRoot, "schemas/events/v1/1.0");

async function readJson(relativePath) {
  return JSON.parse(await readFile(path.join(repositoryRoot, relativePath), "utf8"));
}

function sortedKeys(value) {
  return Object.keys(value).sort();
}

function assertExactFields(scope, schemaProperties, classifications) {
  assert.deepEqual(
    sortedKeys(classifications),
    sortedKeys(schemaProperties),
    `${scope} privacy fields must exactly match its schema properties`,
  );
}

const registry = await readJson("docs/event-privacy-classification.json");
const protocolManifest = await readJson("schemas/events/v1/manifest.json");

assert.equal(registry.schema_version, "1.0");
assert.match(registry.classification_version, /^\d+\.\d+$/u);
assert.deepEqual(protocolManifest.privacy_classification, {
  version: registry.classification_version,
  registry: "docs/event-privacy-classification.json",
});

const allowedSensitivities = new Set(["structural", "low", "moderate", "high"]);
const allowedActions = new Set([
  "validate_children",
  "store",
  "store_bounded",
  "local_only",
  "store_scoped",
  "redact_then_store",
]);
for (const [className, definition] of Object.entries(registry.classes)) {
  assert.ok(
    allowedSensitivities.has(definition.sensitivity),
    `${className} has an unknown sensitivity`,
  );
  for (const mode of ["standard", "full", "strict"]) {
    assert.ok(
      allowedActions.has(definition[mode]),
      `${className}.${mode} has an unknown handling action`,
    );
  }
}

const batchSchema = await readJson("schemas/events/v1/1.0/batch.schema.json");
const envelopeSchema = await readJson("schemas/events/v1/1.0/envelope.schema.json");
const sourceSchema = await readJson("schemas/events/v1/1.0/common/source.schema.json");
const errorSchema = await readJson("schemas/events/v1/1.0/common/errors.schema.json");
const fingerprintSchema = await readJson("schemas/events/v1/1.0/common/fingerprint.schema.json");

assertExactFields("batch", batchSchema.properties, registry.fields.batch);
assertExactFields("envelope", envelopeSchema.properties, registry.fields.envelope);
assertExactFields("source", sourceSchema.properties, registry.fields.source);
assertExactFields("normalized_error", errorSchema.properties, registry.fields.normalized_error);
assertExactFields(
  "signed_bytes_fingerprint",
  fingerprintSchema.properties,
  registry.fields.signed_bytes_fingerprint,
);

const registeredEventTypes = protocolManifest.supported_versions.find(
  ({ version }) => version === registry.schema_version,
).event_types;
assert.deepEqual(
  sortedKeys(registry.event_attributes),
  [...registeredEventTypes].sort(),
  "privacy registry must classify every registered event type exactly once",
);

let classifiedAttributeCount = 0;
for (const eventType of registeredEventTypes) {
  const relativeSchemaPath = protocolManifest.event_types[eventType];
  const eventSchema = await readJson(`schemas/events/v1/${relativeSchemaPath}`);
  const eventShape = eventSchema.allOf.find((part) => part.properties?.attributes?.properties);
  assert.ok(eventShape, `${eventType} must define event attributes`);
  const attributeProperties = eventShape.properties.attributes.properties;
  const classifications = registry.event_attributes[eventType];
  assertExactFields(eventType, attributeProperties, classifications);
  classifiedAttributeCount += Object.keys(classifications).length;

  for (const [fieldName, fieldSchema] of Object.entries(attributeProperties)) {
    if (fieldSchema.$ref?.endsWith("#/$defs/bounded_text")) {
      assert.equal(
        classifications[fieldName],
        "bounded_free_text",
        `${eventType}.${fieldName} must use the redacted free-text class`,
      );
    }
  }
}

assert.equal(
  registry.fields.normalized_error.message,
  "bounded_free_text",
  "normalized error messages must pass through redaction",
);
assert.equal(
  registry.classes.public_chain_identifier.strict,
  "local_only",
  "strict mode must keep signatures and blockhashes local",
);
assert.equal(
  registry.classes.pseudonymous_fingerprint.strict,
  "store_scoped",
  "strict mode must retain environment-scoped retry correlation",
);

const knownClassNames = new Set(Object.keys(registry.classes));
for (const [scope, classifications] of Object.entries(registry.fields)) {
  for (const className of Object.values(classifications)) {
    assert.ok(knownClassNames.has(className), `${scope} uses unknown class ${className}`);
  }
}
for (const [eventType, classifications] of Object.entries(registry.event_attributes)) {
  for (const className of Object.values(classifications)) {
    assert.ok(knownClassNames.has(className), `${eventType} uses unknown class ${className}`);
  }
}

const schemaFiles = protocolManifest.resources.map((resource) =>
  path.join(schemaRoot, resource.replace(/^1\.0\//u, "")),
);
for (const schemaFile of schemaFiles) {
  const contents = await readFile(schemaFile, "utf8");
  for (const prohibitedField of registry.prohibited_fields) {
    assert.ok(
      !new RegExp(`"${prohibitedField}"\\s*:`, "u").test(contents),
      `${path.relative(repositoryRoot, schemaFile)} declares prohibited field ${prohibitedField}`,
    );
  }
}

const sharedFieldCount = Object.values(registry.fields).reduce(
  (count, fields) => count + Object.keys(fields).length,
  0,
);
console.log(
  `Classified ${sharedFieldCount} shared and ${classifiedAttributeCount} event-attribute fields across ${registeredEventTypes.length} event types.`,
);
