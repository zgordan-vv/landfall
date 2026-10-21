#!/usr/bin/env node
/**
 * Landfall demo traffic generator.
 *
 * Sends real Solana transactions on devnet or a local test validator and
 * reports their genuine lifecycle to a Landfall collector. Refuses mainnet.
 *
 *   node scripts/demo-traffic/generate.mjs --once healthy
 *   node scripts/demo-traffic/generate.mjs --all
 *   node scripts/demo-traffic/generate.mjs --loop --interval 30
 *
 * See scripts/demo-traffic/README.md for configuration.
 */
import { createHash, randomBytes } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { parseArgs } from "node:util";
import { createIngestClient, uuidV7 } from "./lib/landfall.mjs";
import { SCENARIOS, runScenario } from "./lib/scenarios.mjs";
import { base58Encode, buildSignedTransfer, createRpc, loadKeypair, sleep } from "./lib/solana.mjs";

const MAINNET_GENESIS_PREFIX = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp";

/** Baseline mixes in the failure modes; the candidate environment models a tuned configuration. */
const MIXES = {
  baseline: {
    healthy: 36,
    "simulation-error": 6,
    "rpc-rejection": 6,
    "execution-error": 6,
    "compute-exhaustion": 6,
    "tight-compute": 6,
    "stale-blockhash": 8,
    "slow-signer": 8,
    "client-timeout": 8,
    "flaky-route-retry": 12,
  },
  candidate: {
    healthy: 72,
    "simulation-error": 4,
    "execution-error": 2,
    "compute-exhaustion": 2,
    "tight-compute": 2,
    "stale-blockhash": 2,
    "client-timeout": 10,
    "flaky-route-retry": 8,
  },
};

const { values } = parseArgs({
  options: {
    once: { type: "string" },
    all: { type: "boolean", default: false },
    loop: { type: "boolean", default: false },
    count: { type: "string" },
    interval: { type: "string", default: "30" },
    concurrency: { type: "string", default: "3" },
    environment: { type: "string" },
  },
});

const env = process.env;
const required = (name) => {
  const value = env[name]?.trim();
  if (!value) throw new Error(`${name} is required (see scripts/demo-traffic/README.md)`);
  return value;
};

const config = {
  collectorUrl: env.LANDFALL_COLLECTOR_URL ?? "http://127.0.0.1:8080",
  ingestToken: env.LANDFALL_INGEST_TOKEN,
  projectId: required("LANDFALL_PROJECT_ID"),
  baselineEnvironmentId: required("LANDFALL_ENVIRONMENT_ID"),
  candidateEnvironmentId: env.LANDFALL_CANDIDATE_ENVIRONMENT_ID,
  rpcUrl: env.SOLANA_RPC_URL ?? "http://127.0.0.1:8899",
  flakyRpcUrl: env.DEMO_FLAKY_RPC_URL ?? "http://10.255.255.1:8899",
  walletPath: env.DEMO_WALLET ?? join(env.HOME ?? ".", ".config/solana/landfall-demo.json"),
  stateDir: env.DEMO_STATE_DIR ?? join(env.HOME ?? ".", ".landfall-demo-traffic"),
  primaryRouteId: env.LANDFALL_ROUTE_ID ?? "0198ef00-0000-7000-8000-000000000400",
  // Registered routes belong to one environment, so the candidate may need its own.
  candidateRouteId: env.LANDFALL_CANDIDATE_ROUTE_ID,
  flakyRouteId: env.LANDFALL_FLAKY_ROUTE_ID ?? "0198ef00-0000-7000-8000-000000000401",
};

const log = (message, detail) =>
  console.log(
    `${new Date().toISOString()} ${message}${detail ? ` ${JSON.stringify(detail)}` : ""}`,
  );

function loadState() {
  const path = join(config.stateDir, "state.json");
  if (existsSync(path)) return JSON.parse(readFileSync(path, "utf8"));
  // Per-installation fingerprint HMAC key and observer identity; not secret material for any wallet.
  const state = {
    fingerprintKeyId: uuidV7(),
    fingerprintKeyHex: randomBytes(32).toString("hex"),
    observerSourceId: uuidV7(),
  };
  mkdirSync(dirname(path), { recursive: true, mode: 0o700 });
  writeFileSync(path, JSON.stringify(state, null, 2), { mode: 0o600 });
  return state;
}

