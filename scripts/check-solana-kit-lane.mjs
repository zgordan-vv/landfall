import { readFile } from "node:fs/promises";

const lane = JSON.parse(
  await readFile(new URL("../config/solana-kit-lane.json", import.meta.url), "utf8"),
);
if (lane.client !== "@solana/kit" || lane.version !== "8.2.0")
  throw new Error("Solana Kit P0 lane must be exactly @solana/kit 8.2.0");
if (!Array.isArray(lane.plugins) || lane.plugins.length !== 0)
  throw new Error("P0 lane must not silently add Kit plugins");
if (lane.node !== ">=24.11.0 <25.0.0")
  throw new Error("Solana Kit lane must use the supported Node 24 range");
if (lane.status !== "supported-p0") throw new Error("Solana Kit lane is not marked supported-p0");
console.log(`Solana Kit lane verified: ${lane.client}@${lane.version} (no plugins)`);
