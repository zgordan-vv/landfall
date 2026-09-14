import * as React from "react";
import { Component, StrictMode, type ErrorInfo, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { LandfallApiClient, type ComparisonSummary, type CreatedTokenResponse, type OverviewSummary, type TraceDetail as ApiTraceDetail, type TraceDiagnostic, type TraceListItem, type TraceRecommendation, type X402PaymentAuditRecord } from "@landfall/api-client";
import "./styles.css";

type Route = "overview" | "onboarding" | "traces" | "comparison" | "payments" | "trace-detail";

function routeFromLocation(): Route {
  const value = window.location.hash.slice(1);
  if (value.startsWith("traces/") || value === "trace-detail") return "trace-detail";
  return value === "traces" || value === "comparison" || value === "onboarding" || value === "payments" ? value : "overview";
}

class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  override state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) { return { error }; }
  override componentDidCatch(error: Error, info: ErrorInfo) { console.error("dashboard render error", error, info); }
  override render() {
    if (this.state.error) return <main className="state-card" role="alert"><h1>Dashboard unavailable</h1><p>Reload the page to try again.</p></main>;
    return this.props.children;
  }
}

const DashboardApiContext = React.createContext<LandfallApiClient | null>(null);

function apiBaseUrl(): string {
  return import.meta.env["VITE_LANDFALL_API_URL"] ?? "";
}

function useDashboardApi(): LandfallApiClient {
  const api = React.useContext(DashboardApiContext);
  if (api === null) throw new Error("Dashboard access token is required");
  return api;
}

function DashboardAccess({ onConnect }: { onConnect: (token: string) => void }) {
  const [token, setToken] = React.useState("");
  return <section className="state-card" aria-label="Dashboard access"><p className="eyebrow">Connect</p><h2>Open your live workspace</h2><p className="muted">Enter a token with <code>traces:read</code> and <code>diagnostics:read</code>. It remains only in this browser tab and is never written to local storage.</p><form className="setup-form" onSubmit={(event) => { event.preventDefault(); onConnect(token.trim()); }}><label>Dashboard token<input required type="password" value={token} onChange={(event) => setToken(event.target.value)} autoComplete="off" /></label><button type="submit">Open workspace</button></form><p className="muted">Need a token? Use <a href="#onboarding">Get started</a> or ask your project administrator.</p></section>;
}

function Dashboard() {
  const [route, setRoute] = React.useState<Route>(routeFromLocation);
  const [accessToken, setAccessToken] = React.useState("");
  React.useEffect(() => { const onHash = () => setRoute(routeFromLocation()); window.addEventListener("hashchange", onHash); return () => window.removeEventListener("hashchange", onHash); }, []);
  const labels: Record<Route, string> = { overview: "Overview", onboarding: "Get started", traces: "Traces", comparison: "Comparison", payments: "x402 payments", "trace-detail": "Trace detail" };
  const api = React.useMemo(() => accessToken ? new LandfallApiClient({ baseUrl: apiBaseUrl(), token: accessToken }) : null, [accessToken]);
  const workspace = route === "overview" ? <OverviewMetrics /> : route === "traces" ? <TraceList /> : route === "trace-detail" ? <TraceDetail /> : route === "comparison" ? <ComparisonView /> : route === "payments" ? <X402PaymentAudit /> : <Onboarding />;
  return <div className="app-shell">
    <header className="topbar"><a className="brand" href="#overview">Landfall</a><span className="eyebrow">transaction observability</span>{api && <button className="secondary-button logout" onClick={() => setAccessToken("")} type="button">Disconnect</button>}</header>
    <div className="layout"><nav aria-label="Primary navigation"><p className="nav-caption">Workspace</p>{(Object.keys(labels) as Route[]).filter((key) => key !== "trace-detail").map((key) => <a className={route === key ? "nav-link active" : "nav-link"} aria-current={route === key ? "page" : undefined} href={`#${key}`} key={key}>{labels[key]}</a>)}</nav>
      <main className="content"><p className="eyebrow">{labels[route]}</p><h1>{route === "overview" ? "Lifecycle evidence at a glance" : route === "onboarding" ? "Connect your first transaction flow" : route === "payments" ? "Controlled x402 payment activity" : labels[route]}</h1><p className="lede">Understand what landed, what succeeded, and what remains unknown.</p>{route === "onboarding" ? <Onboarding /> : api ? <DashboardApiContext.Provider value={api}>{workspace}</DashboardApiContext.Provider> : <DashboardAccess onConnect={setAccessToken} />}</main>
    </div>
  </div>;
}

