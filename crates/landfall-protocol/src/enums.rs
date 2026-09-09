//! Closed protocol-controlled vocabularies for wire version 1.0.

use serde::{Deserialize, Serialize};

macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$variant_meta:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($(#[$variant_meta])* #[doc = concat!("Registered `", stringify!($variant), "` wire variant.")] $variant),+
        }

        impl $name {
            /// All registered values in canonical schema order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }
    };
}

wire_enum!(
    /// Class of component that directly observed an event.
    SourceKind { Sdk, Application, Observer, Collector, Cli }
);

wire_enum!(
    /// Privacy policy applied before event serialization.
    PrivacyMode { Standard, Full, Strict }
);

wire_enum!(
    /// Solana commitment requested or observed by the surrounding field.
    Commitment { Processed, Confirmed, Finalized }
);

wire_enum!(
    /// Parsed Solana transaction wire version.
    TransactionVersion { Legacy, V0, Unsupported }
);

wire_enum!(
    /// Encoding used for a transaction submission payload.
    SubmissionEncoding { Base64, Base58 }
);

wire_enum!(
    /// Outcome of the network transport independently of the RPC operation.
    TransportResult { ResponseReceived, Timeout, ConnectionFailed, Cancelled }
);

wire_enum!(
    /// Result of acquiring a recent blockhash.
    BlockhashResult { Acquired, NotFound, RpcError, TransportError, MalformedResponse }
);

wire_enum!(
    /// RPC-level result of transaction simulation.
    SimulationRpcResult {
        Succeeded,
        ExecutionError,
        BlockhashNotFound,
        NodeError,
        MalformedResponse,
        NotObserved,
    }
);

wire_enum!(
    /// Result of a signing operation.
    SigningResult { Completed, Rejected, Timeout, Failed, Cancelled }
);

wire_enum!(
    /// RPC-level result of transaction submission.
    SubmissionRpcResult {
        Accepted,
        Rejected,
        BlockhashRejected,
        DuplicateSignature,
        AlreadyProcessed,
        RateLimited,
        Unauthorized,
        MalformedResponse,
        NotObserved,
        OutcomeAmbiguous,
    }
);

wire_enum!(
    /// Result of a bounded confirmation-wait operation.
    ConfirmationWaitResult { CommitmentReached, Timeout, Cancelled, Failed }
);

wire_enum!(
    /// Result returned by one configured status-observation source.
    StatusSourceResult { Found, NotFound, Unavailable, MalformedResponse }
);

wire_enum!(
    /// On-chain execution result.
    ExecutionResult { Success, Failure }
);

wire_enum!(
    /// Application-observed business outcome.
    BusinessOutcome { Success, Failure, Timeout, Cancelled, Unknown }
);

wire_enum!(
    /// Stable category of incomplete or contradictory evidence.
    DataQualityCategory {
        TelemetryDropped,
        UnsupportedTransactionVersion,
        UnsupportedDurableNonce,
        FingerprintUnavailable,
        ClockIssue,
        ObserverGap,
        ObserverDisagreement,
        BlockHeightMissing,
        BusinessActionConflict,
        PrivacyFieldOmitted,
        SourceIncomplete,
    }
);

wire_enum!(
    /// Priority of a data-quality condition.
    DataQualitySeverity { Info, Warning, Error }
);

wire_enum!(
    /// Analytical consequence of a data-quality condition.
    DataQualityImpact {
        None,
        ReducedCompleteness,
        ReducedCertainty,
        CorrelationUnavailable,
        ExpirationAnalysisUnavailable,
        ObservationIncomplete,
    }
);

wire_enum!(
    /// Stable normalized category for bounded, redacted source errors.
    NormalizedErrorCategory {
        Unknown,
        InvalidTransaction,
        MissingSignature,
        TransactionTooLarge,
        UnsupportedTransactionVersion,
        UnsupportedDurableNonce,
        BlockhashNotFound,
        BlockhashExpired,
        InstructionError,
        ComputeBudgetExceeded,
        InsufficientFunds,
        AccountInUse,
        SlippageExceeded,
        CustomProgramError,
        SignerRejected,
        SignerTimeout,
        SignerError,
        RpcRejected,
        RateLimited,
        Unauthorized,
        DuplicateSignature,
        AlreadyProcessed,
        TransportTimeout,
        DnsError,
        TlsError,
        ConnectionError,
        MalformedResponse,
        Cancelled,
        ObserverUnavailable,
    }
);
