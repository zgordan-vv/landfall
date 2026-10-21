//! Baseline-versus-candidate comparison of two explicitly designated environments.
//!
//! The question this answers is "we changed how we send transactions; did
//! reliability actually improve?". Roles are never inferred from recency, every
//! metric states its own denominator, and a metric without evidence is `null`
//! rather than zero.

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Stable version of the metric semantics below.
pub const COMPARISON_DEFINITIONS_VERSION: &str = "cohort-comparison-v2";

/// Minimum sample per cohort before a difference is presented as a conclusion.
///
/// At 30 traces a rate near 50% still carries a 95% interval of roughly ±18
/// percentage points, so this is a floor against reading noise, not a claim of
/// statistical significance.
pub const MINIMUM_COHORT_SAMPLE: u64 = 30;

/// Default comparison window when the caller does not choose one.
pub const DEFAULT_WINDOW_HOURS: u32 = 168;

/// Latency changes smaller than this fraction of the baseline are reported as unchanged.
///
/// Confirmation times come from status polling and from two clocks (application
/// and observer), so a few milliseconds of difference is measurement noise.
pub const LATENCY_RELATIVE_RESOLUTION: f64 = 0.05;

/// Latency changes smaller than this many milliseconds are reported as unchanged.
pub const LATENCY_ABSOLUTE_RESOLUTION_MS: f64 = 50.0;

/// Longest accepted comparison window (90 days).
pub const MAXIMUM_WINDOW_HOURS: u32 = 2_160;

/// Where the baseline and candidate roles came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoleSource {
    /// `baseline_environment_id` and `candidate_environment_id` query parameters.
    Query,
    /// The project's persisted comparison cohorts.
    ProjectSetting,
}

/// Baseline and candidate environments chosen for one comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CohortRoles {
    pub baseline: Uuid,
    pub candidate: Uuid,
    pub source: RoleSource,
}

/// Why the requested roles cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleError {
    /// Only one of the two query parameters was supplied.
    IncompleteQuery,
    /// Baseline and candidate are the same environment.
    SameEnvironment,
}

impl RoleError {
    /// Stable API error code and message.
    #[must_use]
    pub const fn code_and_message(self) -> (&'static str, &'static str) {
        match self {
            Self::IncompleteQuery => (
                "incomplete_cohort_roles",
                "pass both baseline_environment_id and candidate_environment_id, or neither",
            ),
            Self::SameEnvironment => (
                "identical_cohort_roles",
                "baseline and candidate must be different environments",
            ),
        }
    }
}

/// Chooses roles from explicit query parameters, then the project setting.
///
/// Returns `Ok(None)` when neither is present; the caller reports
/// "not configured" instead of guessing.
pub fn select_roles(
    query_baseline: Option<Uuid>,
    query_candidate: Option<Uuid>,
    setting: Option<(Uuid, Uuid)>,
) -> Result<Option<CohortRoles>, RoleError> {
    let roles = match (query_baseline, query_candidate) {
        (Some(baseline), Some(candidate)) => Some(CohortRoles {
            baseline,
            candidate,
            source: RoleSource::Query,
        }),
        (None, None) => setting.map(|(baseline, candidate)| CohortRoles {
            baseline,
            candidate,
            source: RoleSource::ProjectSetting,
        }),
        _ => return Err(RoleError::IncompleteQuery),
    };
    match roles {
        Some(roles) if roles.baseline == roles.candidate => Err(RoleError::SameEnvironment),
        roles => Ok(roles),
    }
}

/// Evidence for one trace in the window, reduced from its projection and raw events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TraceEvidence {
    /// Number of `solana.submission.started` events.
    pub submission_attempts: u64,
    /// Number of `solana.submission.retry_scheduled` events.
    pub retries_scheduled: u64,
    /// Projected landing state is processed, confirmed, or finalized.
    pub landed: bool,
    /// Projected landing state is expired.
    pub expired: bool,
    /// Projected execution result; `None` when unknown.
    pub execution_success: Option<bool>,
    /// Earliest `solana.submission.started`.
    pub first_submission_at: Option<OffsetDateTime>,
    /// Earliest `found` status observation at confirmed or finalized commitment.
    pub first_confirmed_at: Option<OffsetDateTime>,
    /// Fee from the latest `solana.execution.enriched` event.
    pub fee_lamports: Option<u64>,
}

