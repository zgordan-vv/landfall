/** Safe parsing and pre-sign authorization for x402 v2 HTTP challenges. */

export interface X402PaymentRequirement {
  readonly scheme: string;
  readonly network: string;
  readonly amount: string;
  readonly asset: string;
  readonly payTo: string;
  readonly maxTimeoutSeconds: number;
}

export interface X402PaymentRequired {
  readonly x402Version: 2;
  readonly resource: { readonly url: string };
  readonly accepts: readonly X402PaymentRequirement[];
}

export interface X402PreAuthorization {
  readonly policyId: string;
  readonly agentId: string;
  readonly merchantOrigin: string;
  readonly network: string;
  readonly asset: string;
  readonly amountAtomic: string;
  readonly idempotencyKey: string;
}

export interface X402Authorizer {
  authorize(request: X402PreAuthorization): Promise<X402AuthorizationDecision>;
}

export interface X402AuthorizationDecision {
  readonly auditId: string;
  readonly decision: "approved" | "denied" | "settled" | "failed";
  readonly reasonCode: string;
  readonly replayed: boolean;
}

/** Signs through the application's own wallet boundary; Landfall never implements this. */
export interface X402PaymentSigner {
  createPaymentSignature(requirement: X402PaymentRequirement): Promise<{ readonly paymentSignature: string; readonly settlementReference?: string }>;
}

/** Sends the signed x402 header directly to the merchant resource. */
export interface X402PaidResourceClient<Response> {
  sendPaymentSignature(paymentSignature: string): Promise<X402PaidResourceResponse<Response>>;
}

/** The merchant response required to determine whether a payment is complete. */
export interface X402PaidResourceResponse<Response> {
  readonly ok: boolean;
  readonly status: number;
  readonly response: Response;
}

export interface X402SettlementRecorder {
  recordSettlement(input: { readonly auditId: string; readonly outcome: "settled" | "failed"; readonly reasonCode: string; readonly settlementReference?: string }): Promise<void>;
}

export type X402PaymentExecution<Response> =
  | { readonly kind: "denied"; readonly authorization: X402AuthorizationDecision }
  | { readonly kind: "already-settled" | "already-failed"; readonly authorization: X402AuthorizationDecision }
  | { readonly kind: "settled"; readonly authorization: X402AuthorizationDecision; readonly response: Response };

export interface X402AuthorizeFetchResponse { readonly status: number; json(): Promise<unknown>; }
export type X402AuthorizeFetch = (input: string, init: { readonly method: "POST"; readonly headers: Readonly<Record<string, string>>; readonly body: string }) => Promise<X402AuthorizeFetchResponse>;

/** Creates the HTTP client for Landfall's F3 pre-payment decision endpoint. */
export function createHttpX402Authorizer(policyServiceUrl: string, bearerToken: string, fetcher: X402AuthorizeFetch): X402Authorizer {
  const endpoint = `${policyServiceUrl.replace(/\/$/, "")}/v1/x402/authorize`;
  if (!/^https:\/\/[^\s/]+(?:\/.*)?$/.test(policyServiceUrl) || bearerToken.trim() === "") throw new Error("x402 policy service requires HTTPS and a bearer token");
  return Object.freeze({
    async authorize(request: X402PreAuthorization): Promise<X402AuthorizationDecision> {
      const response = await fetcher(endpoint, { method: "POST", headers: { authorization: `Bearer ${bearerToken}`, "content-type": "application/json", accept: "application/json" }, body: JSON.stringify({ policy_id: request.policyId, agent_id: request.agentId, merchant_origin: request.merchantOrigin, network: request.network, asset: request.asset, amount_atomic: request.amountAtomic, idempotency_key: request.idempotencyKey }) });
      const body = await response.json();
      if (!isRecord(body) || typeof body["audit_id"] !== "string" || !isUuid(body["audit_id"]) || (body["decision"] !== "approved" && body["decision"] !== "denied" && body["decision"] !== "settled" && body["decision"] !== "failed") || typeof body["reason_code"] !== "string" || typeof body["replayed"] !== "boolean") throw new Error(`x402 policy service returned an invalid response (${response.status})`);
      return Object.freeze({ auditId: body["audit_id"], decision: body["decision"], reasonCode: body["reason_code"], replayed: body["replayed"] });
    },
  });
}

