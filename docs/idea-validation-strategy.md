# Landfall Idea Validation Strategy

Status: Draft for execution  
Version: 0.1  
Date: 2026-08-29  
Owner: Founder  
Working product name: **Landfall**

## 1. Purpose of this document

This document defines how to validate whether Landfall should become a commercial product before investing in a full hosted platform.

Landfall is currently a hypothesis, not a validated business. The working concept is a vendor-neutral observability and diagnostic layer for the complete lifecycle of Solana transactions. It would connect information available inside an application before submission with RPC responses and on-chain outcomes after submission. Its purpose would be to help transaction-heavy teams understand failures, improve landing reliability, reduce unnecessary fees, and investigate incidents faster.

The validation process must answer five questions:

1. Do the intended customers experience this problem frequently and severely enough?
2. Are existing tools insufficient for their actual workflows?
3. Will teams install instrumentation and share enough telemetry for Landfall to be useful?
4. Can Landfall produce actionable findings that customers trust?
5. Will a real buyer pay for a pilot and, later, for continuous monitoring?

The objective is not to collect compliments, GitHub stars, waitlist emails, or general enthusiasm about Solana. The objective is to collect behavioral evidence: access to real incidents, installation of instrumentation, allocation of engineering time, payment, continued usage, and referrals.

## 2. Working product hypothesis

### 2.1 One-sentence hypothesis

Small and mid-sized teams that submit business-critical Solana transactions will pay for a vendor-neutral observability product that correlates client-side transaction attempts with RPC delivery and on-chain outcomes, because their current tools cannot adequately explain transactions that are delayed, dropped before inclusion, executed unsuccessfully, duplicated, or made unnecessarily expensive.

### 2.2 Problem statement

Solana applications can observe different fragments of a transaction lifecycle in different places:

- Application logs know when a transaction was constructed, signed, submitted, retried, or abandoned.
- RPC providers know what their endpoints accepted or rejected, but a customer may use multiple providers or specialized senders.
- On-chain explorers know what was included in a block, but cannot describe a transaction that never landed.
- Simulations can reveal execution problems, but simulation state and execution state may differ.
- Fee estimators recommend bids, but do not necessarily measure the economic result for the customer's workload.

When these fragments are not correlated, engineers often rely on incomplete logs, provider dashboards, explorers, and manual reasoning. The resulting uncertainty can increase incident-resolution time, cause overpayment, encourage unsafe retries, and obscure whether a provider or application change improved reliability.

### 2.3 Proposed value proposition

Landfall should answer four questions for every instrumented transaction:

1. **What happened?** A timeline from construction and simulation through submission, observation, confirmation, finalization, failure, or expiration.
2. **How certain are we?** A diagnosis labeled as confirmed, probable, or unknown, with the supporting evidence shown.
3. **What should the team change?** Deterministic recommendations concerning blockhash freshness, commitment consistency, compute budget, priority fees, routes, and retries.
4. **Did the change work?** Comparative metrics for landing rate, latency, cost, failure categories, routes, transaction types, and releases.

### 2.4 Initial commercial wedge

The first commercial offer is not a generic SaaS subscription. It is a fixed-scope **Solana Transaction Reliability Audit** supported by lightweight Landfall instrumentation.

The audit will:

- instrument one production or staging transaction flow;
- collect sanitized telemetry for an agreed period;
- classify observable failures and unknowns;
- quantify landing rate, latency, retries, and fee behavior;
- identify the highest-impact changes;
- deliver a written report and an implementation proposal;
- compare before-and-after results when the customer applies changes.

This service-first wedge reduces the amount of software required before payment and reveals which analyses repeat across customers. Repeated analyses become the SaaS product.

## 3. Target market hypotheses

### 3.1 Primary ideal customer profile

The primary ICP is a Solana-native or Solana-heavy software team with the following properties:

- 3–20 engineers;
- a product already in beta or production;
- at least 10,000 submitted transactions per day, or a smaller number of transactions with high economic value;
- an internal backend, infrastructure, or trading system that constructs and submits transactions;
- use of one or more RPC providers or specialized submission paths;
- visible sensitivity to landing rate, time-to-inclusion, or transaction cost;
- no dedicated transaction-delivery observability team;
- a technical buyer who can approve a USD 500–3,000 pilot without enterprise procurement.

The 10,000-transactions-per-day threshold is a starting hypothesis, not a market fact. Interviews may show that economic value per transaction is a better segmentation variable than volume.

