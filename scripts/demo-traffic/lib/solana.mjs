/**
 * Minimal, dependency-free Solana helpers for the demo traffic generator.
 *
 * Builds and signs legacy System Program transfers (optionally with Compute
 * Budget instructions) using node:crypto Ed25519. The keypair is a disposable
 * devnet/local-validator wallet; it never leaves this process and is never
 * included in telemetry.
 */
import { createPrivateKey, createPublicKey, sign } from "node:crypto";
import { readFileSync } from "node:fs";

const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BASE58_MAP = new Map([...BASE58_ALPHABET].map((char, index) => [char, index]));

export function base58Encode(bytes) {
  let zeros = 0;
  while (zeros < bytes.length && bytes[zeros] === 0) zeros += 1;
  let value = 0n;
  for (const byte of bytes) value = (value << 8n) | BigInt(byte);
  let out = "";
  while (value > 0n) {
    out = BASE58_ALPHABET[Number(value % 58n)] + out;
    value /= 58n;
  }
  return "1".repeat(zeros) + out;
}

export function base58Decode(text) {
  let zeros = 0;
  while (zeros < text.length && text[zeros] === "1") zeros += 1;
  let value = 0n;
  for (const char of text) {
    const digit = BASE58_MAP.get(char);
    if (digit === undefined) throw new Error("invalid base58 character");
    value = value * 58n + BigInt(digit);
  }
  const bytes = [];
  while (value > 0n) {
    bytes.unshift(Number(value & 0xffn));
    value >>= 8n;
  }
  return Uint8Array.from([...new Array(zeros).fill(0), ...bytes]);
}

const PKCS8_ED25519_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");

/** Loads a solana-keygen JSON keypair (64 bytes: 32-byte seed + 32-byte public key). */
export function loadKeypair(path) {
  const raw = Uint8Array.from(JSON.parse(readFileSync(path, "utf8")));
  if (raw.length !== 64) throw new Error(`${path} is not a 64-byte Solana keypair`);
  const privateKey = createPrivateKey({
    key: Buffer.concat([PKCS8_ED25519_PREFIX, raw.subarray(0, 32)]),
    format: "der",
    type: "pkcs8",
  });
  const derived = createPublicKey(privateKey).export({ format: "der", type: "spki" }).subarray(-32);
  if (!Buffer.from(derived).equals(Buffer.from(raw.subarray(32)))) {
    throw new Error(`${path} public key does not match its seed`);
  }
  return Object.freeze({
    publicKey: raw.slice(32),
    privateKey,
    address: base58Encode(raw.slice(32)),
  });
}

