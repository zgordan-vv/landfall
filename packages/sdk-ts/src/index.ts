import type { TraceId } from "@landfall/protocol";

export interface SdkOptions {
  readonly collectorUrl: string;
  readonly onTelemetryError?: (error: TelemetryError) => void;
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

export interface TraceContext {
  readonly traceId: TraceId;
  readonly businessAction?: BusinessActionContext;
  emit(event: unknown): void;
}

/** Minimal non-blocking instrumentation facade. Telemetry failures are reported, never thrown. */
export class LandfallSdk {
  readonly #options: SdkOptions;

  constructor(options: SdkOptions) {
    if (!options.collectorUrl.startsWith("https://") && !options.collectorUrl.startsWith("http://localhost")) {
      throw new Error("collectorUrl must use HTTPS (or localhost for development)");
    }
    this.#options = options;
  }

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