function X402PaymentAudit() {
  const api = React.useMemo(() => new LandfallApiClient({ baseUrl: import.meta.env["VITE_LANDFALL_API_URL"] ?? "" }), []);
  const [projectId, setProjectId] = React.useState("");
  const [token, setToken] = React.useState("");
  const [records, setRecords] = React.useState<X402PaymentAuditRecord[] | null>(null);
  const [message, setMessage] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const load = async (event: React.FormEvent<HTMLFormElement>) => { event.preventDefault(); setBusy(true); setMessage(null); try { setRecords(await api.getX402PaymentAudit(projectId.trim(), token)); } catch (reason) { setRecords(null); setMessage(reason instanceof Error ? reason.message : "Payment audit request failed"); } finally { setBusy(false); } };
  return <section aria-label="x402 payment audit">
    <section className="state-card"><p className="muted">Use a project administrator token to read this project’s payment ledger. It is used only for this request and is not saved by the browser.</p><form className="setup-form" onSubmit={(event) => { void load(event); }}><label>Project ID<input required value={projectId} onChange={(event) => setProjectId(event.target.value)} placeholder="UUID" /></label><label>Administrator token<input required type="password" value={token} onChange={(event) => setToken(event.target.value)} autoComplete="off" /></label><button disabled={busy} type="submit">{busy ? "Loading…" : "Load payment activity"}</button></form>{message && <p className="setup-message" role="alert">{message}</p>}</section>
    {records !== null && <section className="state-card"><div className="list-heading"><h2>Recent decisions</h2><span className="muted">{records.length} records</span></div>{records.length === 0 ? <p className="empty-state">No x402 payment decisions have been recorded yet.</p> : <div className="payment-table" role="table" aria-label="x402 payment decisions">{records.map((record) => <div className="payment-row" role="row" key={record.audit_id}><div role="cell"><strong>{record.merchant_origin}</strong><span>{record.agent_id} · {record.network} · {record.asset} · {record.amount_atomic} atomic units</span><small>{record.reason_code}{record.settlement_reference ? ` · receipt: ${record.settlement_reference}` : ""}</small></div><span className={`status-label payment-${record.decision}`} role="cell">{record.decision}</span><time role="cell">{new Date(record.decided_at).toLocaleString()}</time></div>)}</div>}</section>}
  </section>;
}

