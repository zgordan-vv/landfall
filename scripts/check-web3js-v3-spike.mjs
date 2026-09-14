import { readFile } from "node:fs/promises";

const spike = JSON.parse(
  await readFile(new URL("../config/web3js-v3-spike.json", import.meta.url), "utf8"),
);
const required = new Set([
  "install",
  "typed-compile",
  "blockhash",
  "simulation",
  "submission",
  "confirmation",
  "privacy",
]);
if (spike.client !== "@solana/web3.js" || spike.major !== 3)
  throw new Error("spike must target @solana/web3.js v3");
if (spike.status !== "recognized-unsupported" || spike.production_adapter !== false)
  throw new Error("web3.js v3 must remain non-production");
if (!Number.isInteger(spike.timebox_hours) || spike.timebox_hours < 1 || spike.timebox_hours > 40)
  throw new Error("spike timebox must be 1-40 hours");
if (
  !Array.isArray(spike.required_evidence) ||
  [...required].some((item) => !spike.required_evidence.includes(item))
)
  throw new Error("spike evidence checklist is incomplete");
console.log(`web3.js v3 spike verified: ${spike.timebox_hours}h, production adapter disabled`);
