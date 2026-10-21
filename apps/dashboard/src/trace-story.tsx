import * as React from "react";
import type {
  TraceDiagnostic,
  TraceTimeline,
  TraceTimelineAttributes,
  TraceTimelineEvent,
} from "@landfall/api-client";

/** Lifecycle phase a timeline event belongs to. */
export type Phase = "prepare" | "sign" | "submit" | "confirm" | "observe" | "outcome";

const PHASES: ReadonlyArray<{ id: Phase; title: string; empty: string }> = [
  { id: "prepare", title: "Prepare", empty: "No blockhash or simulation evidence was reported." },
  { id: "sign", title: "Sign", empty: "No signing evidence was reported." },
  { id: "submit", title: "Submit & retry", empty: "The transaction was not submitted." },
  { id: "confirm", title: "Confirm", empty: "The application did not report a confirmation wait." },
  { id: "observe", title: "Observe on-chain", empty: "No on-chain status was observed." },
  { id: "outcome", title: "Outcome", empty: "No business outcome was reported." },
];

const PHASE_BY_TYPE: Readonly<Record<string, Phase>> = {
  "solana.trace.created": "prepare",
  "solana.blockhash.acquired": "prepare",
  "solana.simulation.started": "prepare",
  "solana.simulation.completed": "prepare",
  "solana.signing.started": "sign",
  "solana.signing.completed": "sign",
  "solana.submission.started": "submit",
  "solana.submission.completed": "submit",
  "solana.submission.retry_scheduled": "submit",
  "solana.confirmation_wait.started": "confirm",
  "solana.confirmation_wait.completed": "confirm",
  "solana.status.observed": "observe",
  "solana.execution.enriched": "observe",
  "solana.business_outcome.observed": "outcome",
  "landfall.data_quality.detected": "outcome",
};

/** Plain-language copy for each diagnostic claim the rule set can produce. */
const DIAGNOSIS_COPY: Readonly<Record<string, { title: string; detail: string }>> = {
  missing_evidence: {
    title: "Missing lifecycle evidence",
    detail: "Landfall cannot establish the transaction outcome from the evidence received so far.",
  },
  simulation_error: {
    title: "Simulation failed",
    detail: "The RPC simulated the transaction and reported an error, so it was never sent.",
  },
  rpc_submission_rejection: {
    title: "RPC rejected the submission",
    detail: "The route answered the send request with a rejection; the transaction did not land.",
  },
  on_chain_execution_error: {
    title: "Transaction failed on-chain",
    detail: "The transaction landed and paid its fee, but an instruction returned an error.",
  },
  compute_budget_failure: {
    title: "Ran out of compute units",
    detail: "Execution exceeded the compute-unit limit set on the transaction.",
  },
  low_compute_headroom: {
    title: "Compute limit nearly exhausted",
    detail: "The transaction succeeded but used most of its compute-unit limit.",
  },
  expired_without_observed_inclusion: {
    title: "Blockhash expired before landing",
    detail:
      "The blockhash's last valid block height passed without the transaction being observed.",
  },
  excessive_signing_delay: {
    title: "Signing took too long",
    detail: "Time spent in the signer consumed a large share of the blockhash validity window.",
  },
  client_timeout_followed_by_network_success: {
    title: "Client gave up, but the transaction landed",
    detail: "The application stopped waiting before the network confirmed the transaction.",
  },
  unsafe_redundant_retry: {
    title: "Retries may have re-sent the same transaction",
    detail:
      "Several attempts carried the same signed transaction without an observed outcome in between.",
  },
  route_degradation_signal: {
    title: "RPC route looked degraded",
    detail: "Attempts on this route timed out or failed while another route succeeded.",
  },
};

