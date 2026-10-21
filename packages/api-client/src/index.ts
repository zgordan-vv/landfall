/** Small fetch-based client for the documented Landfall read APIs. */
export interface FetchResponse {
  readonly ok: boolean;
  readonly status: number;
  json(): Promise<unknown>;
  arrayBuffer?(): Promise<ArrayBuffer>;
}
export interface FetchInit {
  method?: string;
  headers?: Record<string, string>;
  body?: string;
}
export type FetchLike = (input: string, init?: FetchInit) => Promise<FetchResponse>;

export interface ApiClientOptions {
  baseUrl: string;
  fetch?: FetchLike;
  token?: string;
}
export interface MetricSummary {
  numerator: number;
  denominator: number;
  excluded_in_flight: number;
  excluded_unknown: number;
  definition_version: string;
}
export interface DataQualitySummary {
  assessments: number;
  grades: Array<{ grade: string; count: number }>;
  gaps: Array<{ gap: string; count: number }>;
  average_score: number | null;
  definition_version: string;
}
export interface DetailedSystemStatus {
  status: "ok" | "degraded";
  database_ready: boolean;
  queue_depth: number;
  projection_lag_seconds: number;
  observer_routes: number;
  unhealthy_observer_routes: number;
  schema_version: string;
  rule_set_version: string;
  retention_days: number;
}
export interface SystemHealthSummary {
  status: "ok" | "degraded";
  database_ready: boolean;
  projects: number;
  environments: number;
  enabled_routes: number;
  queued_jobs: number;
  dead_letter_jobs: number;
  events_last_24h: number;
}
/** What a comparison cohort is: its environment and the operator's note on what differs. */
export interface CohortEnvironment {
  environment_id: string;
  name: string;
  cluster: string;
  description: string | null;
}
export interface CohortSummary {
  environment: CohortEnvironment;
  /** Traces whose first event falls inside the window. */
  traces: number;
  /** Traces with at least one submission attempt. */
  submitted_traces: number;
  sufficient_sample: boolean;
}
export interface MetricValue {
  /** `null` when the cohort has no evidence for this metric. */
  value: number | null;
  /** Count of traces meeting the condition, for rate metrics only. */
  numerator: number | null;
  /** Denominator of a rate, or the number of traces behind a median. */
  sample_size: number;
}
export interface MetricComparison {
  key: string;
  label: string;
  unit: "ratio" | "milliseconds" | "lamports";
  better_when: "higher" | "lower" | "neutral";
  definition: string;
  baseline: MetricValue;
  candidate: MetricValue;
  /** Candidate minus baseline; `null` when either side has no evidence. */
  delta: number | null;
  outcome: "improved" | "regressed" | "unchanged" | "changed" | "no_evidence";
  sufficient_sample: boolean;
}
export interface ComparisonReport {
  status: "ready" | "not_configured";
  role_source: "query" | "project_setting" | null;
  window: { hours: number; start: string; end: string };
  minimum_sample: number;
  definitions_version: string;
  baseline: CohortSummary | null;
  candidate: CohortSummary | null;
  sufficient_sample: boolean;
  metrics: MetricComparison[];
  environments: CohortEnvironment[];
}
export interface ComparisonOptions {
  /** Explicit roles; pass both or neither. Without them the project's designated cohorts apply. */
  baselineEnvironmentId?: string;
  candidateEnvironmentId?: string;
  /** Window length ending now, 1 to 2160 hours (server default 168). */
  windowHours?: number;
}
export interface ComparisonCohorts {
  project_id: string;
  baseline_environment_id: string;
  candidate_environment_id: string;
  updated_at: string;
}
export interface TraceDiagnostic {
  diagnostic_id: string;
  rule_id: string;
  claim_key: string;
  certainty: "confirmed" | "probable" | "unknown";
  /** Machine-readable gap behind an unknown finding, e.g. `missing_simulation_evidence`. */
  unknown_reason: string | null;
  /** Raw event IDs the rule evaluated; they match `TraceTimelineEvent.event_id`. */
  evidence_event_ids: string[];
}
export interface TraceRecommendation {
  recommendation_id: string;
  recommendation_key: string;
  rule_set_version: string;
}
export interface TraceDetail {
  trace_id: string;
  environment_id: string;
  environment_name: string;
  flow: string | null;
  lifecycle_state: string;
  landing_state: string;
  execution_state: string;
  application_state: string;
  observation_state: string;
  updated_at: string;
  evidence: TraceEvidence;
}
export interface TraceEvidence {
  trace_created: boolean;
  signing_completed: boolean;
  submission_completed: boolean;
  status_observed: boolean;
  simulation_completed: boolean;
}
export interface TraceListDiagnosis {
  rule_id: string;
  claim_key: string;
  certainty: "confirmed" | "probable" | "unknown";
}
export interface TraceListItem {
  trace_id: string;
  environment_id: string;
  environment_name: string;
  flow: string | null;
  lifecycle_state: string;
  landing_state: string;
  execution_state: string;
  application_state: string;
  updated_at: string;
  top_diagnosis: TraceListDiagnosis | null;
  diagnosis_count: number;
}
/** Normalized error without its free-form message. */
export interface TraceTimelineError {
  category: string;
  code?: string;
  instruction_index?: number;
  custom_code?: number;
}
/**
 * Allowlisted, privacy-safe event attributes. Decimal values (durations in
 * nanoseconds, slots, lamports, compute units) are strings to keep u64 precision.
 */