### 3.2 Priority segments

#### Segment A: trading and automation systems

Examples include arbitrage, liquidation, copy-trading, market-making, execution bots, Telegram trading tools, and automated treasury operations.

Expected characteristics:

- high sensitivity to latency and landing probability;
- direct economic cost when an opportunity is missed;
- frequent use of priority fees, Jito, multiple RPC routes, and aggressive retry strategies;
- stronger security and self-hosting requirements;
- greater willingness to pay if Landfall demonstrates measurable improvement.

Risks:

- sophisticated desks may already have internal systems;
- teams may consider their telemetry strategically sensitive;
- the product must never become a source of latency or key exposure.

#### Segment B: wallets, swaps, and consumer transaction flows

Examples include wallets, swap frontends, checkout flows, minting applications, and consumer dApps.

Expected characteristics:

- failed or delayed transactions damage conversion and user trust;
- teams need comparisons by app version, wallet, route, and transaction type;
- transaction volume may be high, while economic value per transaction varies;
- product and support teams may benefit from understandable incident reports.

Risks:

- client-side signing limits what can be captured reliably;
- privacy expectations are high;
- budgets may be lower than those of trading teams.

#### Segment C: payments, payouts, and stablecoin applications

Examples include merchant checkout, mass payouts, x402 services, payroll, remittance, and treasury automation.

Expected characteristics:

- correctness, auditability, and duplicate prevention matter more than minimal latency;
- retries and ambiguous confirmation can create operational risk;
- teams may value a self-hosted or auditable deployment;
- recurring monitoring may be easier to justify than for experimental dApps.

Risks:

- sales cycles and compliance review can be longer;
- integrations may require business-specific idempotency data;
- lower transaction rates may make controlled measurement slower.

### 3.3 Secondary and future customers

- RPC and transaction-routing providers seeking customer-facing diagnostics or white-label observability;
- Solana development agencies that want a repeatable reliability audit;
- larger enterprises requiring self-hosted monitoring;
- protocol teams monitoring integrator transaction outcomes.

These are not the initial focus because they require partnerships, procurement, or functionality that has not yet been validated.

### 3.4 Explicitly excluded early audiences

- retail token traders;
- students and developers using only devnet;
- projects without a working transaction flow;
- teams seeking a general block explorer;
- teams seeking automated trading strategy or alpha;
- the largest trading firms with mature proprietary infrastructure;
- customers who require private keys, custody, or transaction signing from Landfall.

## 4. Users, buyers, and stakeholders

Validation interviews must distinguish the person using the product from the person buying it.

| Role | Primary need | Evidence of pain | Purchasing role |
|---|---|---|---|
| Backend/infrastructure engineer | Reconstruct a failed transaction lifecycle and fix it | Manual log correlation, incident hours, ad hoc scripts | User and technical evaluator |
| On-call engineer | Identify whether the problem is the app, route, fee, or network | Slow incident triage and inconclusive alerts | User |
| CTO/technical founder | Improve reliability without hiring a protocol specialist | Lost revenue, support issues, missed launches | Economic buyer |
| Trading or operations lead | Increase successful execution and control cost | Missed opportunities, open positions, failed payouts | Buyer or sponsor |
| Security/compliance reviewer | Ensure telemetry does not expose secrets or sensitive strategy | Objections to hosted logging or raw transaction storage | Blocker/approver |

## 5. Assumptions to validate

The assumptions are ordered by risk. A technically successful product still fails if the first three are false.

### H1 — The problem is frequent and material

Hypothesis: Transaction-heavy Solana teams experience at least one material transaction-delivery or confirmation incident per month and spend meaningful engineering time diagnosing it.

Evidence sought:

- a recent incident described from memory and supported by logs, tickets, or screenshots;
- measurable consequences such as missed revenue, support volume, delayed launch, or engineer-hours;
- repeated rather than purely exceptional occurrence;
- an existing monitoring or manual diagnostic workflow.

Initial pass threshold:

- at least 9 of 15 qualified interviewees describe a relevant incident in the previous 90 days;
- at least 6 quantify four or more engineer-hours per month or a direct economic consequence;
- at least 4 call it one of their current top five infrastructure problems.

### H2 — Existing alternatives leave an important gap

Hypothesis: RPC dashboards, explorers, fee APIs, application logs, and internal scripts do not provide a complete, vendor-neutral lifecycle view.

Evidence sought:

- use of three or more tools during an incident;
- inability to determine whether a transaction left the application or reached an inclusion path;
- inability to compare providers and strategies on a common metric;
- unsafe or manual retry decisions;
- recurring custom scripts or spreadsheets.

Initial pass threshold:

- at least 8 of 15 qualified interviewees report a gap that Landfall's client-to-chain correlation could address;
- at least 5 currently maintain custom instrumentation or manually correlate data;
- no dominant existing product is identified as fully solving the job for the ICP.

### H3 — Customers will permit useful instrumentation

Hypothesis: Teams will install a non-blocking SDK, wrapper, or local collector if it does not handle private keys, allows redaction, and can run self-hosted.

Evidence sought:

- willingness to share a sanitized sample trace;
- willingness to install instrumentation in staging;
- willingness to install in production for a limited pilot;
- security requirements that can be satisfied without destroying product utility.

Initial pass threshold:

- 5 teams provide sanitized incident data;
- 3 install the prototype in staging;
- 2 run a time-limited production pilot;
- none discovers a fundamental security constraint that makes correlation impossible across the target segment.

### H4 — The analysis produces trusted, actionable findings

Hypothesis: Deterministic, evidence-linked classifications and recommendations can reduce investigation time or improve a chosen metric.

Evidence sought:

- customer engineers agree with the evidence behind confirmed diagnoses;
- probable diagnoses are clearly understood as hypotheses, not facts;
- at least one recommended code or configuration change is accepted;
- a before-and-after measurement is possible.

Initial pass threshold:

- 4 of 5 concierge diagnostics produce at least one finding the customer considers actionable;
- 3 customers implement or schedule a recommended change;
- at least 2 pilots show a measurable improvement in landing rate, latency, cost, duplicate avoidance, or incident-resolution time;
- no critical recommendation causes a production regression.

### H5 — A buyer will pay

Hypothesis: A technical buyer will pay for a reliability audit before a complete SaaS product exists.

Evidence sought, from strongest to weakest:

1. completed payment;
2. signed paid pilot or purchase order;
3. written approval with an agreed price and start date;
4. letter of intent with a named budget owner;
5. verbal willingness to pay;
6. waitlist signup.

Initial pass threshold:

- at least 2 paid pilots at USD 500 or more, **or** one paid engagement at USD 1,500 or more;
- at least one paid customer agrees to a follow-on integration, monitoring subscription, or second measurement period;
- the buyer is not a personal friend and is paying from a business budget.

### H6 — Continuous monitoring has recurring value

Hypothesis: After the audit, teams want ongoing regression detection, comparisons, alerts, and incident history.

Evidence sought:

- monitoring remains installed after the audit;
- the customer opens the dashboard without prompting;
- the customer requests an alert or recurring report;
- the customer will pay monthly rather than only for consulting.

Initial pass threshold:

- 3 pilot teams request continued monitoring;
- 2 accept a paid recurring plan at USD 100 per month or more, or negotiate an annual/self-hosted agreement;
- at least 2 distinct recurring use cases emerge across customers.

### H7 — The founder can repeatedly reach the market

Hypothesis: Qualified prospects can be found and contacted without an unsustainably expensive acquisition channel.

Initial pass threshold:

- 100 researched accounts produce at least 20 relevant conversations;
- at least 10 conversations involve the primary ICP;
- at least 3 pilots originate from repeatable sources such as GitHub, X, communities, referrals, or Upwork;
- at least one customer refers another qualified team.

These numerical thresholds are decision rules chosen for the experiment. They are not claims about industry averages and should be revised only before, not after, observing results.

## 6. Research principles

1. Ask about past behavior before future preferences.
2. Ask for a specific recent incident rather than an opinion about the concept.
3. Do not explain the proposed product until the current workflow and pain are understood.
4. Separate confirmed facts, customer statements, interpretations, and founder assumptions in notes.
5. Do not count compliments as evidence.
6. Do not offer a free pilot without receiving something scarce in return: data, engineering time, a case study, a testimonial, or a referral.
7. Do not claim that a dropped transaction can always be diagnosed. Unknown is a valid and important result.
8. Do not recommend a provider solely from a small or uncontrolled benchmark.
9. Protect keys, proprietary strategies, user addresses, RPC credentials, and business identifiers.
10. Predefine decision gates so that enthusiasm does not move the goalposts.

## 7. Validation program

The validation program has five stages. Each stage should produce evidence required by the next one.