/** Explains the specific gap behind a RULE-UNKNOWN-001 finding. */
const UNKNOWN_REASON_COPY: Readonly<Record<string, string>> = {
  missing_trace_created:
    "The trace-created event was not received, so the trace has no recorded starting point.",
  missing_signing_evidence:
    "Later lifecycle stages were reported, but no signing evidence was received.",
  missing_submission_evidence:
    "Later lifecycle stages were reported, but no submission to an RPC route was received.",
  missing_submission_response: "A submission started, but its RPC response was never recorded.",
  missing_last_valid_block_height:
    "The blockhash validity boundary is missing, so expiry cannot be determined.",
  missing_observer_coverage:
    "The transaction was submitted, but no observer has reported its on-chain status yet.",
  missing_execution_metadata:
    "The transaction was included, but its execution result has not been captured.",
  missing_simulation_evidence:
    "No simulation evidence was received, so pre-flight failures cannot be ruled out.",
  missing_signed_identity:
    "The transaction signature is unavailable, so network observations cannot be correlated.",
  missing_business_action_correlation:
    "No business action is linked to this trace, so its application outcome cannot be correlated.",
  clock_quality_issue: "Event clocks are uncertain, so the lifecycle order may be inexact.",
  observer_disagreement: "Observers reported conflicting results for this transaction.",
};

export function diagnosisCopy(
  claimKey: string,
  unknownReason?: string | null,
): { title: string; detail: string } {
  if (claimKey === "missing_evidence") {
    return {
      title: "Missing lifecycle evidence",
      detail:
        (unknownReason ? UNKNOWN_REASON_COPY[unknownReason] : undefined) ??
        "Landfall cannot establish the transaction outcome from the evidence received so far.",
    };
  }
  return (
    DIAGNOSIS_COPY[claimKey] ?? {
      title: claimKey.replaceAll("_", " "),
      detail: "Review this diagnostic together with the captured lifecycle evidence.",
    }
  );
}

/** Diagnoses collapsed by rule, with evidence from every evaluation of that rule. */
export interface DiagnosisGroup {
  rule_id: string;
  claim_key: string;
  certainty: TraceDiagnostic["certainty"];
  unknown_reason: string | null;
  evidence_event_ids: string[];
}

const CERTAINTY_RANK: Readonly<Record<string, number>> = { confirmed: 0, probable: 1, unknown: 2 };

export function groupDiagnoses(diagnostics: readonly TraceDiagnostic[]): DiagnosisGroup[] {
  const groups = new Map<string, DiagnosisGroup>();
  for (const finding of diagnostics) {
    const key = `${finding.rule_id}:${finding.certainty}:${finding.unknown_reason ?? ""}`;
    const existing = groups.get(key);
    const evidence = finding.evidence_event_ids;
    if (existing) {
      existing.evidence_event_ids = [...new Set([...existing.evidence_event_ids, ...evidence])];
    } else {
      groups.set(key, {
        rule_id: finding.rule_id,
        claim_key: finding.claim_key,
        certainty: finding.certainty,
        unknown_reason: finding.unknown_reason,
        evidence_event_ids: [...new Set(evidence)],
      });
    }
  }
  return [...groups.values()].sort(
    (left, right) =>
      (CERTAINTY_RANK[left.certainty] ?? 3) - (CERTAINTY_RANK[right.certainty] ?? 3) ||
      left.rule_id.localeCompare(right.rule_id),
  );
}

function nanosToMs(value: string | undefined): number | null {
  if (value === undefined) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed / 1_000_000 : null;
}

/** Formats a duration in milliseconds as "820 ms", "1.50 s", "21.0 s", or "2 m 05 s". */
export function formatDuration(ms: number): string {
  if (ms < 1) return `${ms.toFixed(2)} ms`;
  if (ms < 10) return `${ms.toFixed(1)} ms`;
  if (ms < 1000) return `${Math.round(ms)} ms`;
  const seconds = ms / 1000;
  if (seconds < 10) return `${seconds.toFixed(2)} s`;
  if (seconds < 60) return `${seconds.toFixed(1)} s`;
  const minutes = Math.floor(seconds / 60);
  return `${minutes} m ${String(Math.round(seconds % 60)).padStart(2, "0")} s`;
}

function took(attributes: TraceTimelineAttributes): string | null {
  const ms = nanosToMs(attributes.duration_ns);
  return ms === null || ms === 0 ? null : `took ${formatDuration(ms)}`;
}

function shortId(value: string): string {
  return value.length > 12 ? `${value.slice(0, 8)}…${value.slice(-4)}` : value;
}

