/** Small fetch-based client for the documented Landfall read APIs. */
export interface FetchResponse { readonly ok: boolean; readonly status: number; json(): Promise<unknown>; }
export interface FetchInit { method?: string; headers?: Record<string, string>; body?: string; }
export type FetchLike = (input: string, init?: FetchInit) => Promise<FetchResponse>;

export interface ApiClientOptions { baseUrl: string; fetch?: FetchLike; token?: string; }
export interface MetricSummary { numerator: number; denominator: number; excluded_in_flight: number; excluded_unknown: number; definition_version: string; }
export interface DataQualitySummary { assessments: number; grades: Array<{ grade: string; count: number }>; gaps: Array<{ gap: string; count: number }>; average_score: number | null; definition_version: string; }
export interface CohortComparison { baseline_rate: number | null; candidate_rate: number | null; absolute_change: number | null; relative_change: number | null; baseline_sample_size: number; candidate_sample_size: number; baseline_missing_data_rate: number; candidate_missing_data_rate: number; small_sample_warning: boolean; metric_definition: string; }
export interface DetailedSystemStatus { status: "ok" | "degraded"; database_ready: boolean; queue_depth: number; projection_lag_seconds: number; observer_routes: number; unhealthy_observer_routes: number; schema_version: string; rule_set_version: string; retention_days: number; }
export interface SystemHealthSummary { status: "ok" | "degraded"; database_ready: boolean; projects: number; environments: number; enabled_routes: number; queued_jobs: number; dead_letter_jobs: number; events_last_24h: number; }
export interface ComparisonSummary { baseline_environment_id: string; candidate_environment_id: string; baseline_traces: number; candidate_traces: number; baseline_landed: number; candidate_landed: number; }
export interface TraceDiagnostic { diagnostic_id: string; rule_id: string; claim_key: string; certainty: "confirmed" | "probable" | "unknown"; }
export interface TraceRecommendation { recommendation_id: string; recommendation_key: string; rule_set_version: string; }
export interface TraceDetail { trace_id: string; lifecycle_state: string; landing_state: string; execution_state: string; application_state: string; observation_state: string; updated_at: string; }
export interface TraceListItem { trace_id: string; lifecycle_state: string; landing_state: string; execution_state: string; updated_at: string; }
export interface OverviewSummary { window_hours: number; total_traces: number; landed_traces: number; successful_executions: number; unknown_executions: number; updated_at: string; }
export interface ProjectResponse { project_id: string; name: string; }
export interface CreatedTokenResponse { token_id: string; name: string; token_prefix: string; token: string; scopes: string[]; expires_at: string | null; }
export interface CreatedProjectResponse { project: ProjectResponse; initial_token: CreatedTokenResponse; }
export interface EnvironmentResponse { environment_id: string; project_id: string; name: string; cluster: string; }
export interface RouteResponse { route_id: string; environment_id: string; name: string; enabled: boolean; }
export interface X402PaymentAuditRecord { audit_id: string; policy_id: string | null; agent_id: string; merchant_origin: string; network: string; asset: string; amount_atomic: string; decision: "approved" | "denied" | "settled" | "failed"; reason_code: string; settlement_reference: string | null; decided_at: string; }

export class LandfallApiClient {
  private readonly baseUrl: string;
  private readonly request: FetchLike;
  private readonly token: string | undefined;

  constructor(options: ApiClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    const defaultFetch = (globalThis as { fetch?: FetchLike }).fetch;
    if (!options.fetch && !defaultFetch) throw new Error("a fetch implementation is required");
    this.request = options.fetch ?? defaultFetch as FetchLike;
    this.token = options.token;
  }

  async getMetricSummary(): Promise<MetricSummary> { return this.get("/api/v1/metrics/summary"); }
  async getDataQualitySummary(): Promise<DataQualitySummary> { return this.get("/api/v1/data-quality/summary"); }
  async getSystemStatus(): Promise<SystemHealthSummary> { return this.get("/v1/system/status"); }
  async getComparison(): Promise<ComparisonSummary> { return this.get("/v1/comparison"); }
  async getTraceDiagnostics(traceId: string): Promise<TraceDiagnostic[]> { return this.get(`/v1/traces/${encodeURIComponent(traceId)}/diagnostics`); }
  async getTraceRecommendations(traceId: string): Promise<TraceRecommendation[]> { return this.get(`/v1/traces/${encodeURIComponent(traceId)}/recommendations`); }
  async getTraceDetail(traceId: string): Promise<TraceDetail> { return this.get(`/v1/traces/${encodeURIComponent(traceId)}`); }
  async getTraces(limit = 50): Promise<TraceListItem[]> { return this.get(`/v1/traces?limit=${Math.min(100, Math.max(1, limit))}`); }
  async getOverview(): Promise<OverviewSummary> { return this.get("/v1/overview"); }
  async createProject(token: string, name: string, initialTokenName: string): Promise<CreatedProjectResponse> { return this.post("/v1/control/projects", { name, initial_token_name: initialTokenName }, token); }
  async createEnvironment(projectId: string, token: string, name: string, cluster: string): Promise<EnvironmentResponse> { return this.post(`/v1/control/projects/${encodeURIComponent(projectId)}/environments`, { name, cluster }, token); }
  async createRoute(projectId: string, environmentId: string, token: string, name: string, endpoint: string): Promise<RouteResponse> { return this.post(`/v1/control/projects/${encodeURIComponent(projectId)}/environments/${encodeURIComponent(environmentId)}/routes`, { name, endpoint }, token); }
  async createToken(projectId: string, token: string, name: string, scopes: string[]): Promise<CreatedTokenResponse> { return this.post(`/v1/control/projects/${encodeURIComponent(projectId)}/tokens`, { name, scopes }, token); }
  async getX402PaymentAudit(projectId: string, token: string, limit = 50): Promise<X402PaymentAuditRecord[]> { return this.getWithToken(`/v1/control/projects/${encodeURIComponent(projectId)}/x402/audit?limit=${Math.min(100, Math.max(1, limit))}`, token); }

  private async get<T>(path: string): Promise<T> {
    const headers: Record<string, string> = { accept: "application/json" };
    if (this.token) headers["authorization"] = `Bearer ${this.token}`;
    const response = await this.request(`${this.baseUrl}${path}`, { method: "GET", headers });
    if (!response.ok) throw new Error(`Landfall API request failed: ${response.status}`);
    return (await response.json()) as T;
  }

  private async post<T>(path: string, body: unknown, token: string): Promise<T> {
    const response = await this.request(`${this.baseUrl}${path}`, { method: "POST", headers: { accept: "application/json", "content-type": "application/json", authorization: `Bearer ${token}` }, body: JSON.stringify(body) });
    if (!response.ok) throw new Error(`Landfall API request failed: ${response.status}`);
    return (await response.json()) as T;
  }

  private async getWithToken<T>(path: string, token: string): Promise<T> {
    const response = await this.request(`${this.baseUrl}${path}`, { method: "GET", headers: { accept: "application/json", authorization: `Bearer ${token}` } });
    if (!response.ok) throw new Error(`Landfall API request failed: ${response.status}`);
    return (await response.json()) as T;
  }
}
