#!/usr/bin/env node
/**
 * Exercises the x402 spend-policy engine with a stream of authorization
 * requests from a demo agent. Every decision (approved, or denied for amount,
 * daily cap, or unlisted merchant) is made by the real /v1/x402/authorize
 * endpoint and recorded in the audit log. No wallet is involved and no payment
 * is made, so approved decisions stay unsettled.
 *
 *   LANDFALL_X402_TOKEN=... LANDFALL_X402_POLICY_ID=... \
 *     node scripts/demo-traffic/x402-decisions.mjs --count 12
 */
import { parseArgs } from "node:util";
import { uuidV7 } from "./lib/landfall.mjs";
import { sleep } from "./lib/solana.mjs";

const { values } = parseArgs({
  options: { count: { type: "string", default: "10" }, interval: { type: "string", default: "2" } },
});
const required = (name) => {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`${name} is required`);
  return value;
};
const collectorUrl = process.env.LANDFALL_COLLECTOR_URL ?? "http://127.0.0.1:8080";
const token = required("LANDFALL_X402_TOKEN");
const policyId = process.env.LANDFALL_X402_POLICY_ID ?? "0198ef00-0000-7000-8000-000000000900";

// Matches the policy seeded by scripts/seed-demo-control-plane.sh.
const merchants = [
  "https://api.demo-merchant.invalid",
  "https://api.demo-merchant.invalid",
  "https://reports.demo-merchant.invalid",
  "https://unlisted-merchant.invalid",
];
const amountsAtomic = ["250000", "500000", "750000", "1200000", "1800000", "3500000"];
const pick = (items) => items[Math.floor(Math.random() * items.length)];

for (let index = 0; index < Number(values.count); index += 1) {
  const request = {
    policy_id: policyId,
    agent_id: "landfall-demo-agent",
    merchant_origin: pick(merchants),
    network: "solana-devnet",
    asset: "USDC",
    amount_atomic: pick(amountsAtomic),
    idempotency_key: uuidV7(),
  };
  const response = await fetch(new URL("/v1/x402/authorize", collectorUrl), {
    method: "POST",
    headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
    body: JSON.stringify(request),
  });
  const body = await response.json().catch(() => ({}));
  console.log(
    `${request.merchant_origin} ${Number(request.amount_atomic) / 1e6} USDC -> HTTP ${response.status} ${body.decision ?? body.code ?? ""} ${body.reason_code ?? ""}`,
  );
  await sleep(Number(values.interval) * 1_000);
}
