import type {
  EventSource,
  PrivacyMode,
  ProjectId,
  EnvironmentId,
  TraceId,
  WireEvent,
} from "@landfall/protocol";
import type { LandfallSdk } from "./index.js";
import { generateUuidV7 } from "./ids.js";

type EventOf<Type extends WireEvent["event_type"]> = Extract<WireEvent, { event_type: Type }>;
type AttributesOf<Type extends WireEvent["event_type"]> = EventOf<Type>["attributes"];

/** Immutable identity for one application-owned Solana transaction attempt. */
export interface SolanaLifecycleContext {
  readonly projectId: ProjectId;
  readonly environmentId: EnvironmentId;
  readonly traceId: TraceId;
  readonly source: EventSource;
  readonly privacyMode?: PrivacyMode;
  readonly businessActionId?: string;
}

/**
 * Emits validated lifecycle facts around an application's own signing and RPC calls.
 * The recorder never receives a private key and never serializes signed transaction bytes.
 */
export class SolanaLifecycleRecorder {
  readonly #sdk: LandfallSdk;
  readonly #context: SolanaLifecycleContext;

  constructor(sdk: LandfallSdk, context: SolanaLifecycleContext) {
    this.#sdk = sdk;
    this.#context = Object.freeze({ ...context });
  }

  traceCreated(attributes: AttributesOf<"solana.trace.created">): boolean {
    return this.#emit("solana.trace.created", attributes);
  }
  blockhashAcquired(attributes: AttributesOf<"solana.blockhash.acquired">): boolean {
    return this.#emit("solana.blockhash.acquired", attributes);
  }
  signingStarted(attributes: AttributesOf<"solana.signing.started">): boolean {
    return this.#emit("solana.signing.started", attributes);
  }
  signingCompleted(attributes: AttributesOf<"solana.signing.completed">): boolean {
    return this.#emit("solana.signing.completed", attributes);
  }
  submissionStarted(attributes: AttributesOf<"solana.submission.started">): boolean {
    return this.#emit("solana.submission.started", attributes);
  }
  submissionCompleted(attributes: AttributesOf<"solana.submission.completed">): boolean {
    return this.#emit("solana.submission.completed", attributes);
  }
  statusObserved(attributes: AttributesOf<"solana.status.observed">): boolean {
    return this.#emit("solana.status.observed", attributes);
  }

  #emit<Type extends WireEvent["event_type"]>(type: Type, attributes: AttributesOf<Type>): boolean {
    const event = {
      schema_version: "1.0",
      event_id: generateUuidV7(),
      event_type: type,
      occurred_at: new Date().toISOString(),
      project_id: this.#context.projectId,
      environment_id: this.#context.environmentId,
      trace_id: this.#context.traceId,
      ...(this.#context.businessActionId === undefined
        ? {}
        : { business_action_id: this.#context.businessActionId }),
      source: this.#context.source,
      privacy_mode: this.#context.privacyMode ?? "standard",
      privacy_policy_version: "1.0",
      redaction_version: "1.0",
      attributes,
    } as EventOf<Type>;
    return this.#sdk.capture(event);
  }
}

/** Creates a recorder for one trace without changing the application's transaction control flow. */
export function createSolanaLifecycleRecorder(
  sdk: LandfallSdk,
  context: SolanaLifecycleContext,
): SolanaLifecycleRecorder {
  return new SolanaLifecycleRecorder(sdk, context);
}