async function main() {
  const rpc = createRpc(config.rpcUrl);
  const genesis = await rpc("getGenesisHash");
  if (genesis.startsWith(MAINNET_GENESIS_PREFIX)) {
    throw new Error("refusing to run against mainnet-beta; use devnet or a local validator");
  }
  const payer = loadKeypair(config.walletPath);
  const recipient =
    env.DEMO_RECIPIENT ??
    base58Encode(createHash("sha256").update("landfall-demo-recipient-v1").digest());
  const state = loadState();
  const ctx = {
    rpc,
    payer,
    recipient,
    projectId: config.projectId,
    post: createIngestClient(config),
    observerSourceId: state.observerSourceId,
    fingerprintKeyId: state.fingerprintKeyId,
    fingerprintKey: Buffer.from(state.fingerprintKeyHex, "hex"),
    routes: {
      primary: { id: config.primaryRouteId, rpc },
      flaky: { id: config.flakyRouteId, rpc: createRpc(config.flakyRpcUrl, { timeoutMs: 1_500 }) },
    },
    streaming: env.DEMO_STREAM_EVENTS === "1",
    log,
  };

  const balance = (await rpc("getBalance", [payer.address, { commitment: "confirmed" }])).value;
  log(`wallet ${payer.address} balance ${balance / 1e9} SOL on cluster ${genesis.slice(0, 8)}…`);
  if (balance < 10_000_000) {
    throw new Error(
      `wallet balance too low; airdrop test SOL first: solana airdrop 2 ${payer.address} --url ${config.rpcUrl}`,
    );
  }
  await ensureRecipientFunded(rpc, payer, recipient);

  const environments = [
    { id: config.baselineEnvironmentId, routeId: config.primaryRouteId, mix: MIXES.baseline },
    ...(config.candidateEnvironmentId
      ? [
          {
            id: config.candidateEnvironmentId,
            routeId: config.candidateRouteId ?? config.primaryRouteId,
            mix: MIXES.candidate,
          },
        ]
      : []),
  ];
  const run = (name, environment) =>
    runScenario(
      name,
      {
        ...ctx,
        routes: { ...ctx.routes, primary: { ...ctx.routes.primary, id: environment.routeId } },
      },
      environment.id,
    );
  const pickEnvironment = () =>
    values.environment === "candidate" && environments[1]
      ? environments[1]
      : values.environment === "baseline"
        ? environments[0]
        : environments[Math.floor(Math.random() * environments.length)];

  if (values.once) {
    if (!SCENARIOS[values.once])
      throw new Error(`unknown scenario; choose one of ${Object.keys(SCENARIOS).join(", ")}`);
    await run(values.once, pickEnvironment());
    return;
  }
  if (values.all) {
    // One of each scenario, concurrently, so every diagnostic appears quickly.
    await Promise.allSettled(
      Object.keys(SCENARIOS).map((name, index) =>
        sleep(index * 1_500)
          .then(() => run(name, environments[0]))
          .catch((error) => log(`${name} failed: ${error.message}`)),
      ),
    );
    return;
  }
  if (!values.loop) throw new Error("choose --once <scenario>, --all, or --loop");

  const intervalMs = Number(values.interval) * 1_000;
  const concurrency = Number(values.concurrency);
  const limit = values.count ? Number(values.count) : Infinity;
  let running = 0;
  let started = 0;
  let stopping = false;
  process.on("SIGINT", () => (stopping = true));
  process.on("SIGTERM", () => (stopping = true));
  while (!stopping && started < limit) {
    if (running < concurrency) {
      const environment = pickEnvironment();
      const name = weightedPick(environment.mix);
      running += 1;
      started += 1;
      run(name, environment)
        .catch((error) => log(`${name} failed: ${error.message}`))
        .finally(() => (running -= 1));
    }
    await sleep(intervalMs * (0.5 + Math.random()));
  }
  while (running > 0) await sleep(1_000);
}

function weightedPick(mix) {
  const entries = Object.entries(mix);
  let roll = Math.random() * entries.reduce((sum, [, weight]) => sum + weight, 0);
  for (const [name, weight] of entries) {
    roll -= weight;
    if (roll <= 0) return name;
  }
  return entries[0][0];
}

/** Funds the fixed recipient above rent exemption once, so 1,000-lamport transfers succeed. */
async function ensureRecipientFunded(rpc, payer, recipient) {
  const balance = (await rpc("getBalance", [recipient, { commitment: "confirmed" }])).value;
  if (balance >= 1_000_000) return;
  const { value } = await rpc("getLatestBlockhash", [{ commitment: "confirmed" }]);
  const tx = buildSignedTransfer({
    payer,
    recipient,
    lamports: 1_000_000,
    recentBlockhash: value.blockhash,
  });
  await rpc("sendTransaction", [
    tx.base64,
    { encoding: "base64", preflightCommitment: "confirmed" },
  ]);
  for (let attempt = 0; attempt < 30; attempt += 1) {
    const status = (await rpc("getSignatureStatuses", [[tx.signature]])).value[0];
    if (status?.confirmationStatus === "confirmed" || status?.confirmationStatus === "finalized")
      return;
    await sleep(1_000);
  }
  throw new Error("recipient funding transfer did not confirm");
}

main().catch((error) => {
  console.error(error.message);
  process.exit(1);
});
