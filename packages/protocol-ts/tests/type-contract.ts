import type {
  EventBatch,
  SolanaBusinessOutcomeObservedEvent,
  SolanaTraceCreatedEvent,
  WireEvent,
} from "../src/index.js";

const common = {
  schema_version: "1.0",
  event_id: "0198ef00-0000-7000-8000-000000000001",
  occurred_at: "2026-08-29T12:00:00.123Z",
  project_id: "0198ef00-0000-7000-8000-000000000001",
  environment_id: "0198ef00-0000-7000-8000-000000000001",
  trace_id: "0198ef00-0000-7000-8000-000000000001",
  source: {
    kind: "sdk",
    name: "landfall-js",
    version: "0.1.0",
  },
  privacy_mode: "standard",
  privacy_policy_version: "1.0",
  redaction_version: "1.0",
} as const;

const traceCreated = {
  ...common,
  event_type: "solana.trace.created",
  attributes: {
    flow: "swap",
    transaction_version: "v0",
  },
} satisfies SolanaTraceCreatedEvent;

const businessOutcome = {
  ...common,
  event_type: "solana.business_outcome.observed",
  attributes: { outcome: "success" },
} satisfies SolanaBusinessOutcomeObservedEvent;

const batch = {
  batch_id: "0198ef00-0000-7000-8000-000000000001",
  sent_at: "2026-08-29T12:00:00.123Z",
  events: [traceCreated, businessOutcome],
} satisfies EventBatch;

void batch;

const invalidEventType: SolanaTraceCreatedEvent = {
  ...traceCreated,
  // @ts-expect-error The discriminator is closed for this attributes type.
  event_type: "solana.signing.started",
};

const missingAttributes: SolanaTraceCreatedEvent = {
  ...common,
  event_type: "solana.trace.created",
  // @ts-expect-error `attributes` is required by the canonical envelope.
  attributes: undefined,
};

const invalidEnum: SolanaTraceCreatedEvent = {
  ...traceCreated,
  attributes: {
    ...traceCreated.attributes,
    // @ts-expect-error Unknown transaction versions are not widened to string.
    transaction_version: "future_v2",
  },
};

const emptyBatch: EventBatch = {
  batch_id: batch.batch_id,
  sent_at: batch.sent_at,
  // @ts-expect-error A protocol batch contains at least one event.
  events: [],
};

void invalidEventType;
void missingAttributes;
void invalidEnum;
void emptyBatch;

export function eventType(event: WireEvent): WireEvent["event_type"] {
  switch (event.event_type) {
    case "landfall.data_quality.detected":
    case "solana.blockhash.acquired":
    case "solana.business_outcome.observed":
    case "solana.confirmation_wait.completed":
    case "solana.confirmation_wait.started":
    case "solana.execution.enriched":
    case "solana.signing.completed":
    case "solana.signing.started":
    case "solana.simulation.completed":
    case "solana.simulation.started":
    case "solana.status.observed":
    case "solana.submission.completed":
    case "solana.submission.retry_scheduled":
    case "solana.submission.started":
    case "solana.trace.created":
      return event.event_type;
    default: {
      const exhaustive: never = event;
      return exhaustive;
    }
  }
}