## Stage 0 — Prepare the research system

Duration: 2–3 days.

Deliverables:

- one-page description of the problem, not a feature-heavy product pitch;
- a lead database with scoring and source fields;
- interview script;
- note-taking and evidence templates;
- a short privacy statement for shared logs;
- a manually produced example transaction-lifecycle report using synthetic or founder-generated data;
- a simple landing page or repository page with a call to book a transaction health check;
- a calendar link and business email.

Do not build authentication, billing, multi-tenancy, alerts, or a polished dashboard at this stage.

### Lead database fields

| Field | Description |
|---|---|
| Account | Company or project name |
| Product URL | Working product or documentation |
| Segment | Trading, wallet, payments, other |
| Stage | Beta, production, unknown |
| Contact | Name and role |
| Contact channel | GitHub, email, X, Discord, Upwork, referral |
| Trigger | Incident, launch, hiring, integration, public question |
| Suspected transaction volume/value | Evidence or estimate, clearly labeled |
| Infrastructure | RPC, sender, Jito, unknown |
| Pain evidence | Exact statement or link |
| Lead score | A, B, or C |
| Status | New, contacted, replied, interviewed, pilot, closed |
| Last contact | Date |
| Next action | Specific follow-up |
| Evidence permission | Private, anonymized, public |

### Lead scoring

- **A:** working product, explicit recent pain, and reachable technical decision-maker;
- **B:** working product and transaction-heavy use case, but no explicit pain signal;
- **C:** idea-stage project, hobbyist project, unclear use case, or no reachable owner.

Prioritize A, then B. Do not spend validation time on C unless referred by a strong source.

## Stage 1 — Problem interviews

Duration: 2 weeks, overlapping with lead generation.  
Target: 15–20 qualified interviews, with at least 5 from the primary segment and no more than 5 from any single subsegment.

### Recruitment sources

- GitHub issues and repositories containing active Solana transaction-submission code;
- public discussions of confirmation, expiration, RPC, priority-fee, or Jito problems;
- X posts from founders and engineers;
- developer channels operated by Solana ecosystem projects and infrastructure providers;
- recent hackathon, accelerator, and mainnet-launch projects;
- Upwork projects with an explicit Solana bot, RPC, transaction, or Jito requirement;
- referrals at the end of every relevant interview.

Community rules and platform terms must be followed. Public support channels should receive useful technical answers, not unsolicited advertising. Prospects found through Upwork must remain within Upwork's permitted communication and contracting flow.

### Interview opening

> I am researching how production Solana teams diagnose transactions that are delayed, dropped, expired, or executed unsuccessfully. I am developing a possible observability tool, but this conversation is research rather than a sales presentation. I would like to understand a recent real incident and your current workflow. I will not publish company-specific information without explicit permission.

### Core interview questions

1. What does your product do, and which user or system actions create Solana transactions?
2. Approximately how many transactions do you submit in a normal day and during peaks? Is volume or value more important?
3. Tell me about the most recent transaction-delivery or confirmation incident. What first showed that something was wrong?
4. Walk me through the investigation chronologically. Which logs, dashboards, explorers, RPC calls, or people were involved?
5. What did you know with certainty? What remained a guess?
6. How long did the investigation take, and who participated?
7. What was the operational or economic consequence?
8. How do you currently set blockhash commitment, compute limits, priority fees, Jito tips, routes, and retries?
9. How do you distinguish an execution failure from a transaction that never landed?
10. How do you prevent ambiguous retries from causing a duplicate business action?
11. Which provider or internal metrics do you monitor continuously?
12. What have you built internally to address these problems?
13. What prevents the current solution from being better?
14. What data could a third-party tool collect? What data must never leave your environment?
15. Who owns this problem and who would approve a paid solution?
16. What would have to be true for a small production pilot to be acceptable?
17. Who else has this problem and would give a candid answer?

Do not ask “Would you use this?” or “How much would you pay?” until a concrete problem and buying process have been established. When discussing price, propose an actual scoped pilot and observe the response.

### Interview evidence record

For every interview, capture:

- participant and role;
- ICP qualification;
- date and source;
- specific incident;
- frequency;
- current workflow;
- current alternatives;
- consequence;
- exact language used to describe pain;
- data-access constraints;
- buyer and approval path;
- follow-up commitment;
- hypothesis evidence for and against;
- confidence and unanswered questions.

### Stage 1 exit review

