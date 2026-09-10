//! Deterministic ordering of immutable events without assuming synchronized clocks.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use landfall_protocol::{
    AttemptId, DurationNs, EventId, EventSource, OperationId, SourceInstanceId, TraceId,
    UtcTimestamp, WireEvent,
};

/// Default tolerated difference between producer and collector wall clocks.
pub const DEFAULT_MAX_CLOCK_SKEW: Duration = Duration::from_secs(300);

/// Ordering policy that affects warnings, never the deterministic tie-breakers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrderingConfig {
    /// Absolute producer/collector wall-clock difference that triggers a warning.
    pub max_clock_skew: Duration,
}

impl Default for OrderingConfig {
    fn default() -> Self {
        Self {
            max_clock_skew: DEFAULT_MAX_CLOCK_SKEW,
        }
    }
}

/// Accepted event plus collector-assigned receipt time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectedEvent {
    /// Validated immutable wire event.
    pub event: WireEvent,
    /// Collector wall-clock time, kept separate from producer time.
    pub received_at: UtcTimestamp,
}

impl CollectedEvent {
    /// Wraps a wire event with its durable collector receipt time.
    #[must_use]
    pub const fn new(event: WireEvent, received_at: UtcTimestamp) -> Self {
        Self { event, received_at }
    }

    /// Immutable event identity.
    #[must_use]
    pub fn event_id(&self) -> EventId {
        envelope(&self.event).event_id
    }
}

/// Non-fatal limitation or correction applied while deriving a total order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderingWarning {
    /// Repeated delivery of an identical immutable event was collapsed.
    DuplicateDelivery {
        /// Deduplicated immutable event.
        event_id: EventId,
        /// Total number of received copies.
        copies: usize,
    },
    /// Producer and collector wall clocks differ beyond policy.
    ClockSkew {
        /// Event whose two wall-clock readings disagree.
        event_id: EventId,
        /// Absolute clock difference.
        difference: Duration,
    },
    /// A source's wall clock moved backward while its monotonic clock advanced.
    WallClockRegression {
        /// Earlier event according to the source monotonic clock.
        earlier_event_id: EventId,
        /// Later event according to the source monotonic clock.
        later_event_id: EventId,
        /// Amount by which wall time moved backward.
        difference: Duration,
    },
    /// Monotonic order contradicted an explicit start/completion relationship.
    MonotonicSemanticConflict {
        /// Event that monotonic time attempted to place first.
        monotonic_earlier_event_id: EventId,
        /// Event that the explicit semantic relation requires first.
        semantic_earlier_event_id: EventId,
    },
    /// Adjacent events required wall-clock fallback because their clocks cannot be compared.
    IncomparableClockOrder {
        /// Event selected first by stable fallback fields.
        earlier_event_id: EventId,
        /// Event selected second by stable fallback fields.
        later_event_id: EventId,
    },
}

/// Canonical events and deterministic data-quality warnings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalOrder {
    events: Vec<CollectedEvent>,
    warnings: Vec<OrderingWarning>,
}

impl CanonicalOrder {
    /// Events in canonical total order.
    #[must_use]
    pub fn events(&self) -> &[CollectedEvent] {
        &self.events
    }

    /// Clock and duplicate-delivery limitations found while ordering.
    #[must_use]
    pub fn warnings(&self) -> &[OrderingWarning] {
        &self.warnings
    }

    /// Consumes the result and returns the canonically ordered events.
    #[must_use]
    pub fn into_events(self) -> Vec<CollectedEvent> {
        self.events
    }
}

/// Ordering failure that cannot be resolved without discarding evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderingError {
    /// One event ID was reused for two different immutable facts.
    ConflictingDuplicate {
        /// Reused immutable identity.
        event_id: EventId,
    },
    /// Explicit semantic relationships formed a cycle.
    SemanticConstraintCycle,
}

impl std::fmt::Display for OrderingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConflictingDuplicate { event_id } => {
                write!(
                    formatter,
                    "event ID {event_id} identifies conflicting facts"
                )
            }
            Self::SemanticConstraintCycle => {
                formatter.write_str("event semantic relationships contain a cycle")
            }
        }
    }
}

impl std::error::Error for OrderingError {}

