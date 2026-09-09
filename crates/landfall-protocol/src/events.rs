//! Strongly typed envelope, event attributes, event union, and ingestion batch.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser};

use crate::{
    common::{EventSource, NormalizedError, SignedBytesFingerprint},
    enums::{
        BlockhashResult, BusinessOutcome, Commitment, ConfirmationWaitResult, DataQualityCategory,
        DataQualityImpact, DataQualitySeverity, ExecutionResult, PrivacyMode, SigningResult,
        SimulationRpcResult, StatusSourceResult, SubmissionEncoding, SubmissionRpcResult,
        TransactionVersion, TransportResult,
    },
    error::{WireValueError, WireValueErrorKind},
    values::{
        AttemptId, AttemptSequence, BatchId, BlockHeight, BoundedText, BusinessActionId,
        ComputeUnits, ConfirmationCount, DurationNs, EnvironmentId, EventId, JsonPointer, Lamports,
        MaxRetries, ObserverSourceId, OperationId, ProjectId, ProtocolVersion, RequiredSignatures,
        RouteId, RouteReceipt, SchemaVersion, Slot, SolanaBlockhash, SolanaSignature, Token,
        TraceId, UtcTimestamp,
    },
};

mod sealed {
    pub trait Sealed {
        const SCOPE: u8;
    }
}

const TRACE_SCOPE: u8 = 1;
const TRACE_OR_BUSINESS_ACTION_SCOPE: u8 = 2;
const PROJECT_SCOPE: u8 = 3;

/// Behavior shared by the 15 registered v1.0 attributes objects.
///
/// This trait is sealed: downstream crates consume the closed [`WireEvent`]
/// union and cannot add an unregistered raw event type.
pub trait EventAttributes: sealed::Sealed {
    /// Exact `event_type` discriminator registered by the canonical schema.
    const EVENT_TYPE: &'static str;

    #[doc(hidden)]
    fn validate(&self) -> Result<(), WireValueError> {
        Ok(())
    }
}

/// Complete immutable event whose discriminator is encoded by `A`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEnvelope<A> {
    /// Unique idempotency identity of this immutable fact.
    pub event_id: EventId,
    /// Producer-observed UTC wall-clock time.
    pub occurred_at: UtcTimestamp,
    /// Optional producer monotonic-clock reading in nanoseconds.
    pub monotonic_ns: Option<DurationNs>,
    /// Owning project.
    pub project_id: ProjectId,
    /// Owning environment and cluster boundary.
    pub environment_id: EnvironmentId,
    /// Transaction trace identity when required by the event type.
    pub trace_id: Option<TraceId>,
    /// Explicit business-action identity when known.
    pub business_action_id: Option<BusinessActionId>,
    /// Component that directly observed the fact.
    pub source: EventSource,
    /// Privacy policy applied before serialization.
    pub privacy_mode: PrivacyMode,
    /// Environment privacy-policy version.
    pub privacy_policy_version: ProtocolVersion,
    /// Deterministic redaction-pipeline version.
    pub redaction_version: ProtocolVersion,
    /// Closed event-specific payload.
    pub attributes: A,
}

impl<A: EventAttributes> EventEnvelope<A> {
    /// Returns the exact discriminator associated with the attributes type.
    #[must_use]
    pub const fn event_type(&self) -> &'static str {
        A::EVENT_TYPE
    }

    /// Validates relationships that cannot be expressed by an individual scalar type.
    pub fn validate_semantics(&self) -> Result<(), WireValueError> {
        if self.monotonic_ns.is_some() && self.source.instance_id.is_none() {
            return Err(WireValueError::new(
                "source.instance_id",
                WireValueErrorKind::MissingRequiredEvidence,
            ));
        }

        match A::SCOPE {
            TRACE_SCOPE if self.trace_id.is_none() => Err(WireValueError::new(
                "trace_id",
                WireValueErrorKind::MissingRequiredEvidence,
            )),
            TRACE_OR_BUSINESS_ACTION_SCOPE
                if self.trace_id.is_none() && self.business_action_id.is_none() =>
            {
                Err(WireValueError::new(
                    "trace_id|business_action_id",
                    WireValueErrorKind::MissingRequiredEvidence,
                ))
            }
            _ => self.attributes.validate(),
        }
    }
}