function Onboarding() {
  const api = React.useMemo(() => new LandfallApiClient({ baseUrl: import.meta.env["VITE_LANDFALL_API_URL"] ?? "" }), []);
  const [bootstrapToken, setBootstrapToken] = React.useState("");
  const [projectName, setProjectName] = React.useState("");
  const [adminToken, setAdminToken] = React.useState("");
  const [projectId, setProjectId] = React.useState("");
  const [environmentId, setEnvironmentId] = React.useState("");
  const [environmentName, setEnvironmentName] = React.useState("production");
  const [cluster, setCluster] = React.useState("mainnet-beta");
  const [routeName, setRouteName] = React.useState("mainnet-primary");
  const [endpoint, setEndpoint] = React.useState("https://api.mainnet-beta.solana.com");
  const [sdkToken, setSdkToken] = React.useState<CreatedTokenResponse | null>(null);
  const [dashboardToken, setDashboardToken] = React.useState<CreatedTokenResponse | null>(null);
  const [message, setMessage] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const run = async (action: () => Promise<void>) => { setBusy(true); setMessage(null); try { await action(); } catch (reason) { setMessage(reason instanceof Error ? reason.message : "Request failed"); } finally { setBusy(false); } };
  const copy = async (value: string) => { await navigator.clipboard.writeText(value); setMessage("Copied to clipboard. Store the token in your secret manager now."); };
  return <section className="onboarding" aria-label="Landfall setup">
    <p className="muted">This wizard writes real configuration through the control-plane API. Tokens are shown only when created and are not saved in the browser.</p>
    <form className="state-card setup-form" onSubmit={(event) => { event.preventDefault(); void run(async () => { const result = await api.createProject(bootstrapToken, projectName, "dashboard-owner"); setProjectId(result.project.project_id); setAdminToken(result.initial_token.token); setMessage(`Project ${result.project.name} created. Copy and store the administrator token before proceeding.`); }); }}>
      <div><p className="eyebrow">1 · Project</p><h2>Create a project</h2></div><label>Bootstrap token<input required type="password" value={bootstrapToken} onChange={(event) => setBootstrapToken(event.target.value)} autoComplete="off" /></label><label>Project name<input required value={projectName} onChange={(event) => setProjectName(event.target.value)} placeholder="Acme payments" /></label><button disabled={busy} type="submit">Create project</button>
    </form>
    {projectId && <><section className="state-card token-reveal"><p className="eyebrow">Save now</p><h2>Project administrator token</h2><code>{adminToken}</code><button className="secondary-button" onClick={() => void copy(adminToken)} type="button">Copy token</button><p className="muted">It controls this project. Landfall cannot display it again.</p></section>
    <section className="state-card setup-form"><div><p className="eyebrow">Workspace access</p><h2>Create a dashboard token</h2></div>{dashboardToken ? <><code>{dashboardToken.token}</code><button className="secondary-button" onClick={() => void copy(dashboardToken.token)} type="button">Copy dashboard token</button><p className="muted">Use it in Overview, Traces, and Comparison. It is shown only once.</p></> : <button disabled={busy} onClick={() => void run(async () => { const token = await api.createToken(projectId, adminToken, "dashboard-reader", ["traces:read", "diagnostics:read"]); setDashboardToken(token); setMessage("Dashboard token created. Copy it now; it will not be displayed again."); })} type="button">Create dashboard token</button>}</section>
    <form className="state-card setup-form" onSubmit={(event) => { event.preventDefault(); void run(async () => { const result = await api.createEnvironment(projectId, adminToken, environmentName, cluster); setEnvironmentId(result.environment_id); setMessage(`Environment ${result.name} created.`); }); }}><div><p className="eyebrow">2 · Environment</p><h2>Add an environment</h2></div><label>Name<input required value={environmentName} onChange={(event) => setEnvironmentName(event.target.value)} /></label><label>Cluster<input required value={cluster} onChange={(event) => setCluster(event.target.value)} /></label><button disabled={busy || Boolean(environmentId)} type="submit">Create environment</button></form></>}
    {environmentId && <form className="state-card setup-form" onSubmit={(event) => { event.preventDefault(); void run(async () => { await api.createRoute(projectId, environmentId, adminToken, routeName, endpoint); setMessage("RPC route connected. Landfall will use it for eligible observation jobs."); }); }}><div><p className="eyebrow">3 · Observation</p><h2>Connect Solana RPC</h2></div><label>Route name<input required value={routeName} onChange={(event) => setRouteName(event.target.value)} /></label><label>HTTPS endpoint<input required type="url" value={endpoint} onChange={(event) => setEndpoint(event.target.value)} /></label><button disabled={busy} type="submit">Connect route</button><p className="muted">The endpoint is never shown again in the dashboard.</p></form>}
    {environmentId && <section className="state-card setup-form"><div><p className="eyebrow">4 · Instrumentation</p><h2>Create an SDK token</h2></div>{sdkToken ? <><code>{sdkToken.token}</code><button className="secondary-button" onClick={() => void copy(sdkToken.token)} type="button">Copy SDK token</button><pre className="command">{`LANDFALL_TOKEN=${sdkToken.token}\n# Configure your SDK collector with this token.`}</pre></> : <button disabled={busy} onClick={() => void run(async () => { const token = await api.createToken(projectId, adminToken, "sdk-production", ["ingest:write"]); setSdkToken(token); setMessage("SDK token created. Copy it now; it will not be displayed again."); })} type="button">Create SDK token</button>}</section>}
    {message && <p className="setup-message" role="status">{message}</p>}
  </section>;
}

