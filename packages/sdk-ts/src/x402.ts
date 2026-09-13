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
  authorize(request: X402PreAuthorization): Promise<{ readonly decision: "approved" | "denied"; readonly reasonCode: string }>;
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