function routeLabel(name: string | undefined, id: string | undefined): string | null {
  if (name) return name;
  if (id) return `unregistered route ${shortId(id)}`;
  return null;
}

function words(value: string): string {
  return value.replaceAll("_", " ");
}

function errorText(attributes: TraceTimelineAttributes): string | null {
  const error = attributes.error;
  if (!error) return null;
  const parts = [words(error.category)];
  if (error.code) parts.push(error.code);
  if (error.instruction_index !== undefined) parts.push(`instruction ${error.instruction_index}`);
  if (error.custom_code !== undefined) parts.push(`custom code ${error.custom_code}`);
  return parts.join(" · ");
}

function compact(parts: ReadonlyArray<string | null | undefined | false>): string {
  return parts
    .filter((part): part is string => typeof part === "string" && part !== "")
    .join(" · ");
}

/** Human label, detail line, and failure flag for one event. */
export interface EventStory {
  label: string;
  detail: string;
  failed: boolean;
}

export function describeEvent(event: TraceTimelineEvent): EventStory {
  const a = event.attributes;
  const error = errorText(a);
  switch (event.event_type) {
    case "solana.trace.created":
      return {
        label: a.flow ? `Trace started for ${a.flow}` : "Trace started",
        detail: compact([a.transaction_version && `${a.transaction_version} transaction`]),
        failed: false,
      };
    case "solana.blockhash.acquired": {
      const failed = a.result !== undefined && a.result !== "acquired";
      return {
        label: failed ? `Blockhash request failed: ${words(a.result ?? "")}` : "Blockhash acquired",
        detail: compact([
          a.context_slot && `slot ${a.context_slot}`,
          a.last_valid_block_height && `valid until block height ${a.last_valid_block_height}`,
          routeLabel(a.route_name, a.route_id),
          took(a),
          error,
        ]),
        failed,
      };
    }
    case "solana.simulation.started":
      return {
        label: "Simulation started",
        detail: compact([
          a.commitment && `at ${a.commitment}`,
          routeLabel(a.route_name, a.route_id),
        ]),
        failed: false,
      };
    case "solana.simulation.completed": {
      const failed =
        (a.rpc_result !== undefined && a.rpc_result !== "succeeded") ||
        (a.transport_result !== undefined && a.transport_result !== "response_received");
      return {
        label: failed
          ? `Simulation failed: ${words(a.rpc_result === "not_observed" ? (a.transport_result ?? "") : (a.rpc_result ?? ""))}`
          : "Simulation succeeded",
        detail: compact([
          a.units_consumed && `${Number(a.units_consumed).toLocaleString()} compute units`,
          took(a),
          error,
        ]),
        failed,
      };
    }
    case "solana.signing.started":
      return {
        label: "Signing requested",
        detail: compact([
          a.required_signatures !== undefined &&
            `${a.required_signatures} signature${a.required_signatures === 1 ? "" : "s"} required`,
        ]),
        failed: false,
      };
    case "solana.signing.completed": {
      const failed = a.result !== undefined && a.result !== "completed";
      const duration = nanosToMs(a.duration_ns);
      return {
        label: failed
          ? `Signing ${words(a.result ?? "failed")}`
          : duration
            ? `Signed in ${formatDuration(duration)}`
            : "Signed",
        detail: compact([error]),
        failed,
      };
    }
    case "solana.submission.started":
      return {
        label: `Attempt ${a.attempt_sequence ?? "?"} sent`,
        detail: compact([
          routeLabel(a.route_name, a.route_id) && `via ${routeLabel(a.route_name, a.route_id)}`,
          a.skip_preflight === true
            ? "preflight skipped"
            : a.preflight_commitment && `preflight at ${a.preflight_commitment}`,
          a.max_retries !== undefined && `RPC max retries ${a.max_retries}`,
        ]),
        failed: false,
      };
    case "solana.submission.completed": {
      const transportFailed =
        a.transport_result !== undefined && a.transport_result !== "response_received";
      const rejected = a.rpc_result !== undefined && a.rpc_result !== "accepted";
      const duration = nanosToMs(a.duration_ns);
      let label = "RPC accepted the transaction";
      if (transportFailed)
        label = `${a.transport_result === "timeout" ? "Timed out" : `Transport ${words(a.transport_result ?? "")}`}${duration ? ` after ${formatDuration(duration)}` : ""}`;
      else if (rejected) label = `RPC answered ${words(a.rpc_result ?? "")}`;
      return {
        label,
        detail: compact([!transportFailed && took(a), error]),
        failed: transportFailed || rejected,
      };
    }
    case "solana.submission.retry_scheduled": {
      const delay = nanosToMs(a.delay_ns);
      return {
        label:
          `Retry ${a.retry_sequence ?? ""} scheduled${delay ? ` in ${formatDuration(delay)}` : ""}`.replace(
            "  ",
            " ",
          ),
        detail: compact([
          a.reason && `because of ${words(a.reason)}`,
          routeLabel(a.next_route_name, a.next_route_id) &&
            `next on ${routeLabel(a.next_route_name, a.next_route_id)}`,
        ]),
        failed: false,
      };
    }
    case "solana.confirmation_wait.started": {
      const timeout = nanosToMs(a.timeout_ns);
      return {
        label: `Waiting for ${a.commitment ?? "confirmation"}`,
        detail: compact([timeout !== null && `client timeout ${formatDuration(timeout)}`]),
        failed: false,
      };
    }
    case "solana.confirmation_wait.completed": {
      const failed = a.result !== undefined && a.result !== "commitment_reached";
      const duration = nanosToMs(a.duration_ns);
      return {
        label: failed
          ? `Client stopped waiting: ${words(a.result ?? "")}${duration ? ` after ${formatDuration(duration)}` : ""}`
          : `Reached ${a.observed_commitment ?? "commitment"}${duration ? ` in ${formatDuration(duration)}` : ""}`,
        detail: compact([error]),
        failed,
      };
    }
    case "solana.status.observed": {
      const failed = a.source_result !== undefined && a.source_result !== "found";
      return {
        label: failed
          ? a.source_result === "not_found"
            ? "Transaction not found on-chain"
            : `Observer ${words(a.source_result ?? "")}`
          : `Observed at ${a.commitment ?? "unknown commitment"}`,
        detail: compact([
          a.slot && `slot ${a.slot}`,
          a.block_height && `at block height ${a.block_height}`,
          a.confirmations && `${a.confirmations} confirmations`,
          event.source_name && `by ${event.source_name}`,
          error,
        ]),
        failed,
      };
    }
    case "solana.execution.enriched": {
      const failed = a.execution_result === "failure";
      return {
        label: failed ? "Executed with an error" : "Executed successfully",
        detail: compact([
          a.commitment && `at ${a.commitment}`,
          a.fee_lamports && `fee ${Number(a.fee_lamports).toLocaleString()} lamports`,
          a.compute_units_consumed &&
            `${Number(a.compute_units_consumed).toLocaleString()} compute units`,
          error,
          event.source_name && `by ${event.source_name}`,
        ]),
        failed,
      };
    }
    case "solana.business_outcome.observed":
      return {
        label: `Business outcome: ${words(a.outcome ?? "unknown")}`,
        detail: compact([a.reason && words(a.reason)]),
        failed: a.outcome !== undefined && a.outcome !== "success",
      };
    case "landfall.data_quality.detected":
      return {
        label: `Data quality: ${words(a.quality_category ?? "issue")}`,
        detail: compact([a.quality_severity, a.quality_impact && words(a.quality_impact)]),
        failed: a.quality_severity === "error",
      };
    default:
      return { label: words(event.event_type), detail: "", failed: false };
  }
}