impl TraceEvidence {
    const fn submitted(&self) -> bool {
        self.submission_attempts > 0
    }

    fn confirmation_latency_ms(&self) -> Option<f64> {
        let (submitted, confirmed) = self.first_submission_at.zip(self.first_confirmed_at)?;
        let elapsed = confirmed - submitted;
        (!elapsed.is_negative()).then(|| elapsed.as_seconds_f64() * 1_000.0)
    }
}

/// Unit of a metric value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricUnit {
    /// A fraction between 0 and 1.
    Ratio,
    Milliseconds,
    Lamports,
}

/// Which direction of change counts as an improvement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BetterWhen {
    Higher,
    Lower,
    /// Neither direction is better by itself (for example a fee raised on purpose).
    Neutral,
}

/// Reading of one metric's change from baseline to candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricOutcome {
    Improved,
    Regressed,
    Unchanged,
    /// The metric changed but has no preferred direction.
    Changed,
    /// At least one cohort has no evidence for this metric.
    NoEvidence,
}

/// One cohort's value for one metric.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, ToSchema)]
pub struct MetricValue {
    /// `null` when the cohort has no evidence for this metric.
    pub value: Option<f64>,
    /// Count of traces meeting the condition, for rate metrics only.
    pub numerator: Option<u64>,
    /// Traces that contributed: the denominator of a rate, or the count behind a median.
    pub sample_size: u64,
}

/// One metric compared between baseline and candidate.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct MetricComparison {
    pub key: &'static str,
    pub label: &'static str,
    pub unit: MetricUnit,
    pub better_when: BetterWhen,
    /// How the value and its denominator are computed.
    pub definition: &'static str,
    pub baseline: MetricValue,
    pub candidate: MetricValue,
    /// Candidate minus baseline; `null` when either side has no evidence.
    pub delta: Option<f64>,
    pub outcome: MetricOutcome,
    /// Both cohorts reach [`MINIMUM_COHORT_SAMPLE`] for this metric.
    pub sufficient_sample: bool,
}

/// Whether the project has designated comparison roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonStatus {
    Ready,
    /// No roles were passed and the project has not designated any.
    NotConfigured,
}

/// What a cohort is: its environment and the operator's note on what differs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct CohortEnvironment {
    pub environment_id: String,
    pub name: String,
    pub cluster: String,
    /// Operator-supplied note on what this environment changes, if any.
    pub description: Option<String>,
}

/// One cohort in a comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct CohortSummary {
    pub environment: CohortEnvironment,
    /// Traces whose first event falls inside the window.
    pub traces: u64,
    /// Traces with at least one submission attempt.
    pub submitted_traces: u64,
    /// `traces` reaches [`MINIMUM_COHORT_SAMPLE`].
    pub sufficient_sample: bool,
}

/// Time range a comparison covers. Traces belong to it by their first event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ComparisonWindow {
    pub hours: u32,
    /// Inclusive RFC 3339 start.
    pub start: String,
    /// Exclusive RFC 3339 end.
    pub end: String,
}

impl ComparisonWindow {
    /// Window of `hours` ending at `end`.
    #[must_use]
    pub fn new(hours: u32, start: OffsetDateTime, end: OffsetDateTime) -> Self {
        let format = |value: OffsetDateTime| {
            value
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default()
        };
        Self {
            hours,
            start: format(start),
            end: format(end),
        }
    }
}

/// Baseline-versus-candidate comparison served by `GET /v1/comparison`.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct ComparisonReport {
    pub status: ComparisonStatus,
    /// `null` when not configured.
    pub role_source: Option<RoleSource>,
    pub window: ComparisonWindow,
    /// Per-cohort and per-metric sample needed before a difference is a conclusion.
    pub minimum_sample: u64,
    pub definitions_version: &'static str,
    pub baseline: Option<CohortSummary>,
    pub candidate: Option<CohortSummary>,
    /// Both cohorts reach `minimum_sample` traces.
    pub sufficient_sample: bool,
    pub metrics: Vec<MetricComparison>,
    /// The project's environments, so an unconfigured page can say what to choose from.
    pub environments: Vec<CohortEnvironment>,
}

