/** Small fetch-based client for the documented Landfall read APIs. */
export interface FetchResponse { readonly ok: boolean; readonly status: number; json(): Promise<unknown>; }
export interface FetchInit { method?: string; headers?: Record<string, string>; }
export type FetchLike = (input: string, init?: FetchInit) => Promise<FetchResponse>;

export interface ApiClientOptions { baseUrl: string; fetch?: FetchLike; token?: string; }
export interface MetricSummary { numerator: number; denominator: number; excluded_in_flight: number; excluded_unknown: number; definition_version: string; }
export interface DataQualitySummary { assessments: number; grades: Array<{ grade: string; count: number }>; gaps: Array<{ gap: string; count: number }>; average_score: number | null; definition_version: string; }
export interface CohortComparison { baseline_rate: number | null; candidate_rate: number | null; absolute_change: number | null; relative_change: number | null; baseline_sample_size: number; candidate_sample_size: number; baseline_missing_data_rate: number; candidate_missing_data_rate: number; small_sample_warning: boolean; metric_definition: string; }
export interface DetailedSystemStatus { status: "ok" | "degraded"; database_ready: boolean; queue_depth: number; projection_lag_seconds: number; observer_routes: number; unhealthy_observer_routes: number; schema_version: string; rule_set_version: string; retention_days: number; }

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
  async getSystemStatus(): Promise<DetailedSystemStatus> { return this.get("/api/v1/system/status"); }

  private async get<T>(path: string): Promise<T> {
    const headers: Record<string, string> = { accept: "application/json" };
    if (this.token) headers["authorization"] = `Bearer ${this.token}`;
    const response = await this.request(`${this.baseUrl}${path}`, { method: "GET", headers });
    if (!response.ok) throw new Error(`Landfall API request failed: ${response.status}`);
    return (await response.json()) as T;
  }
}