function relativeTime(origin: number, occurredAt: string): string {
  const offset = Date.parse(occurredAt) - origin;
  if (!Number.isFinite(offset)) return "";
  if (offset < 0) return `−${formatDuration(-offset)}`;
  return offset === 0 ? "+0 ms" : `+${formatDuration(offset)}`;
}

type TimelineItem =
  | { kind: "event"; event: TraceTimelineEvent }
  | {
      kind: "attempt";
      key: string;
      sequence: number | null;
      route: string | null;
      events: TraceTimelineEvent[];
    };

/** Groups submission events by attempt so each attempt reads as one unit. */
function itemsForPhase(phase: Phase, events: readonly TraceTimelineEvent[]): TimelineItem[] {
  if (phase !== "submit") return events.map((event) => ({ kind: "event", event }));
  const items: TimelineItem[] = [];
  const attempts = new Map<string, Extract<TimelineItem, { kind: "attempt" }>>();
  for (const event of events) {
    const attemptId =
      event.event_type === "solana.submission.retry_scheduled"
        ? undefined
        : event.attributes.attempt_id;
    if (attemptId === undefined) {
      items.push({ kind: "event", event });
      continue;
    }
    let attempt = attempts.get(attemptId);
    if (!attempt) {
      attempt = { kind: "attempt", key: attemptId, sequence: null, route: null, events: [] };
      attempts.set(attemptId, attempt);
      items.push(attempt);
    }
    attempt.events.push(event);
    attempt.sequence ??= event.attributes.attempt_sequence ?? null;
    attempt.route ??= routeLabel(event.attributes.route_name, event.attributes.route_id);
  }
  return items;
}