/// An environment and the evidence of its traces in the window.
pub type Cohort<'a> = (CohortEnvironment, &'a [TraceEvidence]);

impl ComparisonReport {
    /// Report for a project with no designated roles.
    #[must_use]
    pub const fn not_configured(
        window: ComparisonWindow,
        environments: Vec<CohortEnvironment>,
    ) -> Self {
        Self {
            status: ComparisonStatus::NotConfigured,
            role_source: None,
            window,
            minimum_sample: MINIMUM_COHORT_SAMPLE,
            definitions_version: COMPARISON_DEFINITIONS_VERSION,
            baseline: None,
            candidate: None,
            sufficient_sample: false,
            metrics: Vec::new(),
            environments,
        }
    }

    /// Report comparing the per-trace evidence of two designated cohorts.
    #[must_use]
    pub fn ready(
        roles: CohortRoles,
        window: ComparisonWindow,
        cohorts: (Cohort<'_>, Cohort<'_>),
        environments: Vec<CohortEnvironment>,
    ) -> Self {
        let ((baseline_environment, baseline), (candidate_environment, candidate)) = cohorts;
        let summary = |environment, traces: &[TraceEvidence]| {
            let totals = cohort_totals(traces);
            CohortSummary {
                environment,
                traces: totals.traces,
                submitted_traces: totals.submitted_traces,
                sufficient_sample: totals.traces >= MINIMUM_COHORT_SAMPLE,
            }
        };
        let baseline_summary = summary(baseline_environment, baseline);
        let candidate_summary = summary(candidate_environment, candidate);
        Self {
            status: ComparisonStatus::Ready,
            role_source: Some(roles.source),
            window,
            minimum_sample: MINIMUM_COHORT_SAMPLE,
            definitions_version: COMPARISON_DEFINITIONS_VERSION,
            sufficient_sample: baseline_summary.sufficient_sample
                && candidate_summary.sufficient_sample,
            baseline: Some(baseline_summary),
            candidate: Some(candidate_summary),
            metrics: compare_metrics(baseline, candidate),
            environments,
        }
    }
}

/// Counts that describe one cohort as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CohortTotals {
    pub traces: u64,
    pub submitted_traces: u64,
}

struct MetricSpec {
    key: &'static str,
    label: &'static str,
    unit: MetricUnit,
    better_when: BetterWhen,
    definition: &'static str,
    measure: fn(&[TraceEvidence]) -> MetricValue,
}

const METRICS: &[MetricSpec] = &[
    MetricSpec {
        key: "landing_rate",
        label: "Landing rate",
        unit: MetricUnit::Ratio,
        better_when: BetterWhen::Higher,
        definition: "Submitted traces observed at processed, confirmed, or finalized commitment, divided by traces with at least one submission.",
        measure: |traces| rate(traces, TraceEvidence::submitted, |trace| trace.landed),
    },
    MetricSpec {
        key: "execution_success_rate",
        label: "Execution success rate",
        unit: MetricUnit::Ratio,
        better_when: BetterWhen::Higher,
        definition: "Traces whose on-chain execution succeeded, divided by traces with a known execution result.",
        measure: |traces| {
            rate(
                traces,
                |trace| trace.execution_success.is_some(),
                |trace| trace.execution_success == Some(true),
            )
        },
    },
    MetricSpec {
        key: "expiry_rate",
        label: "Expiry rate",
        unit: MetricUnit::Ratio,
        better_when: BetterWhen::Lower,
        definition: "Submitted traces whose blockhash validity window passed without observed inclusion, divided by traces with at least one submission.",
        measure: |traces| rate(traces, TraceEvidence::submitted, |trace| trace.expired),
    },
    MetricSpec {
        key: "retry_rate",
        label: "Retry rate",
        unit: MetricUnit::Ratio,
        better_when: BetterWhen::Lower,
        definition: "Submitted traces with more than one submission attempt or a scheduled retry, divided by traces with at least one submission.",
        measure: |traces| {
            rate(traces, TraceEvidence::submitted, |trace| {
                trace.submission_attempts > 1 || trace.retries_scheduled > 0
            })
        },
    },
    MetricSpec {
        key: "median_confirmation_ms",
        label: "Median time to confirmation",
        unit: MetricUnit::Milliseconds,
        better_when: BetterWhen::Lower,
        definition: "Median time from the first submission to the first status observation at confirmed or finalized commitment, over traces with both events. Resolution is limited by the observer's polling interval and by clock differences between the application and the observer, so changes under 5% of the baseline (or 50 ms) count as no change.",
        measure: |traces| {
            median(
                traces
                    .iter()
                    .filter_map(TraceEvidence::confirmation_latency_ms),
            )
        },
    },
    MetricSpec {
        key: "median_fee_lamports",
        label: "Median fee",
        unit: MetricUnit::Lamports,
        better_when: BetterWhen::Neutral,
        definition: "Median transaction fee in lamports reported by execution-enriched events. A higher fee can be a deliberate priority-fee change, so no direction is rated.",
        measure: |traces| median(traces.iter().filter_map(fee_lamports)),
    },
];

