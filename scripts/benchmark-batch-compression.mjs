import { gzipSync } from "node:zlib";

const batchSizes = [100, 1_000];
const results = batchSizes.map((count) => {
  const events = Array.from({ length: count }, (_, index) => ({ schema_version: "1.0", event_type: "solana.trace.created", event_id: `evt-${index.toString().padStart(6, "0")}`, project_id: "project-bench", environment_id: "environment-bench", attributes: { flow: "transfer", route: `route-${index % 4}` } }));
  const raw = Buffer.byteLength(JSON.stringify({ events }));
  const compressed = gzipSync(Buffer.from(JSON.stringify({ events }))).byteLength;
  return { events: count, raw_bytes: raw, compressed_bytes: compressed, compression_ratio: compressed / raw };
});
console.log(JSON.stringify({ results }));