#[derive(Serialize)]
struct EnvelopeRef<'a, A> {
    schema_version: SchemaVersion,
    event_id: EventId,
    event_type: &'static str,
    occurred_at: UtcTimestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    monotonic_ns: Option<DurationNs>,
    project_id: ProjectId,
    environment_id: EnvironmentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<TraceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    business_action_id: Option<BusinessActionId>,
    source: &'a EventSource,
    privacy_mode: PrivacyMode,
    privacy_policy_version: &'a ProtocolVersion,
    redaction_version: &'a ProtocolVersion,
    attributes: &'a A,
}

impl<A> Serialize for EventEnvelope<A>
where
    A: EventAttributes + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate_semantics().map_err(ser::Error::custom)?;
        EnvelopeRef {
            schema_version: SchemaVersion,
            event_id: self.event_id,
            event_type: A::EVENT_TYPE,
            occurred_at: self.occurred_at,
            monotonic_ns: self.monotonic_ns,
            project_id: self.project_id,
            environment_id: self.environment_id,
            trace_id: self.trace_id,
            business_action_id: self.business_action_id,
            source: &self.source,
            privacy_mode: self.privacy_mode,
            privacy_policy_version: &self.privacy_policy_version,
            redaction_version: &self.redaction_version,
            attributes: &self.attributes,
        }
        .serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "A: Deserialize<'de>"))]
struct EnvelopeOwned<A> {
    schema_version: SchemaVersion,
    event_id: EventId,
    event_type: String,
    occurred_at: UtcTimestamp,
    monotonic_ns: Option<DurationNs>,
    project_id: ProjectId,
    environment_id: EnvironmentId,
    trace_id: Option<TraceId>,
    business_action_id: Option<BusinessActionId>,
    source: EventSource,
    privacy_mode: PrivacyMode,
    privacy_policy_version: ProtocolVersion,
    redaction_version: ProtocolVersion,
    attributes: A,
}

impl<'de, A> Deserialize<'de> for EventEnvelope<A>
where
    A: Deserialize<'de> + EventAttributes,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EnvelopeOwned::<A>::deserialize(deserializer)?;
        let _ = wire.schema_version;
        if wire.event_type != A::EVENT_TYPE {
            return Err(de::Error::custom(WireValueError::new(
                "event_type",
                WireValueErrorKind::WrongEventType,
            )));
        }
        let event = Self {
            event_id: wire.event_id,
            occurred_at: wire.occurred_at,
            monotonic_ns: wire.monotonic_ns,
            project_id: wire.project_id,
            environment_id: wire.environment_id,
            trace_id: wire.trace_id,
            business_action_id: wire.business_action_id,
            source: wire.source,
            privacy_mode: wire.privacy_mode,
            privacy_policy_version: wire.privacy_policy_version,
            redaction_version: wire.redaction_version,
            attributes: wire.attributes,
        };
        event.validate_semantics().map_err(de::Error::custom)?;
        Ok(event)
    }
}

macro_rules! attributes {
    ($(#[$meta:meta])* $name:ident, $event_type:literal, $scope:ident, { $($fields:tt)* }) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            $($fields)*
        }

        impl sealed::Sealed for $name {
            const SCOPE: u8 = scope_value!($scope);
        }

        impl EventAttributes for $name {
            const EVENT_TYPE: &'static str = $event_type;
        }
    };
}

macro_rules! scope_value {
    (Trace) => {
        TRACE_SCOPE
    };
    (TraceOrBusinessAction) => {
        TRACE_OR_BUSINESS_ACTION_SCOPE
    };
    (Project) => {
        PROJECT_SCOPE
    };
}

attributes!(
    /// Attributes recorded when a transaction trace is created.
    TraceCreatedAttributes,
    "solana.trace.created",
    Trace,
    {
        /// Customer-defined bounded flow name.
        pub flow: Token,
        /// Parsed transaction version.
        pub transaction_version: TransactionVersion,
        /// Whether the transaction uses a durable nonce when known.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub uses_durable_nonce: Option<bool>,
    }
);

attributes!(
    /// Attributes of a recent-blockhash acquisition operation.
    BlockhashAcquiredAttributes,
    "solana.blockhash.acquired",
    Trace,
    {
        /// RPC route used for the request.
        pub route_id: RouteId,
        /// Operation result.
        pub result: BlockhashResult,
        /// Total operation duration.
        pub duration_ns: DurationNs,
        /// Acquired recent blockhash when available and permitted.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub recent_blockhash: Option<SolanaBlockhash>,
        /// Last valid block height returned with the blockhash.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_valid_block_height: Option<BlockHeight>,
        /// RPC context slot.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub context_slot: Option<Slot>,
        /// Normalized failure evidence.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<NormalizedError>,
    }
);

