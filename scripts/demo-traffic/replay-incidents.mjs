#!/usr/bin/env node
/**
 * Replays the golden incident fixtures through /v1/ingest into a dedicated
 * "scenario library" environment, so incident types that cannot be produced
 * on demand (observer disagreement, missing block height, unsupported
 * transaction features, ...) are visible next to live demo traffic.
 *
 * Every identifier is remapped to a fresh UUIDv7 and timestamps are shifted to
 * the present while preserving the fixture's relative timing, so the script can
 * be run repeatedly. The fixtures carry no signatures, so no observer jobs are
 * queued for them.
 *
 *   LANDFALL_PROJECT_ID=... LANDFALL_SCENARIO_ENVIRONMENT_ID=... \
 *     node scripts/demo-traffic/replay-incidents.mjs [fixture-name ...]
 */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createIngestClient, uuidV7 } from "./lib/landfall.mjs";

const repositoryRoot = fileURLToPath(new URL("../..", import.meta.url));
const incidentsDir = join(repositoryRoot, "fixtures/protocol/v1/valid/incidents");

const required = (name) => {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`${name} is required`);
  return value;
};
const projectId = required("LANDFALL_PROJECT_ID");
const environmentId = required("LANDFALL_SCENARIO_ENVIRONMENT_ID");
const post = createIngestClient({
  collectorUrl: process.env.LANDFALL_COLLECTOR_URL ?? "http://127.0.0.1:8080",
  ingestToken: process.env.LANDFALL_INGEST_TOKEN,
});

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

function remap(value, ids) {
  if (Array.isArray(value)) return value.map((item) => remap(item, ids));
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, child]) => [key, remap(child, ids)]),
    );
  }
  if (typeof value === "string" && UUID.test(value)) {
    if (!ids.has(value)) ids.set(value, uuidV7());
    return ids.get(value);
  }
  return value;
}

async function replay(file) {
  const batch = JSON.parse(readFileSync(join(incidentsDir, file), "utf8"));
  const latest = Math.max(...batch.events.map((event) => Date.parse(event.occurred_at)));
  const shift = Date.now() - 5_000 - latest;
  const ids = new Map();
  const events = batch.events.map((event) => ({
    ...remap(event, ids),
    project_id: projectId,
    environment_id: environmentId,
    occurred_at: new Date(Date.parse(event.occurred_at) + shift).toISOString(),
  }));
  await post(events);
  const traces = new Set(events.map((event) => event.trace_id).filter(Boolean));
  console.log(`${file}: ${events.length} events, ${traces.size} trace(s)`);
}

const requested = process.argv.slice(2);
const files = readdirSync(incidentsDir)
  .filter((file) => file.endsWith(".batch.json"))
  .filter((file) => requested.length === 0 || requested.some((name) => file.startsWith(name)));
if (files.length === 0) throw new Error("no matching incident fixtures");
for (const file of files) await replay(file);