export interface TraceTimelineAttributes {
  flow?: string;
  transaction_version?: string;
  result?: string;
  transport_result?: string;
  rpc_result?: string;
  source_result?: string;
  execution_result?: string;
  outcome?: string;
  reason?: string;
  commitment?: string;
  observed_commitment?: string;
  preflight_commitment?: string;
  duration_ns?: string;
  delay_ns?: string;
  timeout_ns?: string;
  slot?: string;
  context_slot?: string;
  block_height?: string;
  last_valid_block_height?: string;
  confirmations?: string;
  block_time?: string;
  units_consumed?: string;
  compute_units_consumed?: string;
  fee_lamports?: string;
  route_id?: string;
  route_name?: string;
  next_route_id?: string;
  next_route_name?: string;
  attempt_id?: string;
  previous_attempt_id?: string;
  attempt_sequence?: number;
  retry_sequence?: number;
  max_retries?: number;
  required_signatures?: number;
  skip_preflight?: boolean;
  logs_present?: boolean;
  quality_category?: string;
  quality_severity?: string;
  quality_impact?: string;
  error?: TraceTimelineError;
}
export interface TraceTimelineEvent {
  event_id: string;
  event_type: string;
  occurred_at: string;
  source_kind: string | null;
  source_name: string | null;
  attributes: TraceTimelineAttributes;
}
export interface TraceTimeline {
  trace_id: string;
  /** `canonical` when the reducer order was applied, `occurred_at` otherwise. */
  ordering: "canonical" | "occurred_at";
  events: TraceTimelineEvent[];
}
export interface OverviewSummary {
  window_hours: number;
  total_traces: number;
  landed_traces: number;
  successful_executions: number;
  unknown_executions: number;
  updated_at: string;
}
export interface ProjectResponse {
  project_id: string;
  name: string;
}
export interface CreatedTokenResponse {
  token_id: string;
  name: string;
  token_prefix: string;
  token: string;
  scopes: string[];
  expires_at: string | null;
}
export interface CreatedProjectResponse {
  project: ProjectResponse;
  initial_token: CreatedTokenResponse;
}
export interface EnvironmentResponse {
  environment_id: string;
  project_id: string;
  name: string;
  cluster: string;
  description: string | null;
}
export interface RouteResponse {
  route_id: string;
  environment_id: string;
  name: string;
  enabled: boolean;
}
export interface RouteVerificationResponse {
  route_id: string;
  reachable: boolean;
  latency_ms: number;
  network_identity: string | null;
}
export interface TokenResponse {
  token_id: string;
  name: string;
  token_prefix: string;
  scopes: string[];
  expires_at: string | null;
  revoked_at: string | null;
  last_used_at: string | null;
}
export interface X402PaymentAuditRecord {
  audit_id: string;
  policy_id: string | null;
  agent_id: string;
  merchant_origin: string;
  network: string;
  asset: string;
  amount_atomic: string;
  decision: "approved" | "denied" | "settled" | "failed";
  reason_code: string;
  settlement_reference: string | null;
  decided_at: string;
}
export interface ReportResponse {
  report_id: string;
  title: string;
  status: string;
  privacy_profile: "internal" | "shareable";
  trace_count: number;
  created_at: string;
}
export interface ProjectConfig {
  project_id: string;
  display_name: string;
  environments: Array<{
    environment_id: string;
    display_name: string;
    cluster: string;
    routes: Array<{
      route_id: string;
      label: string;
      enabled: boolean;
      endpoint_configured: boolean;
    }>;
  }>;
}