attributes!(
    /// Attributes recorded before transaction simulation.
    SimulationStartedAttributes,
    "solana.simulation.started",
    Trace,
    {
        /// Identity shared with the completion event.
        pub simulation_id: OperationId,
        /// RPC route selected for simulation.
        pub route_id: RouteId,
        /// Requested commitment.
        pub commitment: Commitment,
        /// Whether RPC should replace the transaction's recent blockhash.
        pub replace_recent_blockhash: bool,
        /// Whether RPC should verify transaction signatures.
        pub sig_verify: bool,
        /// Optional minimum acceptable context slot.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub min_context_slot: Option<Slot>,
    }
);

/// Attributes recorded after transaction simulation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationCompletedAttributes {
    /// Identity shared with the start event.
    pub simulation_id: OperationId,
    /// RPC route used for simulation.
    pub route_id: RouteId,
    /// Total operation duration.
    pub duration_ns: DurationNs,
    /// Network transport result.
    pub transport_result: TransportResult,
    /// Simulation RPC result.
    pub rpc_result: SimulationRpcResult,
    /// Compute units reported by simulation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units_consumed: Option<ComputeUnits>,
    /// Whether logs were present without retaining the logs themselves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs_present: Option<bool>,
    /// Normalized failure evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<NormalizedError>,
}

impl sealed::Sealed for SimulationCompletedAttributes {
    const SCOPE: u8 = TRACE_SCOPE;
}

impl EventAttributes for SimulationCompletedAttributes {
    const EVENT_TYPE: &'static str = "solana.simulation.completed";

    fn validate(&self) -> Result<(), WireValueError> {
        match (self.transport_result, self.rpc_result) {
            (TransportResult::ResponseReceived, SimulationRpcResult::NotObserved)
            | (
                TransportResult::Timeout
                | TransportResult::ConnectionFailed
                | TransportResult::Cancelled,
                SimulationRpcResult::Succeeded
                | SimulationRpcResult::ExecutionError
                | SimulationRpcResult::BlockhashNotFound
                | SimulationRpcResult::NodeError
                | SimulationRpcResult::MalformedResponse,
            ) => contradictory("transport_result|rpc_result"),
            _ if self.rpc_result == SimulationRpcResult::Succeeded && self.error.is_some() => {
                contradictory("rpc_result|error")
            }
            _ => Ok(()),
        }
    }
}

attributes!(
    /// Attributes recorded before a signing operation.
    SigningStartedAttributes,
    "solana.signing.started",
    Trace,
    {
        /// Identity shared with the completion event.
        pub signing_id: OperationId,
        /// Transaction version being signed.
        pub transaction_version: TransactionVersion,
        /// Number of required signatures when known.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub required_signatures: Option<RequiredSignatures>,
    }
);

/// Attributes recorded after a signing operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SigningCompletedAttributes {
    /// Identity shared with the start event.
    pub signing_id: OperationId,
    /// Total signing duration.
    pub duration_ns: DurationNs,
    /// Signing result.
    pub result: SigningResult,
    /// Public transaction signature when permitted and available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<SolanaSignature>,
    /// Privacy-preserving fingerprint of exact signed transaction bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_bytes_fingerprint: Option<SignedBytesFingerprint>,
    /// Normalized failure evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<NormalizedError>,
}

impl sealed::Sealed for SigningCompletedAttributes {
    const SCOPE: u8 = TRACE_SCOPE;
}

impl EventAttributes for SigningCompletedAttributes {
    const EVENT_TYPE: &'static str = "solana.signing.completed";

    fn validate(&self) -> Result<(), WireValueError> {
        if self.result == SigningResult::Completed && self.error.is_some() {
            contradictory("result|error")
        } else {
            Ok(())
        }
    }
}

attributes!(
    /// Attributes recorded immediately before a real RPC submission.
    SubmissionStartedAttributes,
    "solana.submission.started",
    Trace,
    {
        /// Identity of this real submission attempt.
        pub attempt_id: AttemptId,
        /// RPC route selected for submission.
        pub route_id: RouteId,
        /// One-based attempt number within the trace.
        pub attempt_sequence: AttemptSequence,
        /// Submission payload encoding.
        pub encoding: SubmissionEncoding,
        /// Whether RPC preflight was disabled.
        pub skip_preflight: bool,
        /// Requested preflight commitment when configured.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub preflight_commitment: Option<Commitment>,
        /// RPC-level retry limit when explicitly configured.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub max_retries: Option<MaxRetries>,
        /// Optional minimum acceptable context slot.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub min_context_slot: Option<Slot>,
    }
);

