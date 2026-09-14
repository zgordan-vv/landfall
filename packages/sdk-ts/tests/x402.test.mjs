import test from "node:test";
import assert from "node:assert/strict";
import {
  createHttpX402Authorizer,
  createHttpX402ResourceClient,
  createHttpX402SettlementRecorder,
  createOfficialSvmExactSigner,
  executeAuthorizedX402Payment,
  parsePaymentRequiredHeader,
  preAuthorizeX402Requirement,
} from "../dist/index.js";

const policyId = "0198ef00-0000-7000-8000-000000000401";
const encode = (value) => Buffer.from(JSON.stringify(value)).toString("base64url");
const challenge = () =>
  encode({
    x402Version: 2,
    resource: { url: "https://API.example.com/v1/data?private=no" },
    accepts: [
      {
        scheme: "exact",
        network: "solana:mainnet",
        amount: "1000000",
        asset: "USDC",
        payTo: "merchant",
        maxTimeoutSeconds: 60,
      },
    ],
  });

test("parses a v2 PAYMENT-REQUIRED challenge and keeps only the merchant origin", () => {
  const parsed = parsePaymentRequiredHeader(challenge());
  const request = preAuthorizeX402Requirement(
    parsed,
    parsed.accepts[0],
    policyId,
    "agent",
    "request-1",
  );
  assert.equal(request.merchantOrigin, "https://api.example.com");
  assert.equal(request.amountAtomic, "1000000");
});

test("rejects malformed challenges before any wallet operation", () => {
  assert.throws(() => parsePaymentRequiredHeader("not-base64"));
  assert.throws(() =>
    parsePaymentRequiredHeader(encode({ x402Version: 1, resource: {}, accepts: [] })),
  );
  assert.throws(() =>
    parsePaymentRequiredHeader(
      encode({
        x402Version: 2,
        resource: { url: "https://api.example.com" },
        accepts: [
          {
            scheme: "exact",
            network: "solana:mainnet",
            amount: "01",
            asset: "USDC",
            payTo: "merchant",
            maxTimeoutSeconds: 60,
          },
        ],
      }),
    ),
  );
});

test("sends only normalized pre-authorization data to Landfall before signing", async () => {
  const parsed = parsePaymentRequiredHeader(challenge());
  const request = preAuthorizeX402Requirement(
    parsed,
    parsed.accepts[0],
    policyId,
    "agent",
    "request-1",
  );
  let call;
  const authorizer = createHttpX402Authorizer(
    "https://policy.example",
    "token",
    async (url, init) => {
      call = { url, init };
      return {
        status: 200,
        json: async () => ({
          audit_id: "0198ef00-0000-7000-8000-000000000402",
          decision: "approved",
          reason_code: "approved",
          replayed: false,
        }),
      };
    },
  );
  assert.deepEqual(await authorizer.authorize(request), {
    auditId: "0198ef00-0000-7000-8000-000000000402",
    decision: "approved",
    reasonCode: "approved",
    replayed: false,
  });
  assert.equal(call.url, "https://policy.example/v1/x402/authorize");
  assert.equal(JSON.parse(call.init.body).merchant_origin, "https://api.example.com");
});

test("executes payment only after approval and never sends signature to Landfall", async () => {
  const parsed = parsePaymentRequiredHeader(challenge());
  const authorization = preAuthorizeX402Requirement(
    parsed,
    parsed.accepts[0],
    policyId,
    "agent",
    "request-2",
  );
  const seen = { signer: 0, resourceSignature: "", settlement: undefined };
  const outcome = await executeAuthorizedX402Payment(
    authorization,
    parsed.accepts[0],
    {
      authorize: async () => ({
        auditId: "0198ef00-0000-7000-8000-000000000402",
        decision: "approved",
        reasonCode: "approved",
        replayed: false,
      }),
    },
    {
      createPaymentSignature: async () => {
        seen.signer += 1;
        return {
          paymentSignature: "signed-payment-payload",
          settlementReference: "facilitator-receipt-1",
        };
      },
    },
    {
      sendPaymentSignature: async (signature) => {
        seen.resourceSignature = signature;
        return { ok: true, status: 200, response: { status: 200 } };
      },
    },
    {
      recordSettlement: async (record) => {
        seen.settlement = record;
      },
    },
  );
  assert.equal(outcome.kind, "settled");
  assert.equal(seen.signer, 1);
  assert.equal(seen.resourceSignature, "signed-payment-payload");
  assert.deepEqual(seen.settlement, {
    auditId: "0198ef00-0000-7000-8000-000000000402",
    outcome: "settled",
    reasonCode: "resource_response_received",
    settlementReference: "facilitator-receipt-1",
  });
  assert.notEqual(JSON.stringify(seen.settlement).includes("signed-payment-payload"), true);
});

test("a denied policy never invokes wallet or merchant", async () => {
  const parsed = parsePaymentRequiredHeader(challenge());
  const authorization = preAuthorizeX402Requirement(
    parsed,
    parsed.accepts[0],
    policyId,
    "agent",
    "request-3",
  );
  let walletUsed = false;
  const outcome = await executeAuthorizedX402Payment(
    authorization,
    parsed.accepts[0],
    {
      authorize: async () => ({
        auditId: "0198ef00-0000-7000-8000-000000000403",
        decision: "denied",
        reasonCode: "daily_limit_exceeded",
        replayed: false,
      }),
    },
    {
      createPaymentSignature: async () => {
        walletUsed = true;
        return { paymentSignature: "must-not-happen" };
      },
    },
    {
      sendPaymentSignature: async () => {
        throw new Error("must not call merchant");
      },
    },
    {
      recordSettlement: async () => {
        throw new Error("must not record");
      },
    },
  );
  assert.equal(outcome.kind, "denied");
  assert.equal(walletUsed, false);
});

