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

const rpcEndpoint = process.env.SOLANA_RPC_URL ?? (cluster === "devnet" ? "https://api.devnet.solana.com" : "http://127.0.0.1:8899");
const signedTransaction = process.env.SIGNED_TRANSACTION_BASE64;
async function rpc(method, params) {
  const response = await fetch(rpcEndpoint, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }) });
  if (!response.ok) throw new Error(`Solana RPC returned HTTP ${response.status}`);
  const body = await response.json();
  if (body.error) throw new Error(`Solana RPC ${body.error.code}: ${body.error.message}`);
  return body.result;
}
const client = {
  getLatestBlockhash: async () => { const result = await rpc("getLatestBlockhash", [{ commitment: "confirmed" }]); return { blockhash: result.value.blockhash, lastValidBlockHeight: BigInt(result.value.lastValidBlockHeight) }; },
  simulate: async () => { if (!signedTransaction) throw new Error("SIGNED_TRANSACTION_BASE64 is required for simulation"); return (await rpc("simulateTransaction", [signedTransaction, { encoding: "base64", replaceRecentBlockhash: true, sigVerify: false }])).value; },
  sign: async () => { if (!signedTransaction) throw new Error("SIGNED_TRANSACTION_BASE64 is required (the example never handles private keys)"); return signedTransaction; },
  submit: async (signed) => { if (!allowSubmission) throw new Error("submission is disabled; set ALLOW_SUBMISSION=true explicitly"); return await rpc("sendTransaction", [signed, { encoding: "base64", skipPreflight: false }]); },
  confirm: async (signature) => { const result = await rpc("getSignatureStatuses", [[signature], { searchTransactionHistory: true }]); const status = result.value[0]; if (!status) throw new Error("signature not observed yet"); if (status.err) throw new Error(`transaction execution failed: ${JSON.stringify(status.err)}`); return status; },
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