/// Deduplicates and deterministically orders events for replay.
///
/// Precedence is explicit semantic relationships, comparable source monotonic
/// clocks, producer wall time, collector receipt time, and finally event ID.
pub fn canonical_order(
    events: impl IntoIterator<Item = CollectedEvent>,
    config: OrderingConfig,
) -> Result<CanonicalOrder, OrderingError> {
    let (events, deliveries) = deduplicate(events)?;
    let mut warnings = duplicate_warnings(&deliveries);
    warnings.extend(clock_skew_warnings(&events, config));

    let mut edges = vec![BTreeSet::new(); events.len()];
    add_semantic_edges(&events, &mut edges);
    add_monotonic_edges(&events, &mut edges, &mut warnings);

    let ordered_indices = topological_order(&events, &edges)?;
    add_incomparable_warnings(&events, &edges, &ordered_indices, &mut warnings);
    let mut slots = events.into_iter().map(Some).collect::<Vec<_>>();
    let ordered_events = ordered_indices
        .into_iter()
        .filter_map(|index| slots[index].take())
        .collect();

    Ok(CanonicalOrder {
        events: ordered_events,
        warnings,
    })
}

fn deduplicate(
    events: impl IntoIterator<Item = CollectedEvent>,
) -> Result<(Vec<CollectedEvent>, BTreeMap<EventId, usize>), OrderingError> {
    let mut unique = BTreeMap::<EventId, CollectedEvent>::new();
    let mut deliveries = BTreeMap::<EventId, usize>::new();
    for candidate in events {
        let event_id = candidate.event_id();
        *deliveries.entry(event_id).or_default() += 1;
        if let Some(existing) = unique.get_mut(&event_id) {
            if existing.event != candidate.event {
                return Err(OrderingError::ConflictingDuplicate { event_id });
            }
            if candidate.received_at.get() < existing.received_at.get() {
                existing.received_at = candidate.received_at;
            }
        } else {
            unique.insert(event_id, candidate);
        }
    }
    Ok((unique.into_values().collect(), deliveries))
}

fn duplicate_warnings(deliveries: &BTreeMap<EventId, usize>) -> Vec<OrderingWarning> {
    deliveries
        .iter()
        .filter_map(|(&event_id, &copies)| {
            (copies > 1).then_some(OrderingWarning::DuplicateDelivery { event_id, copies })
        })
        .collect()
}

