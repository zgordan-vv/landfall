import type { OutboundBatch } from "./batching.js";

export interface TransportResponse { readonly status: number }
export type BatchTransport<T> = (batch: OutboundBatch<T>, compressed: boolean) => Promise<TransportResponse>;

export interface FetchResponseLike { readonly status: number }
export type FetchLike = (input: string, init: { readonly method: "POST"; readonly headers: Readonly<Record<string, string>>; readonly body: string }) => Promise<FetchResponseLike>;

/** Sends one JSON event batch to the collector's live ingestion endpoint. */
export function createHttpBatchTransport<T>(collectorUrl: string, fetcher: FetchLike): BatchTransport<T> {
  const endpoint = `${collectorUrl.replace(/\/$/, "")}/v1/ingest`;
  return async (batch): Promise<TransportResponse> => fetcher(endpoint, {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify({ batch_id: batch.batchId, events: batch.events }),
  });
}

export interface RetryOptions {
  readonly maxAttempts?: number;
  readonly baseDelayMs?: number;
  readonly jitter?: () => number;
  readonly sleep?: (delayMs: number) => Promise<void>;
}

/** Sends a batch with bounded exponential backoff while preserving its identity. */
export async function sendWithRetry<T>(batch: OutboundBatch<T>, transport: BatchTransport<T>, options: RetryOptions = {}): Promise<TransportResponse> {
  const maxAttempts = options.maxAttempts ?? 3;
  const baseDelayMs = options.baseDelayMs ?? 100;
  if (!Number.isInteger(maxAttempts) || maxAttempts < 1) throw new Error("maxAttempts must be positive");
  const jitter = options.jitter ?? Math.random;
  const sleep = options.sleep ?? ((delayMs: number) => new Promise<void>((resolve) => {
    const timerHost = globalThis as unknown as { setTimeout: (handler: () => void, timeout: number) => unknown };
    timerHost.setTimeout(resolve, delayMs);
  }));
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      const response = await transport(batch, true);
      if ((response.status < 500 && response.status !== 429) || attempt === maxAttempts) return response;
    } catch (error) {
      if (attempt === maxAttempts) throw error;
    }
    const delay = baseDelayMs * 2 ** (attempt - 1) * (0.5 + Math.max(0, Math.min(1, jitter())));
    await sleep(delay);
  }
  throw new Error("unreachable retry state");
}
