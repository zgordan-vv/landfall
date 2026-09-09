// DO NOT EDIT: generated from schemas/events/v1.
// Generator: json-schema-to-typescript 16.0.0
// Schema SHA-256: d2d4b9a6f7267ec7222e59293c40efcf92861e09b025485772ee3d11fe564a33
// Config SHA-256: b49e84b8de66ce8008462ea62b212e1103e769b8d2cea897d7ff95c8d8eb5218

/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "BatchId".
 */
export type BatchId = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "Rfc3339Utc".
 */
export type Rfc3339Utc = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "WireEvent".
 */
export type WireEvent =
  | LandfallDataQualityDetectedEvent
  | SolanaBlockhashAcquiredEvent
  | SolanaBusinessOutcomeObservedEvent
  | SolanaConfirmationWaitCompletedEvent
  | SolanaConfirmationWaitStartedEvent
  | SolanaExecutionEnrichedEvent
  | SolanaSigningCompletedEvent
  | SolanaSigningStartedEvent
  | SolanaSimulationCompletedEvent
  | SolanaSimulationStartedEvent
  | SolanaStatusObservedEvent
  | SolanaSubmissionCompletedEvent
  | SolanaSubmissionRetryScheduledEvent
  | SolanaSubmissionStartedEvent
  | SolanaTraceCreatedEvent;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "EventId".
 */
export type EventId = string;
/**
 * Non-negative duration or monotonic-clock reading in nanoseconds.
 *
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "DurationNsDecimal".
 */
export type DurationNsDecimal = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "ProjectId".
 */
export type ProjectId = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "EnvironmentId".
 */
export type EnvironmentId = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "TraceId".
 */
export type TraceId = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "BusinessActionId".
 */
export type BusinessActionId = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SourceKind".
 */
export type SourceKind = "sdk" | "application" | "observer" | "collector" | "cli";
/**
 * Canonical lowercase UUIDv7.
 *
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "UuidV7".
 */
export type UuidV7 = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "PrivacyMode".
 */
export type PrivacyMode = "standard" | "full" | "strict";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "Version".
 */
export type Version = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "DataQualityCategory".
 */
export type DataQualityCategory =
  | "telemetry_dropped"
  | "unsupported_transaction_version"
  | "unsupported_durable_nonce"
  | "fingerprint_unavailable"
  | "clock_issue"
  | "observer_gap"
  | "observer_disagreement"
  | "block_height_missing"
  | "business_action_conflict"
  | "privacy_field_omitted"
  | "source_incomplete";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "DataQualitySeverity".
 */
export type DataQualitySeverity = "info" | "warning" | "error";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "DataQualityImpact".
 */
export type DataQualityImpact =
  | "none"
  | "reduced_completeness"
  | "reduced_certainty"
  | "correlation_unavailable"
  | "expiration_analysis_unavailable"
  | "observation_incomplete";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "BoundedText".
 */
export type BoundedText = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "AttemptId".
 */
export type AttemptId = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "JsonPointer".
 */
export type JsonPointer = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "RouteId".
 */
export type RouteId = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "BlockhashResult".
 */
export type BlockhashResult =
  "acquired" | "not_found" | "rpc_error" | "transport_error" | "malformed_response";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaBlockhash".
 */
export type SolanaBlockhash = string;
/**
 * Solana block height as an exact unsigned decimal string.
 *
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "BlockHeightDecimal".
 */
export type BlockHeightDecimal = string;
/**
 * Solana slot as an exact unsigned decimal string.
 *
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SlotDecimal".
 */
export type SlotDecimal = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "NormalizedErrorCategory".
 */
export type NormalizedErrorCategory =
  | "unknown"
  | "invalid_transaction"
  | "missing_signature"
  | "transaction_too_large"
  | "unsupported_transaction_version"
  | "unsupported_durable_nonce"
  | "blockhash_not_found"
  | "blockhash_expired"
  | "instruction_error"
  | "compute_budget_exceeded"
  | "insufficient_funds"
  | "account_in_use"
  | "slippage_exceeded"
  | "custom_program_error"
  | "signer_rejected"
  | "signer_timeout"
  | "signer_error"
  | "rpc_rejected"
  | "rate_limited"
  | "unauthorized"
  | "duplicate_signature"
  | "already_processed"
  | "transport_timeout"
  | "dns_error"
  | "tls_error"
  | "connection_error"
  | "malformed_response"
  | "cancelled"
  | "observer_unavailable";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaBusinessOutcomeObservedEvent".
 */
export type SolanaBusinessOutcomeObservedEvent = SolanaBusinessOutcomeObservedEvent1 &
  SolanaBusinessOutcomeObservedEvent2;