/// Counts traces and submitted traces in one cohort.
#[must_use]
pub fn cohort_totals(traces: &[TraceEvidence]) -> CohortTotals {
    CohortTotals {
        traces: traces.len() as u64,
        submitted_traces: traces.iter().filter(|trace| trace.submitted()).count() as u64,
    }
}

/// Computes every metric for both cohorts, with deltas and direction-aware outcomes.
#[must_use]
pub fn compare_metrics(
    baseline: &[TraceEvidence],
    candidate: &[TraceEvidence],
) -> Vec<MetricComparison> {
    METRICS
        .iter()
        .map(|spec| {
            let baseline = (spec.measure)(baseline);
            let candidate = (spec.measure)(candidate);
            let delta = baseline
                .value
                .zip(candidate.value)
                .map(|(old, new)| new - old);
            MetricComparison {
                key: spec.key,
                label: spec.label,
                unit: spec.unit,
                better_when: spec.better_when,
                definition: spec.definition,
                baseline,
                candidate,
                delta,
                outcome: outcome(delta, baseline.value, spec),
                sufficient_sample: baseline.sample_size >= MINIMUM_COHORT_SAMPLE
                    && candidate.sample_size >= MINIMUM_COHORT_SAMPLE,
            }
        })
        .collect()
}

fn outcome(delta: Option<f64>, baseline: Option<f64>, spec: &MetricSpec) -> MetricOutcome {
    let (Some(delta), Some(baseline)) = (delta, baseline) else {
        return MetricOutcome::NoEvidence;
    };
    let resolution = match spec.unit {
        MetricUnit::Milliseconds => {
            (baseline.abs() * LATENCY_RELATIVE_RESOLUTION).max(LATENCY_ABSOLUTE_RESOLUTION_MS)
        }
        MetricUnit::Ratio | MetricUnit::Lamports => 1e-9,
    };
    if delta.abs() < resolution {
        return MetricOutcome::Unchanged;
    }
    match (spec.better_when, delta > 0.0) {
        (BetterWhen::Neutral, _) => MetricOutcome::Changed,
        (BetterWhen::Higher, true) | (BetterWhen::Lower, false) => MetricOutcome::Improved,
        (BetterWhen::Higher, false) | (BetterWhen::Lower, true) => MetricOutcome::Regressed,
    }
}

#[allow(clippy::cast_precision_loss)]
fn rate(
    traces: &[TraceEvidence],
    eligible: fn(&TraceEvidence) -> bool,
    counted: fn(&TraceEvidence) -> bool,
) -> MetricValue {
    let denominator = traces.iter().filter(|trace| eligible(trace)).count() as u64;
    let numerator = traces
        .iter()
        .filter(|trace| eligible(trace) && counted(trace))
        .count() as u64;
    MetricValue {
        value: (denominator > 0).then(|| numerator as f64 / denominator as f64),
        numerator: Some(numerator),
        sample_size: denominator,
    }
}

#[allow(clippy::cast_precision_loss)]
fn fee_lamports(trace: &TraceEvidence) -> Option<f64> {
    trace.fee_lamports.map(|fee| fee as f64)
}