fn clock_skew_warnings(events: &[CollectedEvent], config: OrderingConfig) -> Vec<OrderingWarning> {
    events
        .iter()
        .filter_map(|event| {
            let metadata = envelope(&event.event);
            let difference = (event.received_at.get() - metadata.occurred_at.get()).unsigned_abs();
            (difference > config.max_clock_skew).then_some(OrderingWarning::ClockSkew {
                event_id: metadata.event_id,
                difference,
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SpanKey {
    Simulation(OperationId),
    Signing(OperationId),
    Submission(AttemptId),
    ConfirmationWait(OperationId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SemanticRole {
    TraceCreated,
    Start(SpanKey),
    Complete(SpanKey),
    RetryAfter(AttemptId),
    Other,
}

fn semantic_role(event: &WireEvent) -> SemanticRole {
    match event {
        WireEvent::TraceCreated(_) => SemanticRole::TraceCreated,
        WireEvent::SimulationStarted(event) => {
            SemanticRole::Start(SpanKey::Simulation(event.attributes.simulation_id))
        }
        WireEvent::SimulationCompleted(event) => {
            SemanticRole::Complete(SpanKey::Simulation(event.attributes.simulation_id))
        }
        WireEvent::SigningStarted(event) => {
            SemanticRole::Start(SpanKey::Signing(event.attributes.signing_id))
        }
        WireEvent::SigningCompleted(event) => {
            SemanticRole::Complete(SpanKey::Signing(event.attributes.signing_id))
        }
        WireEvent::SubmissionStarted(event) => {
            SemanticRole::Start(SpanKey::Submission(event.attributes.attempt_id))
        }
        WireEvent::SubmissionCompleted(event) => {
            SemanticRole::Complete(SpanKey::Submission(event.attributes.attempt_id))
        }
        WireEvent::SubmissionRetryScheduled(event) => {
            SemanticRole::RetryAfter(event.attributes.previous_attempt_id)
        }
        WireEvent::ConfirmationWaitStarted(event) => {
            SemanticRole::Start(SpanKey::ConfirmationWait(event.attributes.wait_id))
        }
        WireEvent::ConfirmationWaitCompleted(event) => {
            SemanticRole::Complete(SpanKey::ConfirmationWait(event.attributes.wait_id))
        }
        WireEvent::BlockhashAcquired(_)
        | WireEvent::StatusObserved(_)
        | WireEvent::ExecutionEnriched(_)
        | WireEvent::BusinessOutcomeObserved(_)
        | WireEvent::DataQualityDetected(_) => SemanticRole::Other,
    }
}

fn add_semantic_edges(events: &[CollectedEvent], edges: &mut [BTreeSet<usize>]) {
    let mut trace_created = BTreeMap::<TraceId, Vec<usize>>::new();
    let mut trace_members = BTreeMap::<TraceId, Vec<usize>>::new();
    let mut starts = BTreeMap::<SpanKey, Vec<usize>>::new();
    let mut completions = BTreeMap::<SpanKey, Vec<usize>>::new();
    let mut attempt_completions = BTreeMap::<AttemptId, Vec<usize>>::new();
    let mut retries = BTreeMap::<AttemptId, Vec<usize>>::new();

    for (index, event) in events.iter().enumerate() {
        let metadata = envelope(&event.event);
        if let Some(trace_id) = metadata.trace_id {
            trace_members.entry(trace_id).or_default().push(index);
            if matches!(semantic_role(&event.event), SemanticRole::TraceCreated) {
                trace_created.entry(trace_id).or_default().push(index);
            }
        }
        match semantic_role(&event.event) {
            SemanticRole::Start(key) => starts.entry(key).or_default().push(index),
            SemanticRole::Complete(key) => {
                completions.entry(key).or_default().push(index);
                if let SpanKey::Submission(attempt_id) = key {
                    attempt_completions
                        .entry(attempt_id)
                        .or_default()
                        .push(index);
                }
            }
            SemanticRole::RetryAfter(attempt_id) => {
                retries.entry(attempt_id).or_default().push(index);
            }
            SemanticRole::TraceCreated | SemanticRole::Other => {}
        }
    }

    for (trace_id, roots) in trace_created {
        if let Some(members) = trace_members.get(&trace_id) {
            for root in roots {
                for &member in members {
                    if !matches!(
                        semantic_role(&events[member].event),
                        SemanticRole::TraceCreated
                    ) {
                        edges[root].insert(member);
                    }
                }
            }
        }
    }
    for (key, span_starts) in starts {
        if let Some(span_completions) = completions.get(&key) {
            for start in span_starts {
                edges[start].extend(span_completions);
            }
        }
    }
    for (attempt_id, completed) in attempt_completions {
        if let Some(scheduled) = retries.get(&attempt_id) {
            for completion in completed {
                edges[completion].extend(scheduled);
            }
        }
    }
}

fn add_monotonic_edges(
    events: &[CollectedEvent],
    edges: &mut [BTreeSet<usize>],
    warnings: &mut Vec<OrderingWarning>,
) {
    let mut sources = BTreeMap::<SourceInstanceId, Vec<(DurationNs, EventId, usize)>>::new();
    for (index, event) in events.iter().enumerate() {
        let metadata = envelope(&event.event);
        if let (Some(instance_id), Some(monotonic_ns)) =
            (metadata.source.instance_id, metadata.monotonic_ns)
        {
            sources
                .entry(instance_id)
                .or_default()
                .push((monotonic_ns, metadata.event_id, index));
        }
    }

    for source_events in sources.values_mut() {
        source_events.sort_unstable();
        for pair in source_events.windows(2) {
            let (earlier_time, earlier_id, earlier) = pair[0];
            let (later_time, later_id, later) = pair[1];
            if later_time == earlier_time {
                continue;
            }
            let earlier_wall = envelope(&events[earlier].event).occurred_at.get();
            let later_wall = envelope(&events[later].event).occurred_at.get();
            if later_wall < earlier_wall {
                warnings.push(OrderingWarning::WallClockRegression {
                    earlier_event_id: earlier_id,
                    later_event_id: later_id,
                    difference: (earlier_wall - later_wall).unsigned_abs(),
                });
            }
            if reachable(edges, later, earlier) {
                warnings.push(OrderingWarning::MonotonicSemanticConflict {
                    monotonic_earlier_event_id: earlier_id,
                    semantic_earlier_event_id: later_id,
                });
            } else {
                edges[earlier].insert(later);
            }
        }
    }
}

fn topological_order(
    events: &[CollectedEvent],
    edges: &[BTreeSet<usize>],
) -> Result<Vec<usize>, OrderingError> {
    let mut indegree = vec![0_usize; events.len()];
    for targets in edges {
        for &target in targets {
            indegree[target] += 1;
        }
    }

    let mut emitted = vec![false; events.len()];
    let mut result = Vec::with_capacity(events.len());
    while result.len() < events.len() {
        let next = (0..events.len())
            .filter(|&index| !emitted[index] && indegree[index] == 0)
            .min_by(|&left, &right| fallback_cmp(&events[left], &events[right]));
        let Some(next) = next else {
            return Err(OrderingError::SemanticConstraintCycle);
        };
        emitted[next] = true;
        result.push(next);
        for &target in &edges[next] {
            indegree[target] -= 1;
        }
    }
    Ok(result)
}

fn fallback_cmp(left: &CollectedEvent, right: &CollectedEvent) -> Ordering {
    let left_metadata = envelope(&left.event);
    let right_metadata = envelope(&right.event);
    left_metadata
        .occurred_at
        .get()
        .cmp(&right_metadata.occurred_at.get())
        .then_with(|| left.received_at.get().cmp(&right.received_at.get()))
        .then_with(|| left_metadata.event_id.cmp(&right_metadata.event_id))
}

fn add_incomparable_warnings(
    events: &[CollectedEvent],
    edges: &[BTreeSet<usize>],
    ordered: &[usize],
    warnings: &mut Vec<OrderingWarning>,
) {
    for pair in ordered.windows(2) {
        let earlier = pair[0];
        let later = pair[1];
        if reachable(edges, earlier, later)
            || comparable_monotonic(&events[earlier], &events[later])
        {
            continue;
        }
        warnings.push(OrderingWarning::IncomparableClockOrder {
            earlier_event_id: events[earlier].event_id(),
            later_event_id: events[later].event_id(),
        });
    }
}

fn comparable_monotonic(left: &CollectedEvent, right: &CollectedEvent) -> bool {
    let left = envelope(&left.event);
    let right = envelope(&right.event);
    left.source.instance_id.is_some()
        && left.source.instance_id == right.source.instance_id
        && left.monotonic_ns.is_some()
        && right.monotonic_ns.is_some()
        && left.monotonic_ns != right.monotonic_ns
}

fn reachable(edges: &[BTreeSet<usize>], from: usize, target: usize) -> bool {
    let mut pending = vec![from];
    let mut visited = vec![false; edges.len()];
    while let Some(current) = pending.pop() {
        if current == target {
            return true;
        }
        if visited[current] {
            continue;
        }
        visited[current] = true;
        pending.extend(edges[current].iter().copied());
    }
    false
}

#[derive(Clone, Copy)]
struct Envelope<'a> {
    event_id: EventId,
    occurred_at: UtcTimestamp,
    monotonic_ns: Option<DurationNs>,
    trace_id: Option<TraceId>,
    source: &'a EventSource,
}

macro_rules! envelope_match {
    ($event:expr, $binding:ident => $result:expr) => {
        match $event {
            WireEvent::TraceCreated($binding) => $result,
            WireEvent::BlockhashAcquired($binding) => $result,
            WireEvent::SimulationStarted($binding) => $result,
            WireEvent::SimulationCompleted($binding) => $result,
            WireEvent::SigningStarted($binding) => $result,
            WireEvent::SigningCompleted($binding) => $result,
            WireEvent::SubmissionStarted($binding) => $result,
            WireEvent::SubmissionCompleted($binding) => $result,
            WireEvent::SubmissionRetryScheduled($binding) => $result,
            WireEvent::ConfirmationWaitStarted($binding) => $result,
            WireEvent::ConfirmationWaitCompleted($binding) => $result,
            WireEvent::StatusObserved($binding) => $result,
            WireEvent::ExecutionEnriched($binding) => $result,
            WireEvent::BusinessOutcomeObserved($binding) => $result,
            WireEvent::DataQualityDetected($binding) => $result,
        }
    };
}

fn envelope(event: &WireEvent) -> Envelope<'_> {
    envelope_match!(event, value => Envelope {
        event_id: value.event_id,
        occurred_at: value.occurred_at,
        monotonic_ns: value.monotonic_ns,
        trace_id: value.trace_id,
        source: &value.source,
    })
}