export class LandfallApiClient {
  private readonly baseUrl: string;
  private readonly request: FetchLike;
  private readonly token: string | undefined;

  constructor(options: ApiClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    const defaultFetch = (globalThis as { fetch?: FetchLike }).fetch;
    if (!options.fetch && !defaultFetch) throw new Error("a fetch implementation is required");
    const browserFetch = defaultFetch as FetchLike;
    this.request = options.fetch ?? ((input, init) => browserFetch.call(globalThis, input, init));
    this.token = options.token;
  }

  async getMetricSummary(): Promise<MetricSummary> {
    return this.get("/api/v1/metrics/summary");
  }
  async getDataQualitySummary(): Promise<DataQualitySummary> {
    return this.get("/api/v1/data-quality/summary");
  }
  async getSystemStatus(): Promise<SystemHealthSummary> {
    return this.get("/v1/system/status");
  }
  async getComparison(options: ComparisonOptions = {}): Promise<ComparisonReport> {
    const query = [
      ["baseline_environment_id", options.baselineEnvironmentId],
      ["candidate_environment_id", options.candidateEnvironmentId],
      ["window_hours", options.windowHours?.toString()],
    ]
      .filter((pair): pair is [string, string] => pair[1] !== undefined)
      .map(([key, value]) => `${key}=${encodeURIComponent(value)}`)
      .join("&");
    return this.get(`/v1/comparison${query ? `?${query}` : ""}`);
  }
  async getTraceDiagnostics(traceId: string): Promise<TraceDiagnostic[]> {
    return this.get(`/v1/traces/${encodeURIComponent(traceId)}/diagnostics`);
  }
  async getTraceRecommendations(traceId: string): Promise<TraceRecommendation[]> {
    return this.get(`/v1/traces/${encodeURIComponent(traceId)}/recommendations`);
  }
  async getTraceTimeline(traceId: string): Promise<TraceTimeline> {
    return this.get(`/v1/traces/${encodeURIComponent(traceId)}/timeline`);
  }
  async getTraceDetail(traceId: string): Promise<TraceDetail> {
    return this.get(`/v1/traces/${encodeURIComponent(traceId)}`);
  }
  async getTraces(limit = 50): Promise<TraceListItem[]> {
    return this.get(`/v1/traces?limit=${Math.min(100, Math.max(1, limit))}`);
  }
  async getOverview(): Promise<OverviewSummary> {
    return this.get("/v1/overview");
  }
  async createProject(
    token: string,
    name: string,
    initialTokenName: string,
  ): Promise<CreatedProjectResponse> {
    return this.post("/v1/control/projects", { name, initial_token_name: initialTokenName }, token);
  }
  async createEnvironment(
    projectId: string,
    token: string,
    name: string,
    cluster: string,
  ): Promise<EnvironmentResponse> {
    return this.post(
      `/v1/control/projects/${encodeURIComponent(projectId)}/environments`,
      { name, cluster },
      token,
    );
  }
  async setEnvironmentDescription(
    projectId: string,
    environmentId: string,
    token: string,
    description: string | null,
  ): Promise<EnvironmentResponse> {
    return this.put(
      `/v1/control/projects/${encodeURIComponent(projectId)}/environments/${encodeURIComponent(environmentId)}/description`,
      { description },
      token,
    );
  }
  async getComparisonCohorts(projectId: string, token: string): Promise<ComparisonCohorts> {
    return this.getWithToken(
      `/v1/control/projects/${encodeURIComponent(projectId)}/comparison-cohorts`,
      token,
    );
  }
  async setComparisonCohorts(
    projectId: string,
    token: string,
    baselineEnvironmentId: string,
    candidateEnvironmentId: string,
  ): Promise<ComparisonCohorts> {
    return this.put(
      `/v1/control/projects/${encodeURIComponent(projectId)}/comparison-cohorts`,
      {
        baseline_environment_id: baselineEnvironmentId,
        candidate_environment_id: candidateEnvironmentId,
      },
      token,
    );
  }
  async createRoute(
    projectId: string,
    environmentId: string,
    token: string,
    name: string,
    endpoint: string,
  ): Promise<RouteResponse> {
    return this.post(
      `/v1/control/projects/${encodeURIComponent(projectId)}/environments/${encodeURIComponent(environmentId)}/routes`,
      { name, endpoint },
      token,
    );
  }
  async verifyRoute(
    projectId: string,
    environmentId: string,
    routeId: string,
    token: string,
  ): Promise<RouteVerificationResponse> {
    return this.post(
      `/v1/control/projects/${encodeURIComponent(projectId)}/environments/${encodeURIComponent(environmentId)}/routes/${encodeURIComponent(routeId)}/verify`,
      {},
      token,
    );
  }
  async createToken(
    projectId: string,
    token: string,
    name: string,
    scopes: string[],
    expiresAt?: string,
  ): Promise<CreatedTokenResponse> {
    return this.post(
      `/v1/control/projects/${encodeURIComponent(projectId)}/tokens`,
      { name, scopes, ...(expiresAt === undefined ? {} : { expires_at: expiresAt }) },
      token,
    );
  }
  async listTokens(projectId: string, token: string): Promise<TokenResponse[]> {
    return this.getWithToken(`/v1/control/projects/${encodeURIComponent(projectId)}/tokens`, token);
  }
  async revokeToken(projectId: string, tokenId: string, token: string): Promise<void> {
    await this.post(
      `/v1/control/projects/${encodeURIComponent(projectId)}/tokens/${encodeURIComponent(tokenId)}/revoke`,
      {},
      token,
    );
  }
  async getX402PaymentAudit(
    projectId: string,
    token: string,
    limit = 50,
  ): Promise<X402PaymentAuditRecord[]> {
    return this.getWithToken(
      `/v1/control/projects/${encodeURIComponent(projectId)}/x402/audit?limit=${Math.min(100, Math.max(1, limit))}`,
      token,
    );
  }
  async getProjectConfig(projectId: string, token: string): Promise<ProjectConfig> {
    return this.getWithToken(`/v1/control/projects/${encodeURIComponent(projectId)}/config`, token);
  }
  async createReport(
    projectId: string,
    token: string,
    title: string,
    privacyProfile: "internal" | "shareable",
  ): Promise<ReportResponse> {
    return this.post(
      `/v1/control/projects/${encodeURIComponent(projectId)}/reports`,
      { title, privacy_profile: privacyProfile },
      token,
    );
  }
  async listReports(projectId: string, token: string): Promise<ReportResponse[]> {
    return this.getWithToken(
      `/v1/control/projects/${encodeURIComponent(projectId)}/reports`,
      token,
    );
  }
  async downloadReport(
    projectId: string,
    reportId: string,
    format: "json" | "html",
    token: string,
  ): Promise<Uint8Array> {
    const response = await this.request(
      `${this.baseUrl}/v1/control/projects/${encodeURIComponent(projectId)}/reports/${encodeURIComponent(reportId)}/${format}`,
      { method: "GET", headers: { authorization: `Bearer ${token}` } },
    );
    if (!response.ok) throw new Error(`Landfall API request failed: ${response.status}`);
    if (!response.arrayBuffer)
      throw new Error("The active fetch implementation cannot download report bytes");
    return new Uint8Array(await response.arrayBuffer());
  }