export type SolanaBusinessOutcomeObservedEvent1 =
  | {
      trace_id: TraceId;
    }
  | {
      business_action_id: BusinessActionId;
    };
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "BusinessOutcome".
 */
export type BusinessOutcome = "success" | "failure" | "timeout" | "cancelled" | "unknown";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "Token".
 */
export type Token = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "OperationId".
 */
export type OperationId = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "ConfirmationWaitResult".
 */
export type ConfirmationWaitResult = "commitment_reached" | "timeout" | "cancelled" | "failed";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "Commitment".
 */
export type Commitment = "processed" | "confirmed" | "finalized";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "ObserverSourceId".
 */
export type ObserverSourceId = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaSignature".
 */
export type SolanaSignature = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "ExecutionResult".
 */
export type ExecutionResult = "success" | "failure";
/**
 * Lamports as an exact unsigned decimal string; never a floating-point SOL value.
 *
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "LamportsDecimal".
 */
export type LamportsDecimal = string;
/**
 * Compute units as an exact unsigned decimal string.
 *
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "ComputeUnitsDecimal".
 */
export type ComputeUnitsDecimal = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SigningResult".
 */
export type SigningResult = "completed" | "rejected" | "timeout" | "failed" | "cancelled";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "TransactionVersion".
 */
export type TransactionVersion = "legacy" | "v0" | "unsupported";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "TransportResult".
 */
export type TransportResult = "response_received" | "timeout" | "connection_failed" | "cancelled";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SimulationRpcResult".
 */
export type SimulationRpcResult =
  | "succeeded"
  | "execution_error"
  | "blockhash_not_found"
  | "node_error"
  | "malformed_response"
  | "not_observed";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "StatusSourceResult".
 */
export type StatusSourceResult = "found" | "not_found" | "unavailable" | "malformed_response";
/**
 * Non-negative confirmation count as an exact unsigned decimal string.
 *
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "ConfirmationCountDecimal".
 */
export type ConfirmationCountDecimal = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SubmissionRpcResult".
 */
export type SubmissionRpcResult =
  | "accepted"
  | "rejected"
  | "blockhash_rejected"
  | "duplicate_signature"
  | "already_processed"
  | "rate_limited"
  | "unauthorized"
  | "malformed_response"
  | "not_observed"
  | "outcome_ambiguous";
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SubmissionEncoding".
 */
export type SubmissionEncoding = "base64" | "base58";
/**
 * Canonical base-10 unsigned integer in the inclusive range 0..18446744073709551615. JSON Schema enforces the lexical form and digit bound; typed semantic parsing enforces the exact upper bound.
 *
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "Uint64Decimal".
 */
export type Uint64Decimal = string;
/**
 * Canonical base-10 signed integer in the inclusive range -9223372036854775808..9223372036854775807. JSON Schema enforces the lexical form and digit bound; typed semantic parsing enforces the exact bounds.
 *
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "Int64Decimal".
 */
export type Int64Decimal = string;
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "Label".
 */
export type Label = string;