/** Creates the HTTP reporter for terminal outcomes; it has no signature parameter. */
export function createHttpX402SettlementRecorder(policyServiceUrl: string, bearerToken: string, fetcher: X402AuthorizeFetch): X402SettlementRecorder {
  const endpoint = `${policyServiceUrl.replace(/\/$/, "")}/v1/x402/settlements`;
  if (!/^https:\/\/[^\s/]+(?:\/.*)?$/.test(policyServiceUrl) || bearerToken.trim() === "") throw new Error("x402 policy service requires HTTPS and a bearer token");
  return Object.freeze({
    async recordSettlement(input: { readonly auditId: string; readonly outcome: "settled" | "failed"; readonly reasonCode: string; readonly settlementReference?: string }): Promise<void> {
      const response = await fetcher(endpoint, { method: "POST", headers: { authorization: `Bearer ${bearerToken}`, "content-type": "application/json", accept: "application/json" }, body: JSON.stringify({ audit_id: input.auditId, outcome: input.outcome, reason_code: input.reasonCode, settlement_reference: input.settlementReference }) });
      if (response.status !== 200) throw new Error(`x402 settlement service returned ${response.status}`);
    },
  });
}

/**
 * Runs the only permitted signing path: authorize, ask the external signer,
 * send its opaque header to the merchant, then record a terminal audit state.
 * The signature is deliberately not passed to `settlementRecorder`.
 */
export async function executeAuthorizedX402Payment<Response>(
  authorization: X402PreAuthorization,
  requirement: X402PaymentRequirement,
  authorizer: X402Authorizer,
  signer: X402PaymentSigner,
  resourceClient: X402PaidResourceClient<Response>,
  settlementRecorder: X402SettlementRecorder,
): Promise<X402PaymentExecution<Response>> {
  const decision = await authorizer.authorize(authorization);
  if (decision.decision === "denied") return Object.freeze({ kind: "denied", authorization: decision });
  if (decision.decision === "settled") return Object.freeze({ kind: "already-settled" as const, authorization: decision });
  if (decision.decision === "failed") return Object.freeze({ kind: "already-failed" as const, authorization: decision });
  let signed: { readonly paymentSignature: string; readonly settlementReference?: string } | undefined;
  try {
    signed = await signer.createPaymentSignature(requirement);
    if (!validText(signed.paymentSignature, 16_384)) throw new Error("external x402 signer returned an invalid payment signature");
    const paidResponse = await resourceClient.sendPaymentSignature(signed.paymentSignature);
    if (!paidResponse.ok) throw new Error(`merchant rejected x402 payment with HTTP ${paidResponse.status}`);
    await settlementRecorder.recordSettlement(settlementInput(decision.auditId, "settled", "resource_response_received", signed.settlementReference));
    return Object.freeze({ kind: "settled" as const, authorization: decision, response: paidResponse.response });
  } catch (cause) {
    await settlementRecorder.recordSettlement(settlementInput(decision.auditId, "failed", "external_payment_failed", signed?.settlementReference));
    throw cause;
  }
}

function settlementInput(auditId: string, outcome: "settled" | "failed", reasonCode: string, settlementReference: string | undefined): { readonly auditId: string; readonly outcome: "settled" | "failed"; readonly reasonCode: string; readonly settlementReference?: string } {
  return settlementReference === undefined ? { auditId, outcome, reasonCode } : { auditId, outcome, reasonCode, settlementReference };
}

