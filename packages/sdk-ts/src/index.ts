import type { TraceId } from "@landfall/protocol";
export * from "./builders.js";
export * from "./ids.js";
export * from "./fingerprint.js";
export * from "./privacy.js";
export * from "./buffer.js";
export * from "./batching.js";
export * from "./transport.js";

export interface SdkOptions {
  readonly collectorUrl: string;
  readonly onTelemetryError?: (error: TelemetryError) => void;
  readonly maxBatchEvents?: number;
  readonly maxBufferEvents?: number;
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
  emit(event: unknown): void;
}

/** Minimal non-blocking instrumentation facade. Telemetry failures are reported, never thrown. */
export class LandfallSdk {
  readonly #options: SdkOptions;
  readonly #config: SdkConfig;

  constructor(options: SdkOptions) {
    this.#config = validateConfig(options);
    this.#options = options;
  }

  get config(): SdkConfig { return this.#config; }

  startTrace(traceId: TraceId, businessAction?: BusinessActionContext): TraceContext {
    const context: TraceContext = {
      traceId,
      emit: (event: unknown): void => {
        void event;
        // Transport/buffering is intentionally deferred to later SDK tasks.
      },
    };
    if (businessAction !== undefined) return { ...context, businessAction };
    return context;
  }

  reportTelemetryError(error: TelemetryError): void {
    this.#options.onTelemetryError?.(error);
  }
}
