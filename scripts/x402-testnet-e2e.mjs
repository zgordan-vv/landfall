#!/usr/bin/env node
/** Execute a real x402 testnet payment without ever loading a wallet key into Landfall. */

import { pathToFileURL } from "node:url";
import { resolve } from "node:path";

import {
  createHttpX402Authorizer,
  createHttpX402ResourceClient,
  createHttpX402SettlementRecorder,
  executeAuthorizedX402Payment,
  parsePaymentRequiredHeader,
  preAuthorizeX402Requirement,
} from "../packages/sdk-ts/dist/index.js";

const required = [
  "LANDFALL_POLICY_URL",
  "LANDFALL_X402_TOKEN",
  "LANDFALL_X402_POLICY_ID",
  "LANDFALL_X402_AGENT_ID",
  "X402_RESOURCE_URL",
  "X402_SIGNER_MODULE",
];

if (process.argv.includes("--help")) {
  process.stdout.write(
    `Runs one real x402 testnet payment. Required variables:\n${required.map((name) => `  ${name}`).join("\n")}\n`,
  );
  process.exit(0);
}

const missing = required.filter((name) => !process.env[name]);
if (missing.length > 0) {
  throw new Error(`x402 testnet e2e needs: ${missing.join(", ")}`);
}

const policyUrl = process.env.LANDFALL_POLICY_URL;
const token = process.env.LANDFALL_X402_TOKEN;
const policyId = process.env.LANDFALL_X402_POLICY_ID;
const agentId = process.env.LANDFALL_X402_AGENT_ID;
const resourceUrl = process.env.X402_RESOURCE_URL;
const signerModule = process.env.X402_SIGNER_MODULE;
const idempotencyKey = process.env.X402_IDEMPOTENCY_KEY ?? `testnet-${crypto.randomUUID()}`;

const initial = await fetch(resourceUrl, { headers: { accept: "application/json" } });
if (initial.status !== 402) {
  throw new Error(`merchant must first return HTTP 402, received ${initial.status}`);
}
const paymentRequiredHeader = initial.headers.get("PAYMENT-REQUIRED");
if (paymentRequiredHeader === null) {
  throw new Error("merchant 402 response did not contain PAYMENT-REQUIRED");
}
const challenge = parsePaymentRequiredHeader(paymentRequiredHeader);
const requirement = selectRequirement(
  challenge.accepts,
  process.env.X402_NETWORK,
  process.env.X402_ASSET,
);
const authorization = preAuthorizeX402Requirement(
  challenge,
  requirement,
  policyId,
  agentId,
  idempotencyKey,
);
const signer = await loadSigner(signerModule, { challenge, requirement, resourceUrl });
const authorizer = createHttpX402Authorizer(policyUrl, token, policyFetch);
const settlementRecorder = createHttpX402SettlementRecorder(policyUrl, token, policyFetch);
const resourceClient = createHttpX402ResourceClient({ url: resourceUrl }, resourceFetch);
const outcome = await executeAuthorizedX402Payment(
  authorization,
  requirement,
  authorizer,
  signer,
  resourceClient,
  settlementRecorder,
);

process.stdout.write(
  `${JSON.stringify({ kind: outcome.kind, audit_id: outcome.authorization.auditId, decision: outcome.authorization.decision, reason_code: outcome.authorization.reasonCode })}\n`,
);

function selectRequirement(requirements, network, asset) {
  const selected = requirements.find(
    (candidate) =>
      (network === undefined || candidate.network === network) &&
      (asset === undefined || candidate.asset === asset),
  );
  if (selected === undefined)
    throw new Error("merchant offered no requirement matching X402_NETWORK/X402_ASSET");
  return selected;
}

async function loadSigner(modulePath, context) {
  const loaded = await import(pathToFileURL(resolve(modulePath)).href);
  if (typeof loaded.createX402PaymentSigner !== "function") {
    throw new Error("X402_SIGNER_MODULE must export createX402PaymentSigner(context)");
  }
  const signer = await loaded.createX402PaymentSigner(context);
  if (
    signer === null ||
    typeof signer !== "object" ||
    typeof signer.createPaymentSignature !== "function"
  ) {
    throw new Error("createX402PaymentSigner must return an X402PaymentSigner");
  }
  return signer;
}

async function policyFetch(input, init) {
  const response = await fetch(input, init);
  return { status: response.status, json: () => response.json() };
}

async function resourceFetch(input, init) {
  const response = await fetch(input, init);
  return { ok: response.ok, status: response.status };
}
