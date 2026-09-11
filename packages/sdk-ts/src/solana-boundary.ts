import type { SimulationRpcResult, SignedBytesFingerprint, UuidV7 } from "@landfall/protocol";
import { fingerprintSignedBytes, type HmacBytes } from "./fingerprint.js";

/** Adapter-facing types intentionally contain no @solana/kit runtime imports. */
export interface SolanaBlockhashSnapshot {
  readonly blockhash: string;
  readonly lastValidBlockHeight: string;
}

export type BlockHeightInput = string | bigint;

export interface SimulationSnapshot {
  readonly rpcResult: SimulationRpcResult;
  readonly unitsConsumed?: string;
  readonly logsPresent?: boolean;
  readonly error?: unknown;
}

export type MonotonicClock = () => bigint;

/** Fingerprints a transient copy of signed bytes and wipes that copy immediately. */
export function fingerprintSignedBytesInMemory(keyId: UuidV7, signedBytes: Uint8Array, hmac: HmacBytes): SignedBytesFingerprint {
  const transient = signedBytes.slice();
  try {
    return fingerprintSignedBytes(keyId, transient, hmac);
  } finally {
    transient.fill(0);
  }
}

export interface SigningMeasurement<T> {
  readonly result: "completed" | "failed";
  readonly durationNs: string;
  readonly value?: T;
  readonly error?: unknown;
}

/** Measures application-owned signing without receiving or inspecting signer material. */
export async function measureSigning<T>(signOperation: () => Promise<T>, clock: MonotonicClock): Promise<SigningMeasurement<T>> {
  const started = clock();
  try {
    const value = await signOperation();
    return Object.freeze({ result: "completed" as const, durationNs: durationDelta(started, clock()), value });
  } catch (error) {
    return Object.freeze({ result: "failed" as const, durationNs: durationDelta(started, clock()), error });
  }
}

function durationDelta(started: bigint, ended: bigint): string {
  if (ended < started) throw new Error("monotonic clock moved backwards");
  return (ended - started).toString(10);
}

export function normalizeSimulationResult(input: unknown): SimulationSnapshot {
  if (input === null || typeof input !== "object") return { rpcResult: "malformed_response" };
  const value = input as { err?: unknown; unitsConsumed?: unknown; logs?: unknown; blockhashNotFound?: unknown };
  let rpcResult: SimulationRpcResult = "succeeded";
  if (value.blockhashNotFound === true) rpcResult = "blockhash_not_found";
  else if (value.err !== undefined && value.err !== null) rpcResult = "execution_error";
  const unitsConsumed = normalizeUnsignedDecimal(value.unitsConsumed);
  const snapshot: SimulationSnapshot = { rpcResult, ...(unitsConsumed === undefined ? {} : { unitsConsumed }), ...(value.logs === undefined ? {} : { logsPresent: Array.isArray(value.logs) && value.logs.length > 0 }), ...(value.err === undefined || value.err === null ? {} : { error: value.err }) };
  return Object.freeze(snapshot);
}

function normalizeUnsignedDecimal(value: unknown): string | undefined {
  if (value === undefined || value === null) return undefined;
  if (typeof value === "bigint") return value < 0n ? undefined : value.toString(10);
  if (typeof value === "string" && /^(0|[1-9][0-9]*)$/.test(value.trim())) return value.trim();
  if (typeof value === "number" && Number.isSafeInteger(value) && value >= 0) return String(value);
  return undefined;
}

/** Normalizes Kit's bigint/string block-height representations at the boundary. */
export function normalizeBlockhashSnapshot(blockhash: string, lastValidBlockHeight: BlockHeightInput): SolanaBlockhashSnapshot {
  const normalizedHash = blockhash.trim();
  const normalizedHeight = typeof lastValidBlockHeight === "bigint"
    ? lastValidBlockHeight.toString(10)
    : lastValidBlockHeight.trim();
  if (normalizedHash.length === 0) throw new Error("blockhash must not be empty");
  if (!/^(0|[1-9][0-9]*)$/.test(normalizedHeight)) throw new Error("lastValidBlockHeight must be a canonical unsigned decimal");
  return Object.freeze({ blockhash: normalizedHash, lastValidBlockHeight: normalizedHeight });
}

export interface SolanaRoute {
  readonly routeId: string;
  readonly endpoint?: string;
}

export interface SubmissionAttemptConfig {
  readonly attemptId: string;
  readonly route: SolanaRoute;
  readonly attemptSequence: number;
  readonly encoding: "base64" | "base58";
  readonly skipPreflight: boolean;
  readonly maxRetries?: number;
}

export interface SubmissionAttemptResult<T> {
  readonly config: SubmissionAttemptConfig;
  readonly result: "accepted" | "failed";
  readonly value?: T;
  readonly error?: unknown;
}

/** Executes one application submission attempt while retaining route metadata. */
export async function submitWithRoute<T>(config: SubmissionAttemptConfig, submit: () => Promise<T>): Promise<SubmissionAttemptResult<T>> {
  if (config.attemptId.trim().length === 0) throw new Error("attemptId must not be empty");
  if (config.route.routeId.trim().length === 0) throw new Error("routeId must not be empty");
  if (!Number.isInteger(config.attemptSequence) || config.attemptSequence < 1) throw new Error("attemptSequence must be positive");
  try {
    const value = await submit();
    return Object.freeze({ config, result: "accepted" as const, value });
  } catch (error) {
    return Object.freeze({ config, result: "failed" as const, error });
  }
}

export interface SolanaClientPort<TPrepared, TSigned, TSignature> {
  readonly getLatestBlockhash: () => Promise<SolanaBlockhashSnapshot>;
  readonly simulate: (transaction: TPrepared) => Promise<unknown>;
  /** Signing remains application-owned; the adapter receives only the signed value. */
  readonly sign: (transaction: TPrepared) => Promise<TSigned>;
  readonly submit: (signed: TSigned, route: SolanaRoute) => Promise<TSignature>;
  readonly confirm: (signature: TSignature, blockhash: SolanaBlockhashSnapshot) => Promise<unknown>;
}

export async function captureLatestBlockhash<TPrepared, TSigned, TSignature>(client: SolanaClientPort<TPrepared, TSigned, TSignature>): Promise<SolanaBlockhashSnapshot> {
  const snapshot = await client.getLatestBlockhash();
  return normalizeBlockhashSnapshot(snapshot.blockhash, snapshot.lastValidBlockHeight);
}

/** Wraps an application operation without changing its return value or error identity. */
export async function preserveCustomerOperation<T>(operation: () => Promise<T>): Promise<T> {
  return operation();
}
