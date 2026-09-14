import type {
  EventSource,
  PrivacyMode,
  ProjectId,
  TraceId,
  EnvironmentId,
  EventId,
  SolanaSigningStartedEvent,
  SolanaSubmissionStartedEvent,
  SolanaTraceCreatedEvent,
} from "@landfall/protocol";

export interface ManualEventContext {
  readonly eventId: EventId;
  readonly occurredAt: string;
  readonly projectId: ProjectId;
  readonly environmentId: EnvironmentId;
  readonly traceId: TraceId;
  readonly source: EventSource;
  readonly privacyMode?: PrivacyMode;
}

const envelope = <T extends string>(context: ManualEventContext, eventType: T) => ({
  schema_version: "1.0" as const,
  event_id: context.eventId,
  event_type: eventType,
  occurred_at: context.occurredAt,
  project_id: context.projectId,
  environment_id: context.environmentId,
  trace_id: context.traceId,
  source: context.source,
  privacy_mode: context.privacyMode ?? ("standard" as const),
  privacy_policy_version: "1.0",
  redaction_version: "1.0",
});

export function buildTraceCreated(
  context: ManualEventContext,
  attributes: SolanaTraceCreatedEvent["attributes"],
): SolanaTraceCreatedEvent {
  return { ...envelope(context, "solana.trace.created"), attributes };
}

export function buildSigningStarted(
  context: ManualEventContext,
  attributes: SolanaSigningStartedEvent["attributes"],
): SolanaSigningStartedEvent {
  return { ...envelope(context, "solana.signing.started"), attributes };
}

export function buildSubmissionStarted(
  context: ManualEventContext,
  attributes: SolanaSubmissionStartedEvent["attributes"],
): SolanaSubmissionStartedEvent {
  return { ...envelope(context, "solana.submission.started"), attributes };
}