export interface EventBatch {
  batch_id: BatchId;
  sent_at: Rfc3339Utc;
  /**
   * @minItems 1
   * @maxItems 100
   */
  events: [WireEvent, ...WireEvent[]];
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "LandfallDataQualityDetectedEvent".
 */
export interface LandfallDataQualityDetectedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "landfall.data_quality.detected";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id?: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    category: DataQualityCategory;
    severity: DataQualitySeverity;
    impact: DataQualityImpact;
    detail?: BoundedText;
    attempt_id?: AttemptId;
    batch_id?: BatchId;
    source_event_id?: EventId;
    field?: JsonPointer;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "EventSource".
 */
export interface EventSource {
  kind: SourceKind;
  name: string;
  version: string;
  instance_id?: UuidV7;
  service?: string;
  app_version?: string;
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaBlockhashAcquiredEvent".
 */
export interface SolanaBlockhashAcquiredEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.blockhash.acquired";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    route_id: RouteId;
    result: BlockhashResult;
    duration_ns: DurationNsDecimal;
    recent_blockhash?: SolanaBlockhash;
    last_valid_block_height?: BlockHeightDecimal;
    context_slot?: SlotDecimal;
    error?: NormalizedError;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "NormalizedError".
 */
export interface NormalizedError {
  category: NormalizedErrorCategory;
  code?: string;
  message?: BoundedText;
  instruction_index?: number;
  custom_code?: number;
}
export interface SolanaBusinessOutcomeObservedEvent2 {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.business_outcome.observed";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id?: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    outcome: BusinessOutcome;
    reason?: Token;
    detail?: BoundedText;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaConfirmationWaitCompletedEvent".
 */
export interface SolanaConfirmationWaitCompletedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.confirmation_wait.completed";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    wait_id: OperationId;
    duration_ns: DurationNsDecimal;
    result: ConfirmationWaitResult;
    observed_commitment?: Commitment;
    error?: NormalizedError;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaConfirmationWaitStartedEvent".
 */
export interface SolanaConfirmationWaitStartedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.confirmation_wait.started";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    wait_id: OperationId;
    commitment: Commitment;
    timeout_ns: DurationNsDecimal;
    search_transaction_history: boolean;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaExecutionEnrichedEvent".
 */
export interface SolanaExecutionEnrichedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.execution.enriched";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    observer_source_id: ObserverSourceId;
    signature?: SolanaSignature;
    slot: SlotDecimal;
    commitment: Commitment;
    execution_result: ExecutionResult;
    block_time?: Rfc3339Utc;
    fee_lamports?: LamportsDecimal;
    compute_units_consumed?: ComputeUnitsDecimal;
    logs_present: boolean;
    error?: NormalizedError;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaSigningCompletedEvent".
 */
export interface SolanaSigningCompletedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.signing.completed";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    signing_id: OperationId;
    duration_ns: DurationNsDecimal;
    result: SigningResult;
    signature?: SolanaSignature;
    signed_bytes_fingerprint?: SignedBytesFingerprint;
    error?: NormalizedError;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SignedBytesFingerprint".
 */
export interface SignedBytesFingerprint {
  algorithm: "lf-hmac-sha256-v1";
  key_id: UuidV7;
  value_hex: string;
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaSigningStartedEvent".
 */
export interface SolanaSigningStartedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.signing.started";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    signing_id: OperationId;
    transaction_version: TransactionVersion;
    required_signatures?: number;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaSimulationCompletedEvent".
 */
export interface SolanaSimulationCompletedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.simulation.completed";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    simulation_id: OperationId;
    route_id: RouteId;
    duration_ns: DurationNsDecimal;
    transport_result: TransportResult;
    rpc_result: SimulationRpcResult;
    units_consumed?: ComputeUnitsDecimal;
    logs_present?: boolean;
    error?: NormalizedError;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaSimulationStartedEvent".
 */
export interface SolanaSimulationStartedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.simulation.started";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    simulation_id: OperationId;
    route_id: RouteId;
    commitment: Commitment;
    replace_recent_blockhash: boolean;
    sig_verify: boolean;
    min_context_slot?: SlotDecimal;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaStatusObservedEvent".
 */
export interface SolanaStatusObservedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.status.observed";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    observer_source_id: ObserverSourceId;
    source_result: StatusSourceResult;
    duration_ns: DurationNsDecimal;
    signature?: SolanaSignature;
    commitment?: Commitment;
    slot?: SlotDecimal;
    confirmations?: ConfirmationCountDecimal;
    block_height?: BlockHeightDecimal;
    error?: NormalizedError;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaSubmissionCompletedEvent".
 */
export interface SolanaSubmissionCompletedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.submission.completed";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    attempt_id: AttemptId;
    route_id: RouteId;
    duration_ns: DurationNsDecimal;
    transport_result: TransportResult;
    rpc_result: SubmissionRpcResult;
    signature?: SolanaSignature;
    route_receipt?: string;
    error?: NormalizedError;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaSubmissionRetryScheduledEvent".
 */
export interface SolanaSubmissionRetryScheduledEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.submission.retry_scheduled";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    previous_attempt_id: AttemptId;
    retry_sequence: number;
    delay_ns: DurationNsDecimal;
    reason: Token;
    next_route_id?: RouteId;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaSubmissionStartedEvent".
 */
export interface SolanaSubmissionStartedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.submission.started";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    attempt_id: AttemptId;
    route_id: RouteId;
    attempt_sequence: number;
    encoding: SubmissionEncoding;
    skip_preflight: boolean;
    preflight_commitment?: Commitment;
    max_retries?: number;
    min_context_slot?: SlotDecimal;
  };
}
/**
 * This interface was referenced by `EventBatch`'s JSON-Schema
 * via the `definition` "SolanaTraceCreatedEvent".
 */
export interface SolanaTraceCreatedEvent {
  schema_version: "1.0";
  event_id: EventId;
  event_type: "solana.trace.created";
  occurred_at: Rfc3339Utc;
  monotonic_ns?: DurationNsDecimal;
  project_id: ProjectId;
  environment_id: EnvironmentId;
  trace_id: TraceId;
  business_action_id?: BusinessActionId;
  source: EventSource;
  privacy_mode: PrivacyMode;
  privacy_policy_version: Version;
  redaction_version: Version;
  attributes: {
    flow: Token;
    transaction_version: TransactionVersion;
    uses_durable_nonce?: boolean;
  };
}