function OverviewMetrics() {
  const api = useDashboardApi();
  const [summary, setSummary] = React.useState<OverviewSummary | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  React.useEffect(() => { api.getOverview().then(setSummary).catch((reason: unknown) => setError(reason instanceof Error ? reason.message : "Overview API request failed")); }, [api]);
  if (error) return <section className="state-card" role="alert"><h2>Overview unavailable</h2><p className="muted">The dashboard could not read overview data: {error}</p></section>;
  if (summary === null) return <section className="state-card" role="status"><h2>Loading overview…</h2><p className="muted">Calculating metrics from durable traces.</p></section>;
  const rate = (numerator: number, denominator: number) => denominator === 0 ? "—" : `${((numerator / denominator) * 100).toFixed(1)}%`;
  const metrics = [["Traces", String(summary.total_traces), `last ${summary.window_hours} hours`], ["Landing rate", rate(summary.landed_traces, summary.total_traces), `${summary.landed_traces} landed`], ["Execution success", rate(summary.successful_executions, summary.total_traces), `${summary.successful_executions} successful`], ["Unknown execution", rate(summary.unknown_executions, summary.total_traces), `${summary.unknown_executions} unknown`]];
  return <section aria-labelledby="metrics-title"><div className="section-heading"><div><p className="eyebrow">Last {summary.window_hours} hours</p><h2 id="metrics-title">Operational overview</h2></div></div><div className="metric-grid">{metrics.map(([name, value, change]) => <div className="metric-card" key={name}><span>{name}</span><strong>{value}</strong><small>{change}</small></div>)}</div><p className="muted">Updated {summary.updated_at}</p></section>;
}

function TraceList() {
  const api = useDashboardApi();
  const [query, setQuery] = React.useState(() => new URLSearchParams(window.location.hash.split("?")[1] ?? "").get("q") ?? "");
  const [apiTraces, setApiTraces] = React.useState<TraceListItem[] | null>(null);
  const [apiError, setApiError] = React.useState<string | null>(null);
  React.useEffect(() => { api.getTraces().then(setApiTraces).catch((error: unknown) => setApiError(error instanceof Error ? error.message : "Trace API request failed")); }, [api]);
  const traces = apiTraces?.map((trace) => ({ id: trace.trace_id, flow: "trace", status: trace.landing_state, certainty: trace.execution_state === "unknown" ? "incomplete" : "confirmed", route: "read-model", time: trace.updated_at })) ?? [];
  const visible = traces.filter((trace) => !query || `${trace.id} ${trace.flow} ${trace.route}`.toLowerCase().includes(query.toLowerCase()));
  function submit(event: React.FormEvent<HTMLFormElement>) { event.preventDefault(); window.location.hash = `traces${query ? `?q=${encodeURIComponent(query)}` : ""}`; }
  if (apiError) return <section className="state-card" role="alert"><h2>Trace list unavailable</h2><p className="muted">The dashboard could not read the trace API: {apiError}</p></section>;
  if (apiTraces === null) return <section className="state-card" role="status"><h2>Loading traces…</h2><p className="muted">Reading durable traces from the API.</p></section>;
  return <section aria-labelledby="trace-list-title"><form className="search-bar" onSubmit={submit}><label htmlFor="trace-query">Search traces, signatures, or business actions</label><div><input id="trace-query" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="e.g. trace UUID" /><button type="submit">Search</button></div></form><div className="list-heading"><h2 id="trace-list-title">Recent traces</h2><span className="muted">{visible.length} of {traces.length} traces</span></div><div className="trace-table" role="table" aria-label="Trace list">{visible.map((trace) => <a className="trace-row" role="row" href={`#traces/${trace.id}`} key={trace.id}><div role="cell"><strong>{trace.id}</strong><span>{trace.flow} · {trace.route}</span></div><span className={`status-label ${trace.certainty}`} role="cell">{trace.status}</span><time role="cell">{trace.time}</time></a>)}{visible.length === 0 && <div className="empty-state" role="status">No traces match this filter.</div>}</div></section>;
}

