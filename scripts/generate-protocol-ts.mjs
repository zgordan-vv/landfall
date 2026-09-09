import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { compile } from "json-schema-to-typescript";

const repositoryRoot = fileURLToPath(new URL("../", import.meta.url));
const schemaRoot = join(repositoryRoot, "schemas/events/v1");
const manifestPath = join(schemaRoot, "manifest.json");
const outputPath = join(repositoryRoot, "packages/protocol-ts/src/generated/v1.ts");
const checkOnly = process.argv.includes("--check");

const generatorConfig = {
  bannerComment: "",
  enableConstEnums: false,
  format: true,
  ignoreMinAndMaxItems: false,
  style: {
    bracketSpacing: true,
    printWidth: 100,
    semi: true,
    singleQuote: false,
    tabWidth: 2,
    trailingComma: "all",
    useTabs: false,
  },
  unknownAny: false,
  unreachableDefinitions: true,
};

function fail(message) {
  throw new Error(message);
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function clone(value) {
  return structuredClone(value);
}

function pascalCase(value) {
  return value
    .split(/[^A-Za-z0-9]+/u)
    .filter(Boolean)
    .map((part) => `${part[0].toUpperCase()}${part.slice(1)}`)
    .join("");
}

function hash(value) {
  return createHash("sha256").update(value).digest("hex");
}

const manifest = readJson(manifestPath);
const version = manifest.supported_versions.find((entry) => entry.version === "1.0");
if (!version) fail("manifest does not register event protocol 1.0");

const generatorMetadata = manifest.typescript_generation;
const configSha256 = hash(JSON.stringify(generatorConfig));
if (
  generatorMetadata?.generator !== "json-schema-to-typescript" ||
  generatorMetadata?.version !== "16.0.0" ||
  generatorMetadata?.config_sha256 !== configSha256 ||
  generatorMetadata?.output !== "packages/protocol-ts/src/generated/v1.ts"
) {
  fail("manifest TypeScript generator metadata differs from the executable configuration");
}

const identifiers = readJson(join(schemaRoot, "1.0/common/identifiers.schema.json"));
const values = readJson(join(schemaRoot, "1.0/common/values.schema.json"));
const enums = readJson(join(schemaRoot, "1.0/common/enums.schema.json"));
const envelope = readJson(join(schemaRoot, version.envelope_schema));
const batch = readJson(join(schemaRoot, version.batch_schema));

const identifierNames = Object.fromEntries(
  Object.keys(identifiers.$defs).map((name) => [name, pascalCase(name)]),
);
const valueNames = Object.fromEntries(
  Object.keys(values.$defs).map((name) => [name, pascalCase(name)]),
);
const enumNames = Object.fromEntries(
  Object.keys(enums.$defs).map((name) => [name, pascalCase(name)]),
);

function resolveValueDefinition(name) {
  const definition = clone(values.$defs[name]);
  if (definition.$ref?.startsWith("#/$defs/")) {
    const target = resolveValueDefinition(definition.$ref.slice("#/$defs/".length));
    delete definition.$ref;
    return { ...target, ...definition };
  }
  return definition;
}

function rewriteReferences(value) {
  if (Array.isArray(value)) return value.map(rewriteReferences);
  if (value === null || typeof value !== "object") return value;

  const rewritten = Object.fromEntries(
    Object.entries(value).map(([key, child]) => [key, rewriteReferences(child)]),
  );
  if (typeof value.$ref !== "string") return rewritten;

  const reference = value.$ref;
  const fragment = reference.split("#/$defs/")[1];
  let target;
  if (reference.includes("identifiers.schema.json#/$defs/")) target = identifierNames[fragment];
  if (reference.includes("values.schema.json#/$defs/")) target = valueNames[fragment];
  if (reference.includes("enums.schema.json#/$defs/")) target = enumNames[fragment];
  if (reference.endsWith("source.schema.json")) target = "EventSource";
  if (reference.endsWith("errors.schema.json")) target = "NormalizedError";
  if (reference.endsWith("fingerprint.schema.json")) target = "SignedBytesFingerprint";
  if (!target) fail(`unregistered generator reference ${reference}`);

  return { ...rewritten, $ref: `#/$defs/${target}` };
}

const definitions = {};
const uuidDefinition = clone(identifiers.$defs.uuid_v7);
for (const [schemaName, typeName] of Object.entries(identifierNames)) {
  definitions[typeName] = { ...clone(uuidDefinition), title: typeName };
  if (schemaName === "uuid_v7") definitions[typeName].description = "Canonical lowercase UUIDv7.";
}
for (const [schemaName, typeName] of Object.entries(valueNames)) {
  definitions[typeName] = { ...resolveValueDefinition(schemaName), title: typeName };
}
for (const [schemaName, typeName] of Object.entries(enumNames)) {
  definitions[typeName] = { ...clone(enums.$defs[schemaName]), title: typeName };
}

for (const [typeName, path] of [
  ["EventSource", "1.0/common/source.schema.json"],
  ["NormalizedError", "1.0/common/errors.schema.json"],
  ["SignedBytesFingerprint", "1.0/common/fingerprint.schema.json"],
]) {
  const source = readJson(join(schemaRoot, path));
  definitions[typeName] = {
    ...rewriteReferences(source),
    $id: undefined,
    $schema: undefined,
    title: typeName,
  };
}

const commonProperties = rewriteReferences(envelope.properties);
const commonRequired = new Set(envelope.required);
const eventTypeNames = {};

for (const [eventType, eventPath] of Object.entries(manifest.event_types)) {
  const source = readJson(join(schemaRoot, eventPath));
  const specialization = source.allOf?.[1];
  if (!specialization?.properties?.attributes) {
    fail(`${eventPath} does not have the registered envelope specialization shape`);
  }

  const typeName = `${pascalCase(eventType)}Event`;
  eventTypeNames[eventType] = typeName;
  const required = new Set([...commonRequired, ...(specialization.required ?? [])]);
  definitions[typeName] = {
    title: typeName,
    type: "object",
    properties: {
      ...clone(commonProperties),
      ...rewriteReferences(specialization.properties),
    },
    required: [...required],
    additionalProperties: false,
  };
  if (specialization.anyOf) {
    definitions[typeName].anyOf = rewriteReferences(specialization.anyOf).map((branch) => ({
      ...branch,
      additionalProperties: false,
    }));
  }
}

definitions.WireEvent = {
  title: "WireEvent",
  oneOf: Object.values(eventTypeNames).map((typeName) => ({
    $ref: `#/$defs/${typeName}`,
  })),
};

const generatedSchema = {
  $schema: manifest.json_schema_draft,
  title: "EventBatch",
  type: "object",
  properties: {
    batch_id: rewriteReferences(batch.properties.batch_id),
    sent_at: rewriteReferences(batch.properties.sent_at),
    events: {
      ...clone(batch.properties.events),
      items: { $ref: "#/$defs/WireEvent" },
    },
  },
  required: clone(batch.required),
  additionalProperties: false,
  $defs: definitions,
};

const schemaHashInput = manifest.resources
  .map((path) => `${path}\0${readFileSync(join(schemaRoot, path), "utf8")}`)
  .join("\0");
const schemaSha256 = hash(schemaHashInput);
const generatedBody = await compile(generatedSchema, "EventBatch", generatorConfig);
const output = `// DO NOT EDIT: generated from schemas/events/v1.\n// Generator: json-schema-to-typescript 16.0.0\n// Schema SHA-256: ${schemaSha256}\n// Config SHA-256: ${configSha256}\n\n${generatedBody}`;

if (checkOnly) {
  let current;
  try {
    current = readFileSync(outputPath, "utf8");
  } catch {
    fail(`${relative(repositoryRoot, outputPath)} is missing; run pnpm generate:protocol`);
  }
  if (current !== output) {
    fail(`${relative(repositoryRoot, outputPath)} is stale; run pnpm generate:protocol`);
  }
} else {
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, output);
  process.stdout.write(`Generated ${relative(repositoryRoot, outputPath)}\n`);
}
