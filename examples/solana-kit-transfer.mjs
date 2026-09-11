import {
  captureLatestBlockhash,
  measureConfirmationWait,
  measureSigning,
  normalizeSimulationResult,
  submitWithRoute,
} from "../packages/sdk-ts/dist/index.js";

const cluster = process.env.SOLANA_CLUSTER ?? "local";
const allowSubmission = process.env.ALLOW_SUBMISSION === "true";
const lamports = 1_000;
if (!new Set(["local", "devnet"]).has(cluster)) throw new Error("SOLANA_CLUSTER must be local or devnet");

// This harness deliberately uses an injected fake client. Replace these methods
// with the @solana/kit 8.2.0 adapter only after an operator opts in to sending.
const client = {
  getLatestBlockhash: async () => ({ blockhash: "demo-blockhash", lastValidBlockHeight: 500n }),
  simulate: async () => ({ err: null, unitsConsumed: 500n, logs: ["system transfer"] }),
  sign: async () => ({ signed: true }),
  submit: async () => "demo-signature",
  confirm: async () => ({ commitment: "confirmed" }),
};
const clock = (() => { let tick = 0n; return () => (tick += 10n); })();
const blockhash = await captureLatestBlockhash(client);
const simulation = normalizeSimulationResult(await client.simulate({ instruction: "system.transfer", lamports }));
const signing = await measureSigning(() => client.sign({ instruction: "system.transfer", lamports }), clock);
const submission = allowSubmission
  ? await submitWithRoute({ attemptId: "demo-attempt-1", route: { routeId: `${cluster}-rpc` }, attemptSequence: 1, encoding: "base64", skipPreflight: false }, () => client.submit(signing.value, { routeId: `${cluster}-rpc` }))
  : { result: "dry_run", value: undefined };
const confirmation = allowSubmission
  ? await measureConfirmationWait(() => client.confirm(submission.value, blockhash), clock)
  : { result: "not_started" };

console.log(JSON.stringify({ cluster, lamports, allowSubmission, blockhash, simulation, signing: { result: signing.result, durationNs: signing.durationNs }, submission: { result: submission.result }, confirmation: { result: confirmation.result } }, null, 2));
