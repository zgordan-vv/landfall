/**
 * Landfall protocol helpers for the demo traffic generator: UUIDv7 IDs, a
 * per-trace event recorder that posts batches to /v1/ingest, and mapping of
 * Solana RPC/transaction errors onto the protocol's normalized error categories.
 */
import { createHmac, randomBytes } from "node:crypto";

export function uuidV7() {
  const bytes = randomBytes(16);
  const now = BigInt(Date.now());
  for (let index = 0; index < 6; index += 1) {
    bytes[index] = Number((now >> BigInt(8 * (5 - index))) & 0xffn);
  }
  bytes[6] = (bytes[6] & 0x0f) | 0x70;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = bytes.toString("hex");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

export const elapsedNs = (startedAt) => (process.hrtime.bigint() - startedAt).toString();

/** Posts event batches to a Landfall collector. */
export function createIngestClient({ collectorUrl, ingestToken }) {
  const url = new URL("/v1/ingest", collectorUrl).toString();
  return async function post(events) {
    if (events.length === 0) return;
    const response = await fetch(url, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        ...(ingestToken ? { authorization: `Bearer ${ingestToken}` } : {}),
      },
      body: JSON.stringify({ batch_id: uuidV7(), events }),
    });
    if (response.status !== 200 && response.status !== 202) {
      const detail = await response.text().catch(() => "");
      throw new Error(`ingest returned HTTP ${response.status} ${detail.slice(0, 200)}`);
    }
  };
}

// Both sources run in this process and share its monotonic clock, so events
// carry monotonic_ns with one instance_id and the server can order them exactly.
const PROCESS_INSTANCE_ID = uuidV7();
const APPLICATION_SOURCE = Object.freeze({
  kind: "application",
  name: "landfall-demo-traffic",
  version: "1.0.0",
  instance_id: PROCESS_INSTANCE_ID,
});
const OBSERVER_SOURCE = Object.freeze({
  kind: "observer",
  name: "landfall-demo-traffic-observer",
  version: "1.0.0",
  instance_id: PROCESS_INSTANCE_ID,
});

/**
 * Collects lifecycle events for one trace. By default events are posted once,
 * when the trace finishes. With streaming enabled every intermediate flush()
 * posts too, so the dashboard shows the trace while it is in progress; note
 * that the server keeps "missing evidence" findings computed from those
 * partial snapshots even after the evidence arrives.
 */
export class TraceRecorder {
  #pending = [];
  #post;
  #streaming;

  constructor({ post, projectId, environmentId, businessActionId, streaming = false }) {
    this.#post = post;
    this.#streaming = streaming;
    this.traceId = uuidV7();
    this.projectId = projectId;
    this.environmentId = environmentId;
    this.businessActionId = businessActionId;
    this.eventCount = 0;
  }

  emit(eventType, attributes, { observer = false } = {}) {
    this.#pending.push({
      schema_version: "1.0",
      event_id: uuidV7(),
      event_type: eventType,
      occurred_at: new Date().toISOString(),
      monotonic_ns: process.hrtime.bigint().toString(),
      project_id: this.projectId,
      environment_id: this.environmentId,
      trace_id: this.traceId,
      ...(this.businessActionId ? { business_action_id: this.businessActionId } : {}),
      source: observer ? OBSERVER_SOURCE : APPLICATION_SOURCE,
      privacy_mode: "standard",
      privacy_policy_version: "1.0",
      redaction_version: "1.0",
      attributes,
    });
    this.eventCount += 1;
  }

  async flush({ final = false } = {}) {
    if (!final && !this.#streaming) return;
    const batch = this.#pending.splice(0);
    await this.#post(batch);
  }
}

/** HMAC-SHA256 fingerprint of exact signed bytes (lf-hmac-sha256-v1); raw bytes are never sent. */
export function fingerprint(keyId, key, signedBytes) {
  return {
    algorithm: "lf-hmac-sha256-v1",
    key_id: keyId,
    value_hex: createHmac("sha256", key).update(signedBytes).digest("hex"),
  };
}

const truncateCode = (value) => String(value).slice(0, 64);

/**
 * Maps a Solana TransactionError JSON value onto a protocol NormalizedError.
 * Only bounded category/code fields are retained; logs and messages are dropped.
 */
export function normalizeTransactionError(err, { transferIndex } = {}) {
  if (err === null || err === undefined) return undefined;
  if (typeof err === "string") {
    const known = {
      BlockhashNotFound: "blockhash_not_found",
      InsufficientFundsForFee: "insufficient_funds",
      AlreadyProcessed: "already_processed",
      AccountInUse: "account_in_use",
      SignatureFailure: "missing_signature",
    };
    return { category: known[err] ?? "unknown", code: truncateCode(err) };
  }
  if (typeof err === "object" && Array.isArray(err.InstructionError)) {
    const [index, detail] = err.InstructionError;
    if (detail === "ComputationalBudgetExceeded" || detail === "ProgramFailedToComplete") {
      return {
        category: "compute_budget_exceeded",
        code: truncateCode(detail),
        instruction_index: index,
      };
    }
    if (detail && typeof detail === "object" && "Custom" in detail) {
      // System Program custom error 1 is ResultWithNegativeLamports (insufficient lamports).
      if (index === transferIndex && detail.Custom === 1) {
        return {
          category: "insufficient_funds",
          code: "Custom",
          instruction_index: index,
          custom_code: 1,
        };
      }
      return {
        category: "custom_program_error",
        code: "Custom",
        instruction_index: index,
        custom_code: detail.Custom,
      };
    }
    return { category: "instruction_error", code: truncateCode(detail), instruction_index: index };
  }
  const key = typeof err === "object" ? Object.keys(err)[0] : undefined;
  return { category: "unknown", ...(key ? { code: truncateCode(key) } : {}) };
}

/** Detects compute exhaustion that some validator versions report only through logs. */
export function refineWithLogs(error, logs) {
  if (error && Array.isArray(logs) && logs.some((line) => /exceeded CUs meter/i.test(line))) {
    return { ...error, category: "compute_budget_exceeded" };
  }
  return error;
}

/** Classifies a sendTransaction RpcError into the protocol's submission rpc_result. */
export function classifySubmissionError(error, context) {
  if (error.httpStatus === 429)
    return { rpcResult: "rate_limited", error: { category: "rate_limited" } };
  if (error.httpStatus === 401 || error.httpStatus === 403) {
    return { rpcResult: "unauthorized", error: { category: "unauthorized" } };
  }
  const message = String(error.message ?? "");
  const dataErr = error.data?.err;
  if (/blockhash not found/i.test(message) || dataErr === "BlockhashNotFound") {
    return {
      rpcResult: "blockhash_rejected",
      error: { category: "blockhash_not_found", code: "BlockhashNotFound" },
    };
  }
  if (/already been processed/i.test(message) || dataErr === "AlreadyProcessed") {
    return {
      rpcResult: "already_processed",
      error: { category: "already_processed", code: "AlreadyProcessed" },
    };
  }
  const normalized = refineWithLogs(normalizeTransactionError(dataErr, context), error.data?.logs);
  return {
    rpcResult: "rejected",
    error: normalized ?? {
      category: "rpc_rejected",
      ...(error.code ? { code: truncateCode(error.code) } : {}),
    },
  };
}