test("a settled idempotent replay never invokes wallet or merchant again", async () => {
  const parsed = parsePaymentRequiredHeader(challenge());
  const authorization = preAuthorizeX402Requirement(
    parsed,
    parsed.accepts[0],
    policyId,
    "agent",
    "request-4",
  );
  let called = false;
  const outcome = await executeAuthorizedX402Payment(
    authorization,
    parsed.accepts[0],
    {
      authorize: async () => ({
        auditId: "0198ef00-0000-7000-8000-000000000404",
        decision: "settled",
        reasonCode: "resource_response_received",
        replayed: true,
      }),
    },
    {
      createPaymentSignature: async () => {
        called = true;
        return { paymentSignature: "must-not-happen" };
      },
    },
    {
      sendPaymentSignature: async () => {
        called = true;
        return { ok: true, status: 200, response: {} };
      },
    },
    {
      recordSettlement: async () => {
        called = true;
      },
    },
  );
  assert.equal(outcome.kind, "already-settled");
  assert.equal(called, false);
});

test("a non-success merchant response is recorded as failed", async () => {
  const parsed = parsePaymentRequiredHeader(challenge());
  const authorization = preAuthorizeX402Requirement(
    parsed,
    parsed.accepts[0],
    policyId,
    "agent",
    "request-5",
  );
  let settlement;
  await assert.rejects(() =>
    executeAuthorizedX402Payment(
      authorization,
      parsed.accepts[0],
      {
        authorize: async () => ({
          auditId: "0198ef00-0000-7000-8000-000000000405",
          decision: "approved",
          reasonCode: "approved",
          replayed: false,
        }),
      },
      {
        createPaymentSignature: async () => ({
          paymentSignature: "signed-payment-payload",
          settlementReference: "receipt-5",
        }),
      },
      { sendPaymentSignature: async () => ({ ok: false, status: 402, response: {} }) },
      {
        recordSettlement: async (input) => {
          settlement = input;
        },
      },
    ),
  );
  assert.deepEqual(settlement, {
    auditId: "0198ef00-0000-7000-8000-000000000405",
    outcome: "failed",
    reasonCode: "external_payment_failed",
    settlementReference: "receipt-5",
  });
});

test("settlement reporter persists only outcome metadata", async () => {
  let call;
  const recorder = createHttpX402SettlementRecorder(
    "https://policy.example",
    "token",
    async (url, init) => {
      call = { url, init };
      return { status: 200, json: async () => ({}) };
    },
  );
  await recorder.recordSettlement({
    auditId: "0198ef00-0000-7000-8000-000000000402",
    outcome: "settled",
    reasonCode: "resource_response_received",
    settlementReference: "receipt",
  });
  assert.equal(call.url, "https://policy.example/v1/x402/settlements");
  assert.equal(JSON.stringify(call.init.body).includes("paymentSignature"), false);
});

test("HTTP resource adapter sends PAYMENT-SIGNATURE only to the merchant", async () => {
  let call;
  const client = createHttpX402ResourceClient(
    {
      url: "https://merchant.example/data",
      method: "POST",
      headers: { accept: "application/json" },
      body: "request-body",
    },
    async (url, init) => {
      call = { url, init };
      return { ok: true, status: 200 };
    },
  );
  const result = await client.sendPaymentSignature("signed-payment-payload");
  assert.equal(result.ok, true);
  assert.equal(call.url, "https://merchant.example/data");
  assert.equal(call.init.headers["PAYMENT-SIGNATURE"], "signed-payment-payload");
  assert.equal(call.init.body, "request-body");
  assert.throws(() =>
    createHttpX402ResourceClient({ url: "http://merchant.example" }, async () => ({
      ok: true,
      status: 200,
    })),
  );
  assert.throws(() =>
    createHttpX402ResourceClient(
      { url: "https://merchant.example", headers: { "PAYMENT-SIGNATURE": "caller-must-not-set" } },
      async () => ({ ok: true, status: 200 }),
    ),
  );
});

test("official SVM adapter rejects a requirement not advertised by merchant", async () => {
  const offered = {
    scheme: "exact",
    network: "solana:mainnet",
    amount: "100",
    asset: "USDC",
    payTo: "merchant",
    maxTimeoutSeconds: 60,
  };
  const signer = createOfficialSvmExactSigner(
    {
      createPaymentPayload: async () => ({ signed: true }),
      encodePaymentSignatureHeader: () => ({ "PAYMENT-SIGNATURE": "official-signed-payload" }),
    },
    { x402Version: 2, resource: { url: "https://merchant.example/data" }, accepts: [offered] },
  );
  await assert.rejects(() => signer.createPaymentSignature({ ...offered, amount: "101" }));
  assert.deepEqual(await signer.createPaymentSignature(offered), {
    paymentSignature: "official-signed-payload",
  });
});