fn median(values: impl Iterator<Item = f64>) -> MetricValue {
    let mut values = values.collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    let value = match values.len() {
        0 => None,
        length if length % 2 == 1 => Some(values[middle]),
        _ => Some(f64::midpoint(values[middle - 1], values[middle])),
    };
    MetricValue {
        value,
        numerator: None,
        sample_size: values.len() as u64,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn at(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(seconds)
    }

    fn landed(latency_seconds: i64, fee: u64) -> TraceEvidence {
        TraceEvidence {
            submission_attempts: 1,
            landed: true,
            execution_success: Some(true),
            first_submission_at: Some(at(100)),
            first_confirmed_at: Some(at(100 + latency_seconds)),
            fee_lamports: Some(fee),
            ..TraceEvidence::default()
        }
    }

    fn expired_after_retry() -> TraceEvidence {
        TraceEvidence {
            submission_attempts: 2,
            retries_scheduled: 1,
            expired: true,
            first_submission_at: Some(at(100)),
            ..TraceEvidence::default()
        }
    }

    fn simulation_only() -> TraceEvidence {
        TraceEvidence::default()
    }

    fn metric<'a>(metrics: &'a [MetricComparison], key: &str) -> &'a MetricComparison {
        metrics
            .iter()
            .find(|metric| metric.key == key)
            .expect("metric is defined")
    }

    #[test]
    fn query_roles_take_precedence_over_the_project_setting() {
        let roles = select_roles(Some(id(1)), Some(id(2)), Some((id(3), id(4))));
        assert_eq!(
            roles,
            Ok(Some(CohortRoles {
                baseline: id(1),
                candidate: id(2),
                source: RoleSource::Query
            }))
        );
    }

    #[test]
    fn project_setting_is_used_when_no_query_is_given() {
        let roles = select_roles(None, None, Some((id(3), id(4))));
        assert_eq!(
            roles,
            Ok(Some(CohortRoles {
                baseline: id(3),
                candidate: id(4),
                source: RoleSource::ProjectSetting
            }))
        );
    }

    #[test]
    fn missing_roles_are_not_configured_rather_than_guessed() {
        assert_eq!(select_roles(None, None, None), Ok(None));
    }

    #[test]
    fn rejects_half_a_query_and_identical_roles() {
        assert_eq!(
            select_roles(Some(id(1)), None, Some((id(3), id(4)))),
            Err(RoleError::IncompleteQuery)
        );
        assert_eq!(
            select_roles(None, Some(id(2)), None),
            Err(RoleError::IncompleteQuery)
        );
        assert_eq!(
            select_roles(Some(id(1)), Some(id(1)), None),
            Err(RoleError::SameEnvironment)
        );
    }

    #[test]
    fn rates_use_submitted_traces_as_denominator() {
        let baseline = [
            landed(2, 5_000),
            expired_after_retry(),
            simulation_only(),
            landed(4, 5_000),
        ];
        let candidate = [landed(1, 5_000), landed(3, 7_000), landed(5, 9_000)];
        let metrics = compare_metrics(&baseline, &candidate);

        let landing = metric(&metrics, "landing_rate");
        assert_eq!(landing.baseline.numerator, Some(2));
        assert_eq!(landing.baseline.sample_size, 3);
        assert_eq!(landing.candidate.value, Some(1.0));
        assert_eq!(landing.outcome, MetricOutcome::Improved);
        assert!(!landing.sufficient_sample);

        let expiry = metric(&metrics, "expiry_rate");
        assert_eq!(expiry.candidate.value, Some(0.0));
        assert_eq!(expiry.outcome, MetricOutcome::Improved);

        let retry = metric(&metrics, "retry_rate");
        assert_eq!(retry.baseline.numerator, Some(1));
        assert_eq!(retry.outcome, MetricOutcome::Improved);

        let execution = metric(&metrics, "execution_success_rate");
        assert_eq!(execution.baseline.sample_size, 2);
        assert_eq!(execution.baseline.value, Some(1.0));
        assert_eq!(execution.outcome, MetricOutcome::Unchanged);

        assert_eq!(
            cohort_totals(&baseline),
            CohortTotals {
                traces: 4,
                submitted_traces: 3
            }
        );
    }

    #[test]
    fn worse_landing_and_slower_confirmation_are_regressions() {
        let baseline = [landed(1, 5_000), landed(1, 5_000)];
        let candidate = [landed(3, 5_000), expired_after_retry()];
        let metrics = compare_metrics(&baseline, &candidate);
        assert_eq!(
            metric(&metrics, "landing_rate").outcome,
            MetricOutcome::Regressed
        );
        assert_eq!(
            metric(&metrics, "expiry_rate").outcome,
            MetricOutcome::Regressed
        );
        let latency = metric(&metrics, "median_confirmation_ms");
        assert_eq!(latency.baseline.value, Some(1_000.0));
        assert_eq!(latency.candidate.value, Some(3_000.0));
        assert_eq!(latency.delta, Some(2_000.0));
        assert_eq!(latency.outcome, MetricOutcome::Regressed);
    }

    #[test]
    fn medians_ignore_traces_without_evidence_and_average_even_counts() {
        let baseline = [landed(1, 5_000), landed(3, 7_000), expired_after_retry()];
        let candidate = [landed(2, 6_000)];
        let metrics = compare_metrics(&baseline, &candidate);
        let latency = metric(&metrics, "median_confirmation_ms");
        assert_eq!(latency.baseline.value, Some(2_000.0));
        assert_eq!(latency.baseline.sample_size, 2);
        let fee = metric(&metrics, "median_fee_lamports");
        assert_eq!(fee.baseline.value, Some(6_000.0));
        assert_eq!(fee.outcome, MetricOutcome::Unchanged);
    }

    #[test]
    fn latency_changes_below_measurement_resolution_are_unchanged() {
        let quick = |offset_ms: i64| TraceEvidence {
            first_confirmed_at: Some(at(100) + time::Duration::milliseconds(offset_ms)),
            ..landed(0, 5_000)
        };
        let metrics = compare_metrics(&[quick(613)], &[quick(617)]);
        assert_eq!(
            metric(&metrics, "median_confirmation_ms").outcome,
            MetricOutcome::Unchanged
        );
        let metrics = compare_metrics(&[quick(2_000)], &[quick(2_200)]);
        assert_eq!(
            metric(&metrics, "median_confirmation_ms").outcome,
            MetricOutcome::Regressed
        );
    }

    #[test]
    fn fee_changes_are_not_rated() {
        let metrics = compare_metrics(&[landed(1, 5_000)], &[landed(1, 9_000)]);
        assert_eq!(
            metric(&metrics, "median_fee_lamports").outcome,
            MetricOutcome::Changed
        );
    }

    #[test]
    fn missing_evidence_stays_null_instead_of_zero() {
        let metrics = compare_metrics(&[simulation_only()], &[landed(1, 5_000)]);
        let landing = metric(&metrics, "landing_rate");
        assert_eq!(landing.baseline.value, None);
        assert_eq!(landing.baseline.sample_size, 0);
        assert_eq!(landing.delta, None);
        assert_eq!(landing.outcome, MetricOutcome::NoEvidence);
        let latency = metric(&metrics, "median_confirmation_ms");
        assert_eq!(latency.baseline.value, None);
        assert_eq!(latency.outcome, MetricOutcome::NoEvidence);
    }

    #[test]
    fn confirmation_before_submission_is_not_a_negative_latency() {
        let skewed = TraceEvidence {
            first_confirmed_at: Some(at(99)),
            ..landed(0, 5_000)
        };
        let metrics = compare_metrics(&[skewed], &[skewed]);
        assert_eq!(
            metric(&metrics, "median_confirmation_ms").baseline.value,
            None
        );
    }

    #[test]
    fn sample_floor_applies_to_both_cohorts() {
        let enough = vec![landed(1, 5_000); 30];
        let metrics = compare_metrics(&enough, &enough);
        assert!(metric(&metrics, "landing_rate").sufficient_sample);
        let metrics = compare_metrics(&enough, &enough[..29]);
        assert!(!metric(&metrics, "landing_rate").sufficient_sample);
    }
}
