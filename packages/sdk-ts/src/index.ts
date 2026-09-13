import type { TraceId, WireEvent } from "@landfall/protocol";
import { HealthCounters, type HealthChangeCallback, type SdkHealth } from "./diagnostics.js";
export * from "./diagnostics.js";
export * from "./builders.js";
export * from "./ids.js";
export * from "./fingerprint.js";
export * from "./privacy.js";
export * from "./buffer.js";
export * from "./batching.js";
export * from "./transport.js";
export * from "./flush.js";
export * from "./solana-boundary.js";
export * from "./x402.js";
import { boundedFlush, type FlushOptions, type FlushResult } from "./flush.js";
import { BatchAssembler, type OutboundBatch } from "./batching.js";
import { EventBuffer } from "./buffer.js";
import { createHttpBatchTransport, sendWithRetry, type FetchLike, type RetryOptions } from "./transport.js";

export interface SdkOptions {
  readonly collectorUrl: string;
  readonly onTelemetryError?: (error: TelemetryError) => void;
  readonly onHealthChange?: HealthChangeCallback;
  readonly maxBatchEvents?: number;
  readonly maxBufferEvents?: number;
  /** Injectable only for tests or runtimes that wrap the standard Fetch API. */
  readonly fetch?: FetchLike;
  readonly retry?: RetryOptions;
}

export interface SdkConfig {
  readonly collectorUrl: string;
  readonly maxBatchEvents: number;
  readonly maxBufferEvents: number;
}

export function validateConfig(options: SdkOptions): SdkConfig {
  const collectorUrl = options.collectorUrl.trim();
  if (!/^https:\/\/[^\s/]+(?:\/.*)?$/.test(collectorUrl) && !/^http:\/\/localhost(?::\d+)?(?:\/.*)?$/.test(collectorUrl)) {
    throw new Error("collectorUrl must use HTTPS (or localhost for development)");
  }
  const maxBatchEvents = options.maxBatchEvents ?? 100;
  const maxBufferEvents = options.maxBufferEvents ?? 1_000;
  if (!Number.isInteger(maxBatchEvents) || maxBatchEvents < 1 || maxBatchEvents > 1_000) throw new Error("maxBatchEvents must be between 1 and 1000");
  if (!Number.isInteger(maxBufferEvents) || maxBufferEvents < maxBatchEvents || maxBufferEvents > 100_000) throw new Error("maxBufferEvents must be between maxBatchEvents and 100000");
  return Object.freeze({ collectorUrl, maxBatchEvents, maxBufferEvents });
}

export interface TelemetryError {
  readonly code: "transport" | "validation" | "buffer_overflow";
  readonly message: string;
  readonly cause?: unknown;
}

export interface BusinessActionContext {
  readonly businessActionId: string;
  readonly name?: string;
}

export function createBusinessActionContext(businessActionId: string, name?: string): BusinessActionContext {
  if (businessActionId.trim().length === 0 || businessActionId.length > 128) throw new Error("businessActionId must be 1-128 characters");
  const context: BusinessActionContext = { businessActionId: businessActionId.trim() };
  return Object.freeze(name === undefined ? context : { ...context, name: name.slice(0, 160) });
}

export interface TraceContext {
  readonly traceId: TraceId;
  readonly businessAction?: BusinessActionContext;
  /** Captures a complete protocol event without blocking the customer operation. */
  emit(event: WireEvent): boolean;
}

/** Minimal non-blocking instrumentation facade. Telemetry failures are reported, never thrown. */
export class LandfallSdk {
  readonly #options: SdkOptions;
  readonly #config: SdkConfig;
  readonly #health: HealthCounters;
  readonly #buffer: EventBuffer<WireEvent>;
  readonly #assembler: BatchAssembler<WireEvent>;
  readonly #transport;
  readonly #retry: RetryOptions;
  #inFlight: OutboundBatch<WireEvent> | undefined;

  constructor(options: SdkOptions) {
    this.#config = validateConfig(options);
    this.#options = options;
    this.#health = new HealthCounters(options.onHealthChange);
    this.#buffer = new EventBuffer<WireEvent>(this.#config.maxBufferEvents);
    this.#assembler = new BatchAssembler<WireEvent>(this.#config.maxBatchEvents);
    const runtimeFetch = (globalThis as unknown as { fetch?: FetchLike }).fetch;
    const fetcher = options.fetch ?? runtimeFetch?.bind(globalThis);
    if (fetcher === undefined) throw new Error("Fetch API is required to send telemetry");
    this.#transport = createHttpBatchTransport(this.#config.collectorUrl, fetcher);
    this.#retry = options.retry ?? {};
  }

  get config(): SdkConfig { return this.#config; }
  get health(): SdkHealth { return this.#health.snapshot; }

  get bufferedEventCount(): number { return this.#buffer.size + (this.#inFlight?.events.length ?? 0); }

  recordDroppedEvents(count = 1): void { this.#health.recordDropped(count); }
  recordTransportFailure(count = 1): void { this.#health.recordTransportFailure(count); }

  /** Delivers buffered telemetry. An optional operation is retained for backward compatibility. */
  async flush(flushOperation?: () => Promise<void>, options: FlushOptions = {}): Promise<FlushResult> {
    const operation = flushOperation ?? (() => this.#flushBuffered());
    const result = await boundedFlush(operation, options);
    if (result.status === "failed") {
      this.recordTransportFailure();
      this.reportTelemetryError({ code: "transport", message: "shutdown flush failed", cause: result.error });
    }
    return result;
  }

  startTrace(traceId: TraceId, businessAction?: BusinessActionContext): TraceContext {
    const context: TraceContext = {
      traceId,
      emit: (event: WireEvent): boolean => this.capture(event, traceId),
    };
    if (businessAction !== undefined) return { ...context, businessAction };
    return context;
  }

  reportTelemetryError(error: TelemetryError): void {
    this.#options.onTelemetryError?.(error);
  }

  /** Queues one event. It never throws into the customer transaction path. */
  capture(event: WireEvent, expectedTraceId?: TraceId): boolean {
    if (expectedTraceId !== undefined && event.trace_id !== expectedTraceId) {
      this.reportTelemetryError({ code: "validation", message: "event trace_id does not match its trace context" });
      return false;
    }
    if (!this.#buffer.push(event)) {
      this.recordDroppedEvents();
      this.reportTelemetryError({ code: "buffer_overflow", message: "telemetry buffer is full" });
      return false;
    }
    return true;
  }

  async #flushBuffered(): Promise<void> {
    const batch = this.#inFlight ?? this.#assembler.assemble(this.#buffer);
    if (batch === undefined) return;
    this.#inFlight = batch;
    let response;
    try {
      response = await sendWithRetry(batch, this.#transport, this.#retry);
    } catch (cause) {
      this.#restoreInFlight();
      throw cause;
    }
    if (response.status >= 200 && response.status < 300) {
      this.#inFlight = undefined;
      return;
    }
    this.#restoreInFlight();
    throw new Error(`collector rejected telemetry batch with HTTP ${response.status}`);
  }

  #restoreInFlight(): void {
    const batch = this.#inFlight;
    if (batch === undefined) return;
    this.#buffer.restoreFront(batch.events);
    this.#inFlight = undefined;
  }
}
