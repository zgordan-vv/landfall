import test from "node:test";
import assert from "node:assert/strict";
import { parsePaymentRequiredHeader, preAuthorizeX402Requirement } from "../dist/index.js";

const policyId = "0198ef00-0000-7000-8000-000000000401";
const encode = (value) => Buffer.from(JSON.stringify(value)).toString("base64url");
const challenge = () => encode({ x402Version: 2, resource: { url: "https://API.example.com/v1/data?private=no" }, accepts: [{ scheme: "exact", network: "solana:mainnet", amount: "1000000", asset: "USDC", payTo: "merchant", maxTimeoutSeconds: 60 }] });

test("parses a v2 PAYMENT-REQUIRED challenge and keeps only the merchant origin", () => {
  const parsed = parsePaymentRequiredHeader(challenge());
  const request = preAuthorizeX402Requirement(parsed, parsed.accepts[0], policyId, "agent", "request-1");
  assert.equal(request.merchantOrigin, "https://api.example.com");
  assert.equal(request.amountAtomic, "1000000");
});

test("rejects malformed challenges before any wallet operation", () => {
  assert.throws(() => parsePaymentRequiredHeader("not-base64"));
  assert.throws(() => parsePaymentRequiredHeader(encode({ x402Version: 1, resource: {}, accepts: [] })));
  assert.throws(() => parsePaymentRequiredHeader(encode({ x402Version: 2, resource: { url: "https://api.example.com" }, accepts: [{ scheme: "exact", network: "solana:mainnet", amount: "01", asset: "USDC", payTo: "merchant", maxTimeoutSeconds: 60 }] })));
});