function EventRow({
  event,
  origin,
  citedBy,
  focused,
}: {
  event: TraceTimelineEvent;
  origin: number;
  citedBy: readonly DiagnosisGroup[];
  focused: boolean;
}): React.JSX.Element {
  const story = describeEvent(event);
  return (
    <li
      id={`event-${event.event_id}`}
      className={`timeline-event${story.failed ? " failed" : ""}${focused ? " focused" : ""}`}
    >
      <time className="timeline-offset" dateTime={event.occurred_at} title={event.occurred_at}>
        {relativeTime(origin, event.occurred_at)}
      </time>
      <span className="timeline-marker" aria-hidden="true" />
      <div className="timeline-body">
        <strong>
          {story.failed && <span className="visually-hidden">Failure: </span>}
          {story.label}
        </strong>
        {story.detail && <span>{story.detail}</span>}
        {citedBy.length > 0 && (
          <span className="timeline-citations">
            Evidence for{" "}
            {citedBy.map((group) => (
              <span
                className="rule-chip"
                key={group.rule_id}
                title={diagnosisCopy(group.claim_key, group.unknown_reason).title}
              >
                {group.rule_id}
              </span>
            ))}
          </span>
        )}
      </div>
    </li>
  );
}

export function TraceTimelineView({
  timeline,
  diagnoses,
  focusedEventIds,
}: {
  timeline: TraceTimeline;
  diagnoses: readonly DiagnosisGroup[];
  focusedEventIds: ReadonlySet<string>;
}): React.JSX.Element {
  const origin = timeline.events.reduce(
    (earliest, event) => Math.min(earliest, Date.parse(event.occurred_at) || earliest),
    Number.POSITIVE_INFINITY,
  );
  const citations = new Map<string, DiagnosisGroup[]>();
  for (const group of diagnoses)
    for (const eventId of group.evidence_event_ids)
      citations.set(eventId, [...(citations.get(eventId) ?? []), group]);
  const byPhase = new Map<Phase, TraceTimelineEvent[]>();
  for (const event of timeline.events) {
    const phase = PHASE_BY_TYPE[event.event_type] ?? "outcome";
    byPhase.set(phase, [...(byPhase.get(phase) ?? []), event]);
  }
  const row = (event: TraceTimelineEvent) => (
    <EventRow
      key={event.event_id}
      event={event}
      origin={origin}
      citedBy={citations.get(event.event_id) ?? []}
      focused={focusedEventIds.has(event.event_id)}
    />
  );
  return (
    <div className="timeline">
      {PHASES.map((phase) => {
        const events = byPhase.get(phase.id) ?? [];
        const failures = events.filter((event) => describeEvent(event).failed).length;
        return (
          <section className="timeline-phase" key={phase.id} aria-labelledby={`phase-${phase.id}`}>
            <h3 id={`phase-${phase.id}`}>
              {phase.title}
              {failures > 0 && (
                <span className="status-label failed">
                  {failures} failure{failures === 1 ? "" : "s"}
                </span>
              )}
            </h3>
            {events.length === 0 ? (
              <p className="muted timeline-empty">{phase.empty}</p>
            ) : (
              <ol className="timeline-events">
                {itemsForPhase(phase.id, events).map((item) =>
                  item.kind === "event" ? (
                    row(item.event)
                  ) : (
                    <li className="timeline-attempt" key={item.key}>
                      <p className="timeline-attempt-title">
                        Attempt {item.sequence ?? "?"}
                        {item.route && <span> · {item.route}</span>}
                      </p>
                      <ol className="timeline-events">{item.events.map(row)}</ol>
                    </li>
                  ),
                )}
              </ol>
            )}
          </section>
        );
      })}
    </div>
  );
}