  private async get<T>(path: string): Promise<T> {
    const headers: Record<string, string> = { accept: "application/json" };
    if (this.token) headers["authorization"] = `Bearer ${this.token}`;
    const response = await this.request(`${this.baseUrl}${path}`, { method: "GET", headers });
    if (!response.ok) throw new Error(`Landfall API request failed: ${response.status}`);
    return (await response.json()) as T;
  }

  private async post<T>(path: string, body: unknown, token: string): Promise<T> {
    return this.send("POST", path, body, token);
  }

  private async put<T>(path: string, body: unknown, token: string): Promise<T> {
    return this.send("PUT", path, body, token);
  }

  private async send<T>(method: string, path: string, body: unknown, token: string): Promise<T> {
    const response = await this.request(`${this.baseUrl}${path}`, {
      method,
      headers: {
        accept: "application/json",
        "content-type": "application/json",
        authorization: `Bearer ${token}`,
      },
      body: JSON.stringify(body),
    });
    if (!response.ok) throw new Error(`Landfall API request failed: ${response.status}`);
    return (await response.json()) as T;
  }

  private async getWithToken<T>(path: string, token: string): Promise<T> {
    const response = await this.request(`${this.baseUrl}${path}`, {
      method: "GET",
      headers: { accept: "application/json", authorization: `Bearer ${token}` },
    });
    if (!response.ok) throw new Error(`Landfall API request failed: ${response.status}`);
    return (await response.json()) as T;
  }
}
