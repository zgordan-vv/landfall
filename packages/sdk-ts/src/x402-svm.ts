/** Adapter for an application-configured official x402 SVM `exact` client. */

import type { X402PaymentRequirement, X402PaymentSigner } from "./x402.js";

/** Complete v2 challenge consumed by the official x402 SDK. */
export interface OfficialX402PaymentRequired {
  readonly x402Version: 2;
  readonly resource: Readonly<Record<string, unknown>>;
  readonly accepts: readonly (X402PaymentRequirement & Readonly<Record<string, unknown>>)[];
  readonly extensions?: Readonly<Record<string, unknown>>;
}

/** Structural contract of x402-foundation's `x402HTTPClient`. */
export interface OfficialX402HttpClient {
  createPaymentPayload(paymentRequired: OfficialX402PaymentRequired): Promise<unknown>;
  encodePaymentSignatureHeader(paymentPayload: unknown): Readonly<Record<string, string>>;
}

/**
 * Makes an `X402PaymentSigner` backed by an application-owned official
 * x402 SVM `exact` client. Configure it in the wallet process with
 * `@x402/core`, `@x402/svm`, and its compatible `@solana/kit` version.
 * The key-bearing signer never crosses this adapter boundary.
 */
export function createOfficialSvmExactSigner(
  httpClient: OfficialX402HttpClient,
  paymentRequired: OfficialX402PaymentRequired,
): X402PaymentSigner {
  if (paymentRequired.x402Version !== 2 || paymentRequired.accepts.length === 0) {
    throw new Error("official x402 SVM signer requires a v2 payment challenge");
  }
  return Object.freeze({
    async createPaymentSignature(requirement: X402PaymentRequirement) {
      if (!paymentRequired.accepts.some((candidate) => sameRequirement(candidate, requirement))) {
        throw new Error("payment requirement was not offered by the merchant challenge");
      }
      const payload = await httpClient.createPaymentPayload(paymentRequired);
      const header = httpClient.encodePaymentSignatureHeader(payload)["PAYMENT-SIGNATURE"];
      if (typeof header !== "string" || header.length === 0) {
        throw new Error("official x402 SVM signer returned no PAYMENT-SIGNATURE header");
      }
      return Object.freeze({ paymentSignature: header });
    },
  });
}

function sameRequirement(left: X402PaymentRequirement, right: X402PaymentRequirement): boolean {
  return left.scheme === right.scheme
    && left.network === right.network
    && left.amount === right.amount
    && left.asset === right.asset
    && left.payTo === right.payTo
    && left.maxTimeoutSeconds === right.maxTimeoutSeconds;
}