/** Headline numbers derived from the timeline for the trace summary card. */
export function timelineFacts(timeline: TraceTimeline): Array<{ label: string; value: string }> {
  const events = timeline.events;
  const facts: Array<{ label: string; value: string }> = [];
  const attempts = events.filter((event) => event.event_type === "solana.submission.started");
  if (attempts.length) {
    const routes = [
      ...new Set(
        attempts.map(
          (event) =>
            routeLabel(event.attributes.route_name, event.attributes.route_id) ?? "unknown route",
        ),
      ),
    ];
    facts.push({
      label: "Submission attempts",
      value: `${attempts.length} across ${routes.length} route${routes.length === 1 ? "" : "s"}`,
    });
  }
  const signing = events.find((event) => event.event_type === "solana.signing.completed");
  const signingMs = nanosToMs(signing?.attributes.duration_ns);
  if (signingMs) facts.push({ label: "Signing time", value: formatDuration(signingMs) });
  const first = events[0];
  const landed = events.find(
    (event) =>
      event.event_type === "solana.status.observed" && event.attributes.source_result === "found",
  );
  if (first && landed) {
    const elapsed = Date.parse(landed.occurred_at) - Date.parse(first.occurred_at);
    if (Number.isFinite(elapsed))
      facts.push({
        label: "First on-chain observation",
        value: `${formatDuration(elapsed)} after start (${landed.attributes.commitment ?? "unknown"})`,
      });
  }
  const lastValid = events.find((event) => event.event_type === "solana.blockhash.acquired")
    ?.attributes.last_valid_block_height;
  const observedHeight = [...events]
    .reverse()
    .find((event) => event.attributes.block_height !== undefined)?.attributes.block_height;
  if (lastValid)
    facts.push({
      label: "Blockhash valid until",
      value:
        observedHeight && Number(observedHeight) > Number(lastValid)
          ? `block height ${lastValid} (expired; chain observed at ${observedHeight})`
          : `block height ${lastValid}`,
    });
  const execution = [...events]
    .reverse()
    .find((event) => event.event_type === "solana.execution.enriched");
  if (execution?.attributes.fee_lamports)
    facts.push({
      label: "Fee",
      value: `${Number(execution.attributes.fee_lamports).toLocaleString()} lamports`,
    });
  const units =
    execution?.attributes.compute_units_consumed ??
    events.find((event) => event.event_type === "solana.simulation.completed")?.attributes
      .units_consumed;
  if (units)
    facts.push({
      label: execution?.attributes.compute_units_consumed
        ? "Compute units"
        : "Simulated compute units",
      value: Number(units).toLocaleString(),
    });
  facts.push({ label: "Recorded events", value: String(events.length) });
  return facts;
}

/** Short label for an event, used when a diagnosis links to its evidence. */
export function evidenceLabel(timeline: TraceTimeline, eventId: string): string | null {
  const event = timeline.events.find((candidate) => candidate.event_id === eventId);
  if (!event) return null;
  const origin = Date.parse(timeline.events[0]?.occurred_at ?? event.occurred_at);
  return `${describeEvent(event).label} (${relativeTime(origin, event.occurred_at)})`;
}

/** Badge class for landing and execution states. */
export function stateTone(state: string): "confirmed" | "incomplete" | "failed" {
  if (["finalized", "confirmed", "processed", "success", "succeeded"].includes(state))
    return "confirmed";
  if (["failure", "failed", "expired", "dropped", "rejected"].includes(state)) return "failed";
  return "incomplete";
}