/// Attributes recorded when one submission call completes locally.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubmissionCompletedAttributes {
    /// Identity of the corresponding real attempt.
    pub attempt_id: AttemptId,
    /// RPC route used for submission.
    pub route_id: RouteId,
    /// Total submission-call duration.
    pub duration_ns: DurationNs,
    /// Network transport result.
    pub transport_result: TransportResult,
    /// Submission RPC result.
    pub rpc_result: SubmissionRpcResult,
    /// Public signature when returned, known, and permitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<SolanaSignature>,
    /// Bounded opaque route receipt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_receipt: Option<RouteReceipt>,
    /// Normalized failure evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<NormalizedError>,
}

impl sealed::Sealed for SubmissionCompletedAttributes {
    const SCOPE: u8 = TRACE_SCOPE;
}

impl EventAttributes for SubmissionCompletedAttributes {
    const EVENT_TYPE: &'static str = "solana.submission.completed";

    fn validate(&self) -> Result<(), WireValueError> {
        match (self.transport_result, self.rpc_result) {
            (TransportResult::ResponseReceived, SubmissionRpcResult::NotObserved)
            | (
                TransportResult::Timeout
                | TransportResult::ConnectionFailed
                | TransportResult::Cancelled,
                SubmissionRpcResult::Accepted
                | SubmissionRpcResult::Rejected
                | SubmissionRpcResult::BlockhashRejected
                | SubmissionRpcResult::DuplicateSignature
                | SubmissionRpcResult::AlreadyProcessed
                | SubmissionRpcResult::RateLimited
                | SubmissionRpcResult::Unauthorized
                | SubmissionRpcResult::MalformedResponse,
            ) => contradictory("transport_result|rpc_result"),
            _ if self.rpc_result == SubmissionRpcResult::Accepted && self.error.is_some() => {
                contradictory("rpc_result|error")
            }
            _ => Ok(()),
        }
    }
}

attributes!(
    /// Attributes recording retry intent without inventing a new attempt.
    SubmissionRetryScheduledAttributes,
    "solana.submission.retry_scheduled",
    Trace,
    {
        /// Attempt whose result caused the retry decision.
        pub previous_attempt_id: AttemptId,
        /// One-based retry number.
        pub retry_sequence: AttemptSequence,
        /// Planned delay before the next submission.
        pub delay_ns: DurationNs,
        /// Bounded application/SDK reason token.
        pub reason: Token,
        /// Next route when already selected.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub next_route_id: Option<RouteId>,
    }
);

attributes!(
    /// Attributes recorded when bounded confirmation waiting begins.
    ConfirmationWaitStartedAttributes,
    "solana.confirmation_wait.started",
    Trace,
    {
        /// Identity shared with the completion event.
        pub wait_id: OperationId,
        /// Target commitment.
        pub commitment: Commitment,
        /// Configured wait timeout.
        pub timeout_ns: DurationNs,
        /// Whether historical transaction status may be queried.
        pub search_transaction_history: bool,
    }
);

/// Attributes recorded when bounded confirmation waiting ends.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationWaitCompletedAttributes {
    /// Identity shared with the start event.
    pub wait_id: OperationId,
    /// Total wait duration.
    pub duration_ns: DurationNs,
    /// Wait result.
    pub result: ConfirmationWaitResult,
    /// Commitment actually observed before returning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_commitment: Option<Commitment>,
    /// Normalized failure evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<NormalizedError>,
}

impl sealed::Sealed for ConfirmationWaitCompletedAttributes {
    const SCOPE: u8 = TRACE_SCOPE;
}

impl EventAttributes for ConfirmationWaitCompletedAttributes {
    const EVENT_TYPE: &'static str = "solana.confirmation_wait.completed";

    fn validate(&self) -> Result<(), WireValueError> {
        if self.result == ConfirmationWaitResult::CommitmentReached
            && self.observed_commitment.is_none()
        {
            Err(WireValueError::new(
                "observed_commitment",
                WireValueErrorKind::MissingRequiredEvidence,
            ))
        } else {
            Ok(())
        }
    }
}

