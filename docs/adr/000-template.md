# ADR-NNN: Short decision title

- **Status:** Proposed
- **Date:** YYYY-MM-DD
- **Decision owners:** Name or role
- **Related requirements:** Requirement IDs or `None`
- **Related ADRs:** ADR links or `None`
- **Supersedes:** ADR link or `None`
- **Superseded by:** ADR link or `None`

## Context

Describe the concrete problem, uncertainty, or conflict that requires a durable
decision. State the current situation and why leaving the choice implicit would
create implementation, product, security, or operational risk.

Keep this section factual. Do not present the preferred solution as inevitable.

## Decision drivers

List the forces that distinguish a good option from a poor one. Include relevant
product constraints, expected scale, team size, delivery stage, compatibility,
security, privacy, reliability, performance, cost, and operational simplicity.

- Driver one.
- Driver two.
- Driver three.

## Options considered

### Option A — Descriptive name

Explain how the option would work in Landfall, including meaningful advantages,
drawbacks, risks, and assumptions.

### Option B — Descriptive name

Explain how the option would work in Landfall, including meaningful advantages,
drawbacks, risks, and assumptions.

Add further realistic options when necessary. Include “defer the decision” or
“do nothing” when either is genuinely viable.

## Decision

State the selected option unambiguously and at an implementation-relevant level.
Separate the binding decision from examples and possible future improvements.

## Detailed design and boundaries

Explain the component boundaries, data flow, ownership, invariants, public
contracts, and failure behavior established by this decision. Include a small
diagram or example only when it materially improves understanding.

Explicitly state what this ADR does not decide.

## Consequences

### Positive

- Benefit we gain.

### Negative

- Cost, limitation, or complexity we knowingly accept.

### Risks and mitigations

| Risk | Impact | Mitigation or detection |
|---|---|---|
| Describe a credible risk | Describe the failure or harm | Describe the control, test, metric, or fallback |

## Security and privacy impact

Describe changes to trust boundaries, authentication, authorization, sensitive
data, secrets, retention, logging, abuse resistance, and threat-model entries.
Write `No material impact` only after checking each category.

## Reliability and failure behavior

Describe normal failure modes, retry/idempotency expectations, consistency,
recovery, degradation, and what callers or operators observe.

## Performance and capacity impact

Record expected workload, important limits, likely bottlenecks, and the metric or
threshold that would trigger reconsideration. Link the capacity model when it is
relevant.

## Operational impact

Describe deployment, configuration, migrations, monitoring, alerting, backup,
rollback, support, and local-development consequences.

## Verification

List the tests, benchmarks, review evidence, or production metrics that will
demonstrate the decision is implemented correctly. Prefer falsifiable exit
criteria over statements such as “test thoroughly.”

- Verification item one.
- Verification item two.

## Rollout and migration

Explain how the decision is introduced, how existing data or clients migrate,
and how to roll back when rollback is possible. State `Not applicable` with a
reason when no rollout is involved.

## Reconsideration triggers

List observable conditions that justify reopening the decision without implying
that it is temporary by default.

- A measurable scale, cost, reliability, or compatibility threshold.
- A product requirement or threat-model change.

## References

- Link to the relevant PRD section.
- Link to the relevant system-design section.
- Link to authoritative external specifications or measurements when used.
