import { readFile } from "node:fs/promises";

const scenarios = JSON.parse(await readFile(new URL("../fixtures/solana/lifecycle-scenarios.json", import.meta.url), "utf8"));
const expected = new Set(["success", "simulation-failure", "timeout-later-success", "expiration"]);
if (!Array.isArray(scenarios) || new Set(scenarios.map((item) => item.id)).size !== expected.size || scenarios.some((item) => !expected.has(item.id))) throw new Error("lifecycle fixture set is incomplete");
const expiration = scenarios.find((item) => item.id === "expiration");
if (Number(expiration.observed_block_height) <= Number(expiration.last_valid_block_height)) throw new Error("expiration fixture must observe after validity height");
if (scenarios.find((item) => item.id === "timeout-later-success").observed !== "confirmed") throw new Error("timeout fixture must land later");
console.log(`Solana lifecycle fixtures verified: ${scenarios.length} scenarios`);