attributes!(
    /// Evidence returned by one configured status observer.
    StatusObservedAttributes,
    "solana.status.observed",
    Trace,
    {
        /// Configured observer source.
        pub observer_source_id: ObserverSourceId,
        /// Result of querying that source.
        pub source_result: StatusSourceResult,
        /// Total query duration.
        pub duration_ns: DurationNs,
        /// Public signature when permitted.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub signature: Option<SolanaSignature>,
        /// Observed commitment.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub commitment: Option<Commitment>,
        /// Observed slot.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub slot: Option<Slot>,
        /// Remaining confirmations when supplied by RPC.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub confirmations: Option<ConfirmationCount>,
        /// Observer's current block height when captured.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub block_height: Option<BlockHeight>,
        /// Normalized failure evidence.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<NormalizedError>,
    }
);

/// Attributes enriched from an observed on-chain transaction.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEnrichedAttributes {
    /// Configured observer source used for enrichment.
    pub observer_source_id: ObserverSourceId,
    /// Public signature when permitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<SolanaSignature>,
    /// Execution slot.
    pub slot: Slot,
    /// Commitment of the enrichment evidence.
    pub commitment: Commitment,
    /// On-chain execution result.
    pub execution_result: ExecutionResult,
    /// Block time converted to a UTC timestamp when RPC supplied it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_time: Option<UtcTimestamp>,
    /// Transaction fee in lamports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_lamports: Option<Lamports>,
    /// Compute units consumed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_units_consumed: Option<ComputeUnits>,
    /// Whether transaction logs were present without retaining their content.
    pub logs_present: bool,
    /// Normalized on-chain failure evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<NormalizedError>,
}

impl sealed::Sealed for ExecutionEnrichedAttributes {
    const SCOPE: u8 = TRACE_SCOPE;
}

impl EventAttributes for ExecutionEnrichedAttributes {
    const EVENT_TYPE: &'static str = "solana.execution.enriched";

    fn validate(&self) -> Result<(), WireValueError> {
        if self.execution_result == ExecutionResult::Success && self.error.is_some() {
            contradictory("execution_result|error")
        } else {
            Ok(())
        }
    }
}

attributes!(
    /// Application-observed outcome attributes.
    BusinessOutcomeObservedAttributes,
    "solana.business_outcome.observed",
    TraceOrBusinessAction,
    {
        /// Application's outcome classification.
        pub outcome: BusinessOutcome,
        /// Optional bounded machine-readable reason.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub reason: Option<Token>,
        /// Optional bounded, redacted explanation.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub detail: Option<BoundedText>,
    }
);

attributes!(
    /// Explicit evidence-quality condition and its analytical impact.
    DataQualityDetectedAttributes,
    "landfall.data_quality.detected",
    Project,
    {
        /// Stable condition category.
        pub category: DataQualityCategory,
        /// Priority assigned by the emitting policy.
        pub severity: DataQualitySeverity,
        /// Analytical consequence.
        pub impact: DataQualityImpact,
        /// Optional bounded, redacted explanation.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub detail: Option<BoundedText>,
        /// Related attempt when applicable.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub attempt_id: Option<AttemptId>,
        /// Related ingestion batch when applicable.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub batch_id: Option<BatchId>,
        /// Related immutable source event when applicable.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub source_event_id: Option<EventId>,
        /// Related field represented as a bounded JSON Pointer.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub field: Option<JsonPointer>,
    }
);

fn contradictory(field: &'static str) -> Result<(), WireValueError> {
    Err(WireValueError::new(
        field,
        WireValueErrorKind::ContradictoryEvidence,
    ))
}

