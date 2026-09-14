/**
 * A real devnet transaction wrapped with Landfall telemetry.
 * The Solana CLI owns the configured keypair; Landfall receives neither the key nor signed bytes.
 */
import { spawnSync } from "node:child_process";
import {
  LandfallSdk,
  createSolanaLifecycleRecorder,
  generateUuidV7,
} from "../packages/sdk-ts/dist/index.js";

const required = [
  "LANDFALL_COLLECTOR_URL",
  "LANDFALL_INGEST_TOKEN",
  "LANDFALL_PROJECT_ID",
  "LANDFALL_ENVIRONMENT_ID",
  "LANDFALL_ROUTE_ID",
  "SOLANA_RPC_URL",
];
for (const name of required) {
  if (!process.env[name]) throw new Error(`${name} is required`);
}
const amount = process.env.SOLANA_AMOUNT_SOL ?? "0.000001";
const recipient = process.env.SOLANA_RECIPIENT ?? run("solana", ["address"]);
const sdk = new LandfallSdk({
  collectorUrl: process.env.LANDFALL_COLLECTOR_URL,
  ingestToken: process.env.LANDFALL_INGEST_TOKEN,
});
const traceId = generateUuidV7();
const recorder = createSolanaLifecycleRecorder(sdk, {
  projectId: process.env.LANDFALL_PROJECT_ID,
  environmentId: process.env.LANDFALL_ENVIRONMENT_ID,
  traceId,
  source: { kind: "application", name: "solana-cli-sdk-transfer", version: "1.0.0" },
});
const routeId = process.env.LANDFALL_ROUTE_ID;
const startedAt = process.hrtime.bigint();
recorder.traceCreated({ flow: "devnet-cli-transfer", transaction_version: "legacy" });
const attemptId = generateUuidV7();
recorder.submissionStarted({
  attempt_id: attemptId,
  route_id: routeId,
  attempt_sequence: 1,
  encoding: "base64",
  skip_preflight: false,
});

let signature;
try {
  const output = run("solana", [
    "transfer",
    recipient,
    amount,
    "--allow-unfunded-recipient",
    "--url",
    process.env.SOLANA_RPC_URL,
  ]);
  signature = output.match(/Signature:\s*([1-9A-HJ-NP-Za-km-z]+)/)?.[1];
  if (!signature) throw new Error("Solana CLI returned no transaction signature");
  recorder.submissionCompleted({
    attempt_id: attemptId,
    route_id: routeId,
    duration_ns: (process.hrtime.bigint() - startedAt).toString(),
    transport_result: "response_received",
    rpc_result: "accepted",
    signature,
  });
} catch (error) {
  recorder.submissionCompleted({
    attempt_id: attemptId,
    route_id: routeId,
    duration_ns: (process.hrtime.bigint() - startedAt).toString(),
    transport_result: "connection_failed",
    rpc_result: "rejected",
  });
  await flushOrThrow(sdk);
  throw error;
}

const statusStartedAt = process.hrtime.bigint();
const status = await waitForStatus(signature);
recorder.statusObserved({
  observer_source_id: generateUuidV7(),
  source_result: "found",
  duration_ns: (process.hrtime.bigint() - statusStartedAt).toString(),
  signature,
  commitment: status.confirmationStatus ?? "confirmed",
  slot: String(status.slot),
  ...(status.confirmations === null || status.confirmations === undefined
    ? {}
    : { confirmations: String(status.confirmations) }),
});
await flushOrThrow(sdk);
console.log(
  JSON.stringify({ traceId, signature, recipient, amount, status: status.confirmationStatus }),
);

function run(command, args) {
  const result = spawnSync(command, args, { encoding: "utf8" });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(result.stderr.trim() || `${command} failed`);
  return result.stdout.trim();
}

async function rpc(method, params) {
  const response = await fetch(process.env.SOLANA_RPC_URL, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  });
  if (!response.ok) throw new Error(`Solana RPC returned HTTP ${response.status}`);
  const body = await response.json();
  if (body.error) throw new Error(`Solana RPC ${body.error.code}: ${body.error.message}`);
  return body.result;
}

async function waitForStatus(signature) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    const result = await rpc("getSignatureStatuses", [
      [signature],
      { searchTransactionHistory: true },
    ]);
    const status = result.value[0];
    if (status?.confirmationStatus === "confirmed" || status?.confirmationStatus === "finalized") {
      if (status.err)
        throw new Error(`Transaction execution failed: ${JSON.stringify(status.err)}`);
      return status;
    }
    await new Promise((resolve) => setTimeout(resolve, 1_000));
  }
  throw new Error("Transaction was accepted but not confirmed within 20 seconds");
}

async function flushOrThrow(sdk) {
  const result = await sdk.flush();
  if (result.status !== "flushed")
    throw result.error ?? new Error(`Telemetry flush ${result.status}`);
}