function TraceDetail() {
  const api = useDashboardApi();
  const traceId = window.location.hash.split("/")[1] ?? "";
  const [apiTrace, setApiTrace] = React.useState<ApiTraceDetail | null>(null);
  const [apiError, setApiError] = React.useState<string | null>(null);
  const [diagnostics, setDiagnostics] = React.useState<TraceDiagnostic[] | null>(null);
  const [recommendations, setRecommendations] = React.useState<TraceRecommendation[] | null>(null);
  React.useEffect(() => { let active = true; if (!traceId) { setApiError("A trace ID is required"); return () => { active = false; }; } Promise.all([api.getTraceDetail(traceId), api.getTraceDiagnostics(traceId), api.getTraceRecommendations(traceId)]).then(([value, findings, advice]) => { if (active) { setApiTrace(value); setDiagnostics(findings); setRecommendations(advice); } }).catch((error: unknown) => { if (active) setApiError(error instanceof Error ? error.message : "Trace API request failed"); }); return () => { active = false; }; }, [api, traceId]);
  if (apiError) return <section className="state-card" role="alert"><a className="back-link" href="#traces">← Back to traces</a><h2>Trace unavailable</h2><p className="muted">The dashboard could not read this trace from the API: {apiError}</p></section>;
  if (apiTrace === null) return <section className="state-card" role="status"><a className="back-link" href="#traces">← Back to traces</a><h2>Loading trace…</h2><p className="muted">Reading durable trace evidence from the API.</p></section>;
  const lifecycle = apiTrace.lifecycle_state;
  const landing = apiTrace.landing_state;
  return <section aria-labelledby="detail-title"><a className="back-link" href="#traces">← Back to traces</a><div className="detail-header"><div><p className="eyebrow">Trace</p><h2 id="detail-title">{apiTrace.trace_id}</h2></div><span className="status-label confirmed">{landing} · API</span></div><div className="detail-grid"><section className="state-card"><h2>Lifecycle summary</h2><div className="lifecycle"><span>Created</span><span>Signed</span><span>Submitted</span><span className="current">{lifecycle}</span></div><p className="muted">Execution: {apiTrace.execution_state} · Observation: {apiTrace.observation_state}</p></section><section className="state-card"><h2>Attempts & observations</h2><ul className="detail-list"><li>Read model source · PostgreSQL API</li><li>Landing · {landing}</li><li>Application · {apiTrace.application_state}</li></ul></section><section className="state-card"><h2>Diagnoses & recommendations</h2>{diagnostics?.length ? <ul className="detail-list">{diagnostics.map((finding) => <li key={finding.diagnostic_id}>{finding.claim_key} <span className={`status-label ${finding.certainty === "unknown" ? "incomplete" : "confirmed"}`}>{finding.certainty}</span></li>)}</ul> : <p className="muted">No findings recorded for this trace.</p>}{recommendations?.length ? <ul className="detail-list">{recommendations.map((advice) => <li key={advice.recommendation_id}>{advice.recommendation_key} <small>{advice.rule_set_version}</small></li>)}</ul> : <p className="muted">No recommendations recorded for this trace.</p>}</section><section className="state-card"><h2>Evidence checklist</h2><ul className="detail-list checklist"><li>✓ Trace created</li><li>✓ Read from durable API</li><li>! Simulation evidence unavailable</li></ul></section></div></section>;
}

function ComparisonView() {
  const api = useDashboardApi();
  const [summary, setSummary] = React.useState<ComparisonSummary | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  React.useEffect(() => { api.getComparison().then(setSummary).catch((reason: unknown) => setError(reason instanceof Error ? reason.message : "Comparison API request failed")); }, [api]);
  if (error) return <section className="state-card" role="alert"><h2>Comparison unavailable</h2><p className="muted">{error}. Create traces in at least two environments to compare them.</p></section>;
  if (summary === null) return <section className="state-card" role="status"><h2>Loading comparison…</h2><p className="muted">Reading cohort totals from durable traces.</p></section>;
  const baselineRate = summary.baseline_traces ? summary.baseline_landed / summary.baseline_traces : 0;
  const candidateRate = summary.candidate_traces ? summary.candidate_landed / summary.candidate_traces : 0;
  const change = (candidateRate - baselineRate) * 100;
  return <section aria-labelledby="comparison-title"><div className="cohort-builder"><p className="eyebrow">Environment cohorts</p><h2 id="comparison-title">Compare observed traces</h2><p className="muted">Baseline {summary.baseline_environment_id} · candidate {summary.candidate_environment_id}</p></div><div className="comparison-grid"><div className="state-card"><p className="eyebrow">Landing-rate change</p><div className="comparison-number">{change >= 0 ? "+" : ""}{change.toFixed(1)} pp</div><p className="muted">{(baselineRate * 100).toFixed(1)}% → {(candidateRate * 100).toFixed(1)}%</p></div><div className="state-card"><p className="eyebrow">Sample sizes</p><div className="comparison-number">{summary.baseline_traces} → {summary.candidate_traces}</div><p className="muted">Durable traces per environment</p></div></div></section>;
}

const rootElement = document.querySelector<HTMLDivElement>("#root");

if (rootElement === null) {
  throw new Error("Dashboard root element was not found");
}

createRoot(rootElement).render(
  <StrictMode>
    <ErrorBoundary><Dashboard /></ErrorBoundary>
  </StrictMode>,
);