/// Typed `solana.trace.created` event.
pub type TraceCreatedEvent = EventEnvelope<TraceCreatedAttributes>;
/// Typed `solana.blockhash.acquired` event.
pub type BlockhashAcquiredEvent = EventEnvelope<BlockhashAcquiredAttributes>;
/// Typed `solana.simulation.started` event.
pub type SimulationStartedEvent = EventEnvelope<SimulationStartedAttributes>;
/// Typed `solana.simulation.completed` event.
pub type SimulationCompletedEvent = EventEnvelope<SimulationCompletedAttributes>;
/// Typed `solana.signing.started` event.
pub type SigningStartedEvent = EventEnvelope<SigningStartedAttributes>;
/// Typed `solana.signing.completed` event.
pub type SigningCompletedEvent = EventEnvelope<SigningCompletedAttributes>;
/// Typed `solana.submission.started` event.
pub type SubmissionStartedEvent = EventEnvelope<SubmissionStartedAttributes>;
/// Typed `solana.submission.completed` event.
pub type SubmissionCompletedEvent = EventEnvelope<SubmissionCompletedAttributes>;
/// Typed `solana.submission.retry_scheduled` event.
pub type SubmissionRetryScheduledEvent = EventEnvelope<SubmissionRetryScheduledAttributes>;
/// Typed `solana.confirmation_wait.started` event.
pub type ConfirmationWaitStartedEvent = EventEnvelope<ConfirmationWaitStartedAttributes>;
/// Typed `solana.confirmation_wait.completed` event.
pub type ConfirmationWaitCompletedEvent = EventEnvelope<ConfirmationWaitCompletedAttributes>;
/// Typed `solana.status.observed` event.
pub type StatusObservedEvent = EventEnvelope<StatusObservedAttributes>;
/// Typed `solana.execution.enriched` event.
pub type ExecutionEnrichedEvent = EventEnvelope<ExecutionEnrichedAttributes>;
/// Typed `solana.business_outcome.observed` event.
pub type BusinessOutcomeObservedEvent = EventEnvelope<BusinessOutcomeObservedAttributes>;
/// Typed `landfall.data_quality.detected` event.
pub type DataQualityDetectedEvent = EventEnvelope<DataQualityDetectedAttributes>;

/// Closed union of all raw event types accepted by wire version 1.0.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum WireEvent {
    /// Trace creation evidence.
    TraceCreated(TraceCreatedEvent),
    /// Recent-blockhash acquisition evidence.
    BlockhashAcquired(BlockhashAcquiredEvent),
    /// Simulation-start evidence.
    SimulationStarted(SimulationStartedEvent),
    /// Simulation-completion evidence.
    SimulationCompleted(SimulationCompletedEvent),
    /// Signing-start evidence.
    SigningStarted(SigningStartedEvent),
    /// Signing-completion evidence.
    SigningCompleted(SigningCompletedEvent),
    /// Submission-start evidence.
    SubmissionStarted(SubmissionStartedEvent),
    /// Submission-completion evidence.
    SubmissionCompleted(SubmissionCompletedEvent),
    /// Retry-scheduling evidence.
    SubmissionRetryScheduled(SubmissionRetryScheduledEvent),
    /// Confirmation-wait start evidence.
    ConfirmationWaitStarted(ConfirmationWaitStartedEvent),
    /// Confirmation-wait completion evidence.
    ConfirmationWaitCompleted(ConfirmationWaitCompletedEvent),
    /// Observer status evidence.
    StatusObserved(StatusObservedEvent),
    /// On-chain execution enrichment evidence.
    ExecutionEnriched(ExecutionEnrichedEvent),
    /// Application-observed business outcome evidence.
    BusinessOutcomeObserved(BusinessOutcomeObservedEvent),
    /// Data-quality evidence.
    DataQualityDetected(DataQualityDetectedEvent),
}

/// Bounded ingestion batch containing between 1 and 100 events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventBatch {
    /// Idempotency identity of the transport batch.
    pub batch_id: BatchId,
    /// Producer time at which this batch was sent.
    pub sent_at: UtcTimestamp,
    /// Ordered immutable event documents.
    pub events: Vec<WireEvent>,
}

impl EventBatch {
    /// Creates a batch after enforcing the protocol collection bound.
    pub fn new(
        batch_id: BatchId,
        sent_at: UtcTimestamp,
        events: Vec<WireEvent>,
    ) -> Result<Self, WireValueError> {
        validate_batch_length(&events)?;
        Ok(Self {
            batch_id,
            sent_at,
            events,
        })
    }
}

fn validate_batch_length(events: &[WireEvent]) -> Result<(), WireValueError> {
    if (1..=100).contains(&events.len()) {
        Ok(())
    } else {
        Err(WireValueError::new(
            "events",
            WireValueErrorKind::OutOfRange,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchOwned {
    batch_id: BatchId,
    sent_at: UtcTimestamp,
    events: Vec<WireEvent>,
}

#[derive(Serialize)]
struct BatchRef<'a> {
    batch_id: BatchId,
    sent_at: UtcTimestamp,
    events: &'a [WireEvent],
}

impl Serialize for EventBatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        validate_batch_length(&self.events).map_err(ser::Error::custom)?;
        BatchRef {
            batch_id: self.batch_id,
            sent_at: self.sent_at,
            events: &self.events,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EventBatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BatchOwned::deserialize(deserializer)?;
        Self::new(wire.batch_id, wire.sent_at, wire.events).map_err(de::Error::custom)
    }
}