At the end of 15 qualified interviews:

- score H1, H2, H3, and H7;
- cluster incidents without forcing them into the proposed taxonomy;
- identify the segment with the strongest combination of frequency, consequence, access, and buyer urgency;
- select one transaction flow for the concierge diagnostic;
- revise the ICP and value proposition once, with reasons recorded.

If H1 fails, stop building the observability product and investigate another idea. If H1 passes but H2 fails, pivot toward an integration, reporting, or service layer around the dominant existing solution. If H3 fails, test a fully self-hosted or offline report workflow before stopping.

## Stage 2 — Concierge diagnostics

Duration: 2–3 weeks.  
Target: 5 teams, of which at least 3 match the primary ICP.

A concierge diagnostic uses minimal software and manual analysis to simulate the future product experience.

### Inputs requested

- application timestamps for transaction construction, signing, and submission;
- generated signature or message digest;
- recent blockhash and last valid block height;
- commitment and preflight settings;
- simulation response and units consumed, when available;
- requested compute limit and priority fee;
- RPC or submission route identifier, with credentials removed;
- submission response and errors;
- retry timestamps and whether bytes/signature changed;
- status observations;
- on-chain transaction/error, if one exists;
- application version and transaction-flow label;
- business outcome, represented by a sanitized correlation ID.

Raw serialized transactions and account addresses are optional and should be excluded or hashed unless essential and explicitly approved. Private keys, seed phrases, signing material, bearer tokens, and full credential-bearing RPC URLs must never be requested.

### Concierge output

Each team receives:

- a lifecycle timeline;
- a data-completeness assessment;
- confirmed, probable, and unknown classifications;
- supporting evidence for every classification;
- aggregate reliability and latency metrics where sample size permits;
- prioritized recommendations;
- limitations and alternative explanations;
- a proposed instrumentation and measurement plan;
- a fixed-scope paid pilot proposal if there is a credible next step.

### Free diagnostic exchange

A free diagnostic is permitted only if the team agrees to at least two of the following:

- provide a real sanitized dataset;
- allocate an engineer for integration and review;
- permit an anonymized case study;
- provide a testimonial if the result is useful;
- introduce one qualified team;
- evaluate a paid pilot by a specific date.

## Stage 3 — Paid pilot

Duration: 2–4 weeks per customer.  
Initial price: USD 500–1,500 for the first two customers; later pilots should move toward USD 1,500–3,000 as scope and credibility improve.

Pricing is a validation instrument. Discounts must be explicit and tied to case-study access or unusually high learning value.

### Standard pilot scope

- one application;
- one environment;
- one high-value transaction flow;
- one or two submission routes;
- instrumentation support;
- 7–14 days of measurement;
- one baseline report;
- one recommendation workshop;
- one after-change report when timing permits;
- no production signing or custody;
- no promise of a specific landing-rate improvement before baseline measurement.

### Pilot success contract

Before starting, customer and founder must agree on:

- target transaction flow;
- baseline period;
- primary success metric;
- minimum useful sample size;
- allowed data and redaction policy;
- deployment model;
- named technical owner;
- report date;
- acceptable change risk;
- definition of an actionable finding;
- price and payment schedule;
- case-study permission.

Possible primary metrics:

- landing rate;
- p50/p95 time-to-processed or time-to-confirmed;
- cost per landed transaction;
- expiration rate;
- duplicate or ambiguous retry rate;
- incident mean time to diagnose;
- percentage of attempts with sufficient diagnostic data.

### What counts as pilot success

A pilot succeeds if it delivers at least one of:

- a verified improvement to the agreed metric;
- a previously unknown and economically meaningful cause;
- a safe configuration or code change accepted by the customer;
- a material reduction in investigation time;
- a defensible conclusion that disproves a suspected cause and prevents wasted work.

A pilot does not count as successful merely because telemetry was displayed in a dashboard.

## Stage 4 — Recurring product test

The hosted or self-hosted recurring product is tested only after at least one paid pilot produces actionable value.

Offer pilot customers a simple continuation plan containing only the features they requested repeatedly, such as:

- retained transaction timelines;
- release and route comparisons;
- regression alerts;
- weekly reliability report;
- incident investigation links;
- configurable retention;
- self-hosted collector;
- support during incidents.

Test three price anchors rather than asking an abstract willingness-to-pay question:

- Starter: USD 99/month with limited events and retention;
- Team: USD 299–499/month with alerts, longer retention, and multiple routes;
- Self-hosted/Business: annual contract or monthly minimum negotiated around support and deployment requirements.

These are test prices, not committed public pricing. Usage-based pricing may be tested if customers consider transaction count predictable and fair. Avoid pricing that penalizes customers during incidents without a protective cap.

## 8. Outreach strategy

### 8.1 Research message

> Hi {{name}} — I saw {{specific product, issue, or post}}. I am researching how production Solana teams diagnose transactions that are delayed, dropped before inclusion, expired, or executed unsuccessfully. Could I ask you 5–6 questions about a recent incident and your current workflow? I am not asking you to buy anything. In return, I can review one sanitized incident and share what I find.

### 8.2 Problem-aware diagnostic message

> Hi {{name}} — your note about {{specific symptom}} caught my attention. A signature alone often cannot distinguish an RPC delivery problem from expiration or an inclusion issue, but application-side timestamps, blockhash data, submission responses, and status observations can usually narrow the possibilities. I am building a vendor-neutral transaction lifecycle diagnostic and can review one sanitized incident. Would a 20-minute technical call be useful?

### 8.3 Pilot transition

> Based on the logs, the unresolved issue is {{customer's words and evidence}}. I can instrument one transaction flow, measure it for {{period}}, and deliver a reliability report plus prioritized changes. The fixed pilot scope is {{scope}} for {{price}}. If the data cannot support a conclusion, the report will say so rather than manufacture a diagnosis.

### 8.4 Follow-up cadence

- Day 0: personalized first message;
- Day 3 or 4: one follow-up adding a useful observation or question;
- Day 9 or 10: final follow-up offering a checklist or short resource;
- then stop unless the prospect produces a new public signal.

No mass automated direct messaging should be used during validation. Thirty relevant, researched messages are more useful than one thousand generic messages in a narrow technical market.

## 9. Weekly operating cadence

### Daily

- research 10 accounts;
- qualify at least 5;
- send 3–5 personalized messages;
- follow up on due conversations;
- record evidence immediately after calls;
- spend no more than 60 minutes building functionality not required by an active diagnostic or pilot.

### Weekly

- conduct 3–5 interviews;
- review conversion by source and segment;
- update the assumption scorecard;
- publish one useful technical observation when evidence can be shared safely;
- ask every relevant participant for one introduction;
- decide which requested feature is repeated and which is a one-customer customization;
- maintain a written list of evidence against the idea.

### Funnel metrics

Track separately by source and segment:

- accounts researched;
- qualified leads;
- messages sent;
- reply rate;
- interview acceptance rate;
- completed interview rate;
- sanitized dataset rate;
- diagnostic acceptance rate;
- pilot proposal rate;
- paid pilot close rate;
- continuation rate;
- referral rate;
- median sales-cycle duration.

Do not optimize message volume before verifying that replies come from the target customer.

## 10. Evidence quality hierarchy

| Strength | Evidence |
|---|---|
| 7 — strongest | Recurring payment and continued production installation |
| 6 | Completed paid pilot with an accepted result |
| 5 | Installation in production and allocation of engineering time |
| 4 | Access to a real sanitized incident dataset |
| 3 | Detailed recent incident with corroborating artifacts |
| 2 | Verbal commitment, introduction, or scheduled next step |
| 1 | Opinion, compliment, waitlist signup, social engagement |

Conclusions should state both the amount and strength of evidence. For example, “12 people liked the idea” is weak; “three teams installed the collector and two paid” is strong.

## 11. Decision gates

### Gate A — Problem validation

Evaluate after 15 qualified interviews.

Continue if H1 and H2 pass.  
Narrow or reposition if H1 passes only within one segment.  
Stop or change the idea if the pain is rare, immaterial, or already adequately solved.

### Gate B — Data access and utility

Evaluate after 5 concierge diagnostics.

Continue if H3 and H4 pass.  
Move to self-hosted/offline analysis if value exists but hosted telemetry is unacceptable.  
Stop if useful classifications require data customers cannot or will not provide.

### Gate C — Commercial validation

Evaluate after at least 5 qualified pilot proposals.

Continue if H5 passes.  
Change segment, scope, or offer if customers provide data but do not pay.  
Stop treating the project as a startup if ten qualified buyers decline paid pilots for similar reasons. It may remain an open-source portfolio project.

### Gate D — Recurring value

Evaluate after 3 completed pilots.

Build the hosted recurring product if H6 passes.  
Remain a consulting/productized-service business if audits sell but subscriptions do not.  
Investigate Idea 4 or another market if neither audits nor recurring monitoring attract payment.

## 12. Pivot indicators

Evidence may support a narrower or different product:

| Observed evidence | Possible pivot |
|---|---|
| Teams mainly need a correct retry implementation | Open-source retry/idempotency SDK plus paid integration |
| Provider comparisons dominate demand | Vendor-neutral route benchmarking and health monitoring |
| Fee waste is more urgent than failures | Fee policy and cost optimization product |
| Customers refuse hosted telemetry | Local CLI/self-hosted appliance with exportable reports |
| Wallet teams need support tooling | User-facing transaction incident lookup for support teams |
| Audits sell but software does not | Productized Solana reliability consultancy |
| Payments teams care primarily about reconciliation | Reassess the stablecoin reconciliation concept |

## 13. Invalid evidence and common traps

Do not treat the following as validation:

- building a technically impressive Rust system;
- winning a hackathon without customer adoption;
- GitHub stars from other developers;
- traffic caused by a general Solana tutorial;
- free users who will not provide data or engineering time;
- a prospect saying “interesting” without a next action;
- an investor or infrastructure provider praising the market;
- performance measurements taken only on devnet;
- an improvement observed without a stable baseline or comparable workload;
- one customer requesting a highly specific custom feature;
- a diagnosis inferred solely from the absence of an on-chain signature.

## 14. Security and research ethics

- Never request a seed phrase, private key, signer access, custody, or remote access to a wallet.
- Prefer sanitized structured events over arbitrary application logs.
- Remove credentials from RPC URLs and headers.
- Hash or omit user wallet addresses when identity is unnecessary.
- Assign a random correlation ID rather than ingesting customer business IDs.
- Agree on retention and deletion before a pilot.
- Use least-privilege access and customer-controlled export.
- Do not publish provider rankings from customer data without permission and a reproducible methodology.
- Do not identify customers, transaction strategies, or incidents without written approval.
- Clearly label confirmed, probable, and unknown diagnoses.
- Avoid guarantees of transaction landing or financial returns.

## 15. Four-week initial execution plan

### Week 1

- create the first 50-account lead list;
- complete 5 interviews;
- produce one synthetic lifecycle report;
- refine the problem language using customer words;
- obtain at least one sanitized incident.

### Week 2

- reach 15 total qualified interviews;
- score Gate A;
- choose the strongest subsegment;
- perform the first two concierge diagnostics;
- define the minimum event schema required by real evidence.

### Week 3

- complete five concierge diagnostics;
- score Gate B;
- build only the collector/adapter functionality required to reduce repeated manual work;
- send at least three fixed-scope pilot proposals.

### Week 4

- close the first paid pilot or document why proposals failed;
- install instrumentation for the pilot;
- define the baseline and success metric;
- decide whether to proceed with the PRD's MVP, narrow it, or keep the project portfolio-only.

## 16. Validation scorecard

Update this table weekly without retroactively changing thresholds.

| Hypothesis | Threshold | Current evidence | Status | Decision |
|---|---|---|---|---|
| H1: frequent, material problem | 9/15 recent incidents; 6/15 material cost | None yet | Untested | — |
| H2: alternatives leave a gap | 8/15 lifecycle gap; 5/15 manual tooling | None yet | Untested | — |
| H3: instrumentation acceptable | 5 datasets; 3 staging; 2 production | None yet | Untested | — |
| H4: analysis is actionable | 4/5 useful; 3 changes accepted; 2 measurable outcomes | None yet | Untested | — |
| H5: buyers pay | 2 × USD 500 or 1 × USD 1,500 | None yet | Untested | — |
| H6: recurring value | 3 request continuation; 2 pay USD 100+/month | None yet | Untested | — |
| H7: repeatable access | 20 conversations/100 accounts; 3 pilots; 1 referral | None yet | Untested | — |

## 17. Final validation rule

Landfall is commercially validated only when all of the following are true:

1. A narrowly defined customer segment repeatedly experiences the problem.
2. The team can provide sufficient telemetry without unacceptable risk.
3. Landfall produces an actionable or cost-saving result using that telemetry.
4. At least one unrelated business pays a meaningful price for that result.
5. A repeatable path exists to find the next similar customer.

Until then, every product feature is an experiment, and every experiment must identify the assumption it is intended to test.