const SYSTEM_PROGRAM = new Uint8Array(32);
const COMPUTE_BUDGET_PROGRAM = base58Decode("ComputeBudget111111111111111111111111111111");
const MEMO_PROGRAM = base58Decode("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

function shortVec(length) {
  const out = [];
  let rest = length;
  for (;;) {
    let byte = rest & 0x7f;
    rest >>= 7;
    if (rest === 0) {
      out.push(byte);
      return out;
    }
    byte |= 0x80;
    out.push(byte);
  }
}

function u32le(value) {
  const buffer = Buffer.alloc(4);
  buffer.writeUInt32LE(value);
  return [...buffer];
}

function u64le(value) {
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64LE(BigInt(value));
  return [...buffer];
}

/**
 * Builds and signs a legacy transfer. Returns the wire bytes, base64 encoding,
 * the transaction signature, and the index of the transfer instruction.
 */
export function buildSignedTransfer({
  payer,
  recipient,
  lamports,
  recentBlockhash,
  computeUnitLimit,
  computeUnitPriceMicroLamports,
  memo,
}) {
  const recipientKey = typeof recipient === "string" ? base58Decode(recipient) : recipient;
  // Order: writable signer, writable non-signer, read-only non-signers.
  const accounts = [
    payer.publicKey,
    recipientKey,
    SYSTEM_PROGRAM,
    COMPUTE_BUDGET_PROGRAM,
    MEMO_PROGRAM,
  ];
  const header = [1, 0, 3];
  const instructions = [];
  if (computeUnitLimit !== undefined) {
    instructions.push({ program: 3, accounts: [], data: [2, ...u32le(computeUnitLimit)] });
  }
  if (computeUnitPriceMicroLamports !== undefined) {
    instructions.push({
      program: 3,
      accounts: [],
      data: [3, ...u64le(computeUnitPriceMicroLamports)],
    });
  }
  const transferIndex = instructions.length;
  instructions.push({ program: 2, accounts: [0, 1], data: [...u32le(2), ...u64le(lamports)] });
  // Optional SPL Memo (a BPF program): its compute cost grows with memo length.
  if (memo !== undefined)
    instructions.push({ program: 4, accounts: [], data: [...Buffer.from(memo, "utf8")] });

  const message = Uint8Array.from([
    ...header,
    ...shortVec(accounts.length),
    ...accounts.flatMap((key) => [...key]),
    ...base58Decode(recentBlockhash),
    ...shortVec(instructions.length),
    ...instructions.flatMap((ix) => [
      ix.program,
      ...shortVec(ix.accounts.length),
      ...ix.accounts,
      ...shortVec(ix.data.length),
      ...ix.data,
    ]),
  ]);
  const signature = sign(null, message, payer.privateKey);
  const wire = Uint8Array.from([...shortVec(1), ...signature, ...message]);
  return Object.freeze({
    wire,
    base64: Buffer.from(wire).toString("base64"),
    signature: base58Encode(signature),
    transferIndex,
  });
}

export class RpcError extends Error {
  constructor(message, { code, data, httpStatus } = {}) {
    super(message);
    this.name = "RpcError";
    this.code = code;
    this.data = data;
    this.httpStatus = httpStatus;
  }
}

export class TransportError extends Error {
  constructor(message, kind, options) {
    super(message, options);
    this.name = "TransportError";
    this.kind = kind; // "timeout" | "connection_failed"
  }
}

/**
 * JSON-RPC client with an explicit per-request timeout. HTTP 429 responses are
 * retried with exponential backoff (honoring Retry-After): a rate-limited
 * request was not processed, so resending it, including sendTransaction, is safe.
 */
export function createRpc(endpoint, { timeoutMs = 15_000, maxRateLimitRetries = 6 } = {}) {
  let id = 0;
  return async function rpc(method, params = [], options = {}) {
    for (let attempt = 0; ; attempt += 1) {
      try {
        return await once(method, params, options);
      } catch (error) {
        if (!(error instanceof RpcError) || error.httpStatus !== 429) throw error;
        if (attempt >= maxRateLimitRetries) throw error;
        const backoffMs = error.retryAfterMs ?? Math.min(1_000 * 2 ** attempt, 30_000);
        await sleep(backoffMs + Math.random() * 250);
      }
    }
  };

  async function once(method, params, options) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), options.timeoutMs ?? timeoutMs);
    let response;
    try {
      response = await fetch(endpoint, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: (id += 1), method, params }),
        signal: controller.signal,
      });
    } catch (error) {
      throw new TransportError(
        `${method} transport failure`,
        controller.signal.aborted ? "timeout" : "connection_failed",
        { cause: error },
      );
    } finally {
      clearTimeout(timer);
    }
    if (response.status === 429) {
      const retryAfterSeconds = Number(response.headers.get("retry-after"));
      const error = new RpcError("rate limited", { httpStatus: 429 });
      if (Number.isFinite(retryAfterSeconds) && retryAfterSeconds > 0) {
        error.retryAfterMs = retryAfterSeconds * 1_000;
      }
      throw error;
    }
    if (!response.ok)
      throw new RpcError(`HTTP ${response.status}`, { httpStatus: response.status });
    const body = await response.json();
    if (body.error) {
      throw new RpcError(body.error.message ?? "rpc error", {
        code: body.error.code,
        data: body.error.data,
      });
    }
    return body.result;
  }
}

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
