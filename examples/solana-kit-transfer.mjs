import {
  LandfallSdk,
  captureLatestBlockhash,
  createSolanaLifecycleRecorder,
  generateUuidV7,
  measureConfirmationWait,
  measureSigning,
  normalizeSimulationResult,
  submitWithRoute,
} from "../packages/sdk-ts/dist/index.js";

const cluster = process.env.SOLANA_CLUSTER ?? "local";
const allowSubmission = process.env.ALLOW_SUBMISSION === "true";
const lamports = 1_000;
if (!new Set(["local", "devnet"]).has(cluster))
  throw new Error("SOLANA_CLUSTER must be local or devnet");

const rpcEndpoint =
  process.env.SOLANA_RPC_URL ??
  (cluster === "devnet" ? "https://api.devnet.solana.com" : "http://127.0.0.1:8899");
const signedTransaction = process.env.SIGNED_TRANSACTION_BASE64;
const landfall = {
  collectorUrl: process.env.LANDFALL_COLLECTOR_URL,
  ingestToken: process.env.LANDFALL_INGEST_TOKEN,
  projectId: process.env.LANDFALL_PROJECT_ID,
  environmentId: process.env.LANDFALL_ENVIRONMENT_ID,
  routeId: process.env.LANDFALL_ROUTE_ID,
};
for (const [name, value] of Object.entries(landfall)) {
  if (!value) throw new Error(`${name} is required to send lifecycle telemetry`);
}
const sdk = new LandfallSdk({
  collectorUrl: landfall.collectorUrl,
  ingestToken: landfall.ingestToken,
});
const traceId = generateUuidV7();
const recorder = createSolanaLifecycleRecorder(sdk, {
  projectId: landfall.projectId,
  environmentId: landfall.environmentId,
  traceId,
  source: { kind: "application", name: "solana-kit-transfer", version: "1.0.0" },
});
const elapsed = async (operation) => {
  const started = process.hrtime.bigint();
  try {
    return { value: await operation(), durationNs: (process.hrtime.bigint() - started).toString() };
  } catch (error) {
    return { error, durationNs: (process.hrtime.bigint() - started).toString() };
  }
};
async function rpc(method, params) {
  const response = await fetch(rpcEndpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  });
  if (!response.ok) throw new Error(`Solana RPC returned HTTP ${response.status}`);
  const body = await response.json();
  if (body.error) throw new Error(`Solana RPC ${body.error.code}: ${body.error.message}`);
  return body.result;
}
const client = {
  getLatestBlockhash: async () => {
    const result = await rpc("getLatestBlockhash", [{ commitment: "confirmed" }]);
    return {
      blockhash: result.value.blockhash,
      lastValidBlockHeight: BigInt(result.value.lastValidBlockHeight),
    };
  },
  simulate: async () => {
    if (!signedTransaction) throw new Error("SIGNED_TRANSACTION_BASE64 is required for simulation");
    return (
      await rpc("simulateTransaction", [
        signedTransaction,
        { encoding: "base64", replaceRecentBlockhash: true, sigVerify: false },
      ])
    ).value;
  },
  sign: async () => {
    if (!signedTransaction)
      throw new Error(
        "SIGNED_TRANSACTION_BASE64 is required (the example never handles private keys)",
      );
    return signedTransaction;
  },
  submit: async (signed) => {
    if (!allowSubmission)
      throw new Error("submission is disabled; set ALLOW_SUBMISSION=true explicitly");
    return await rpc("sendTransaction", [signed, { encoding: "base64", skipPreflight: false }]);
  },
  confirm: async (signature) => {
    const result = await rpc("getSignatureStatuses", [
      [signature],
      { searchTransactionHistory: true },
    ]);
    const status = result.value[0];
    if (!status) throw new Error("signature not observed yet");
    if (status.err) throw new Error(`transaction execution failed: ${JSON.stringify(status.err)}`);
    return status;
  },
};
const clock = (() => {
  let tick = 0n;
  return () => (tick += 10n);
})();
recorder.traceCreated({ flow: "sdk-instrumented-transfer", transaction_version: "legacy" });
const blockhashMeasurement = await elapsed(() => captureLatestBlockhash(client));
if (blockhashMeasurement.error) throw blockhashMeasurement.error;
const blockhash = blockhashMeasurement.value;
recorder.blockhashAcquired({
  route_id: landfall.routeId,
  result: "acquired",
  duration_ns: blockhashMeasurement.durationNs,
  recent_blockhash: blockhash.blockhash,
  last_valid_block_height: blockhash.lastValidBlockHeight,
});
const simulationId = generateUuidV7();
recorder.simulationStarted({
  simulation_id: simulationId,
  route_id: landfall.routeId,
  commitment: "confirmed",
  replace_recent_blockhash: true,
  sig_verify: false,
});
const simulationMeasurement = await elapsed(() =>
  client.simulate({ instruction: "system.transfer", lamports }),
);
const simulation = normalizeSimulationResult(
  simulationMeasurement.error ? { err: simulationMeasurement.error } : simulationMeasurement.value,
);
recorder.simulationCompleted({
  simulation_id: simulationId,
  route_id: landfall.routeId,
  duration_ns: simulationMeasurement.durationNs,
  transport_result: simulationMeasurement.error ? "connection_failed" : "response_received",
  rpc_result: simulation.rpcResult,
  ...(simulation.unitsConsumed === undefined ? {} : { units_consumed: simulation.unitsConsumed }),
  ...(simulation.logsPresent === undefined ? {} : { logs_present: simulation.logsPresent }),
});
if (simulationMeasurement.error) throw simulationMeasurement.error;
const signing = await measureSigning(
  () => client.sign({ instruction: "system.transfer", lamports }),
  clock,
);
const attemptId = generateUuidV7();
const submission = allowSubmission
  ? (recorder.submissionStarted({
      attempt_id: attemptId,
      route_id: landfall.routeId,
      attempt_sequence: 1,
      encoding: "base64",
      skip_preflight: false,
    }),
    await submitWithRoute(
      {
        attemptId,
        route: { routeId: landfall.routeId },
        attemptSequence: 1,
        encoding: "base64",
        skipPreflight: false,
      },
      () => client.submit(signing.value, { routeId: landfall.routeId }),
    ))
  : { result: "dry_run", value: undefined };
if (allowSubmission) {
  recorder.submissionCompleted({
    attempt_id: attemptId,
    route_id: landfall.routeId,
    duration_ns: "0",
    transport_result: "response_received",
    rpc_result: submission.result === "accepted" ? "accepted" : "rejected",
    ...(submission.value === undefined ? {} : { signature: submission.value }),
  });
}
const confirmation = allowSubmission
  ? await measureConfirmationWait(() => client.confirm(submission.value, blockhash), clock)
  : { result: "not_started" };
if (allowSubmission && submission.value !== undefined) {
  recorder.statusObserved({
    observer_source_id: generateUuidV7(),
    source_result: confirmation.result === "commitment_reached" ? "found" : "not_found",
    duration_ns: confirmation.durationNs ?? "0",
    signature: submission.value,
    commitment: "confirmed",
  });
}
const delivery = await sdk.flush();
if (delivery.status !== "flushed")
  throw delivery.error ?? new Error(`telemetry flush ${delivery.status}`);

console.log(
  JSON.stringify(
    {
      cluster,
      lamports,
      allowSubmission,
      blockhash,
      simulation,
      signing: { result: signing.result, durationNs: signing.durationNs },
      submission: { result: submission.result },
      confirmation: { result: confirmation.result },
      traceId,
    },
    null,
    2,
  ),
);