/** Decodes and validates the standard base64 `PAYMENT-REQUIRED` HTTP header. */
export function parsePaymentRequiredHeader(header: string): X402PaymentRequired {
  let decoded: unknown;
  try {
    decoded = JSON.parse(decodeBase64UrlUtf8(header));
  } catch {
    throw new Error("PAYMENT-REQUIRED is not valid base64 JSON");
  }
  if (!isRecord(decoded) || decoded["x402Version"] !== 2 || !isRecord(decoded["resource"]) || typeof decoded["resource"]["url"] !== "string" || !Array.isArray(decoded["accepts"]) || decoded["accepts"].length === 0) {
    throw new Error("PAYMENT-REQUIRED is not a supported x402 v2 challenge");
  }
  const accepts = decoded["accepts"].map((candidate) => parseRequirement(candidate));
  return Object.freeze({ x402Version: 2, resource: Object.freeze({ url: decoded["resource"]["url"] }), accepts: Object.freeze(accepts) });
}

/** Builds the input for Landfall's policy service; it never signs or settles a payment. */
export function preAuthorizeX402Requirement(challenge: X402PaymentRequired, requirement: X402PaymentRequirement, policyId: string, agentId: string, idempotencyKey: string): X402PreAuthorization {
  const merchantOrigin = normalizeHttpsOrigin(challenge.resource.url);
  if (!isUuid(policyId) || !validText(agentId, 160) || !validText(idempotencyKey, 256)) throw new Error("x402 pre-authorization identity is invalid");
  return Object.freeze({ policyId, agentId, merchantOrigin, network: requirement.network, asset: requirement.asset, amountAtomic: requirement.amount, idempotencyKey });
}

function parseRequirement(value: unknown): X402PaymentRequirement {
  if (!isRecord(value) || typeof value["scheme"] !== "string" || typeof value["network"] !== "string" || !canonicalAtomic(value["amount"]) || typeof value["asset"] !== "string" || typeof value["payTo"] !== "string" || !Number.isInteger(value["maxTimeoutSeconds"]) || (value["maxTimeoutSeconds"] as number) <= 0) throw new Error("x402 payment requirement is invalid");
  return Object.freeze({ scheme: value["scheme"], network: value["network"], amount: value["amount"], asset: value["asset"], payTo: value["payTo"], maxTimeoutSeconds: value["maxTimeoutSeconds"] as number });
}

function normalizeHttpsOrigin(resourceUrl: string): string {
  const match = /^https:\/\/([^\/?#@\s]+)(?:[/?#].*)?$/i.exec(resourceUrl);
  const host = match?.[1];
  if (host === undefined || host === "") throw new Error("x402 resource must use a credential-free HTTPS origin");
  return `https://${host.toLowerCase()}`;
}

function decodeBase64UrlUtf8(value: string): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  const source = value.replace(/-/g, "+").replace(/_/g, "/").replace(/=+$/, "");
  if (source.length === 0 || /[^A-Za-z0-9+/]/.test(source)) throw new Error("invalid base64");
  const bytes: number[] = [];
  for (let index = 0; index < source.length; index += 4) {
    const first = alphabet.indexOf(source[index] ?? ""); const second = alphabet.indexOf(source[index + 1] ?? "");
    const third = alphabet.indexOf(source[index + 2] ?? "A"); const fourth = alphabet.indexOf(source[index + 3] ?? "A");
    if (first < 0 || second < 0 || third < 0 || fourth < 0) throw new Error("invalid base64");
    const bits = (first << 18) | (second << 12) | (third << 6) | fourth;
    bytes.push((bits >> 16) & 255); if (index + 2 < source.length) bytes.push((bits >> 8) & 255); if (index + 3 < source.length) bytes.push(bits & 255);
  }
  return decodeUtf8(bytes);
}

function decodeUtf8(bytes: number[]): string { return new (globalThis as unknown as { TextDecoder: new () => { decode(input: Uint8Array): string } }).TextDecoder().decode(Uint8Array.from(bytes)); }

function canonicalAtomic(value: unknown): value is string { return typeof value === "string" && /^(?:[1-9][0-9]*)$/.test(value); }
function validText(value: string, maximum: number): boolean { return value.trim().length > 0 && value.length <= maximum; }
function isUuid(value: string): boolean { return /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value); }
function isRecord(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
