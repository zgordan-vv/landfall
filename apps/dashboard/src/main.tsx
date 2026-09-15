import * as React from "react";
import { Component, StrictMode, type ErrorInfo, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import {
  LandfallApiClient,
  type ComparisonSummary,
  type CreatedTokenResponse,
  type OverviewSummary,
  type ReportResponse,
  type TraceDetail as ApiTraceDetail,
  type TraceDiagnostic,
  type TraceListItem,
  type TraceRecommendation,
  type X402PaymentAuditRecord,
} from "@landfall/api-client";
import "./styles.css";

type Route =
  "overview" | "onboarding" | "traces" | "comparison" | "payments" | "reports" | "trace-detail";

function routeFromLocation(): Route {
  const value = window.location.hash.slice(1);
  if (value === "demo") return "overview";
  if (value.startsWith("traces/") || value === "trace-detail") return "trace-detail";
  return value === "traces" ||
    value === "comparison" ||
    value === "onboarding" ||
    value === "payments" ||
    value === "reports"
    ? value
    : "overview";
}

class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  override state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  override componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("dashboard render error", error, info);
  }
  override render() {
    if (this.state.error)
      return (
        <main className="state-card" role="alert">
          <h1>Dashboard unavailable</h1>
          <p>Reload the page to try again.</p>
        </main>
      );
    return this.props.children;
  }
}

const DashboardApiContext = React.createContext<LandfallApiClient | null>(null);
type Access = { mode: "private"; token: string } | { mode: "demo" };

function apiBaseUrl(): string {
  return import.meta.env["VITE_LANDFALL_API_URL"] ?? "";
}

function useDashboardApi(): LandfallApiClient {
  const api = React.useContext(DashboardApiContext);
  if (api === null) throw new Error("Dashboard access token is required");
  return api;
}

function recommendationCopy(recommendationKey: string, trace: ApiTraceDetail) {
  if (
    recommendationKey === "improve_evidence_coverage" &&
    trace.lifecycle_state === "signed" &&
    trace.landing_state === "not_observed"
  ) {
    return {
      title: "Confirm whether the signed transaction was sent",
      detail:
        "Landfall received signing evidence but no submission or on-chain observation. If you intended to send it, submit it once. If its blockhash has expired, create and sign a fresh transaction instead.",
    };
  }
  if (recommendationKey === "improve_evidence_coverage") {
    return {
      title: "Capture the missing lifecycle evidence",
      detail:
        "Add the missing SDK or observer event so Landfall can determine the transaction outcome rather than leaving it unknown.",
    };
  }
  return { title: recommendationKey.replaceAll("_", " "), detail: "Review this recommendation." };
}

function diagnosticCopy(claimKey: string, count: number) {
  if (claimKey === "missing_evidence") {
    return {
      title: "Missing lifecycle evidence",
      detail:
        count > 1
          ? "Two independent checks could not establish the transaction outcome. Review the evidence checklist and the recommendation below."
          : "Landfall cannot establish the transaction outcome from the evidence received so far.",
    };
  }
  return {
    title: claimKey.replaceAll("_", " "),
    detail: "Review this diagnostic together with the captured lifecycle evidence.",
  };
}

function DashboardAccess({
  onConnect,
  onTryDemo,
}: {
  onConnect: (token: string) => void;
  onTryDemo: () => void;
}) {
  const [token, setToken] = React.useState("");
  return (
    <section className="state-card" aria-label="Dashboard access">
      <p className="eyebrow">Connect</p>
      <h2>Open your live workspace</h2>
      <p className="muted">
        Enter a token with <code>traces:read</code> and <code>diagnostics:read</code>. It remains
        only in this browser tab and is never written to local storage.
      </p>
      <form
        className="setup-form"
        onSubmit={(event) => {
          event.preventDefault();
          onConnect(token.trim());
        }}
      >
        <label>
          Dashboard token
          <input
            required
            type="password"
            value={token}
            onChange={(event) => setToken(event.target.value)}
            autoComplete="off"
          />
        </label>
        <button type="submit">Open workspace</button>
      </form>
      <button className="secondary-button" onClick={onTryDemo} type="button">
        Try live demo
      </button>
      <p className="muted">
        The demo is public and read-only. It shows a separate project with real devnet traces.
      </p>
      <p className="muted">
        Need a token? Use <a href="#onboarding">Get started</a> or ask your project administrator.
      </p>
    </section>
  );
}

function Dashboard() {
  const [route, setRoute] = React.useState<Route>(routeFromLocation);
  const [access, setAccess] = React.useState<Access | null>(() =>
    window.location.hash === "#demo" ? { mode: "demo" } : null,
  );
  React.useEffect(() => {
    const onHash = () => {
      if (window.location.hash === "#demo") setAccess({ mode: "demo" });
      setRoute(routeFromLocation());
    };
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);
  const labels: Record<Route, string> = {
    overview: "Overview",
    onboarding: "Get started",
    traces: "Traces",
    comparison: "Comparison",
    payments: "x402 payments",
    reports: "Reports",
    "trace-detail": "Trace detail",
  };
  const isDemo = access?.mode === "demo";
  const activeRoute =
    isDemo && !["overview", "traces", "comparison", "trace-detail"].includes(route)
      ? "overview"
      : route;
  const api = React.useMemo(
    () =>
      access === null
        ? null
        : new LandfallApiClient({
            baseUrl: access.mode === "demo" ? `${apiBaseUrl().replace(/\/$/, "")}/demo` : apiBaseUrl(),
            ...(access.mode === "private" ? { token: access.token } : {}),
          }),
    [access],
  );
  const workspace =
    activeRoute === "overview" ? (
      <OverviewMetrics />
    ) : activeRoute === "traces" ? (
      <TraceList />
    ) : activeRoute === "trace-detail" ? (
      <TraceDetail />
    ) : activeRoute === "comparison" ? (
      <ComparisonView />
    ) : activeRoute === "payments" ? (
      <X402PaymentAudit />
    ) : activeRoute === "reports" ? (
      <ReportExports />
    ) : (
      <Onboarding />
    );
  return (
    <div className="app-shell">
      <header className="topbar">
        <a className="brand" href="#overview">
          Landfall
        </a>
        <span className="eyebrow">transaction observability</span>
        {api && (
          <button
            className="secondary-button logout"
            onClick={() => setAccess(null)}
            type="button"
          >
            Disconnect
          </button>
        )}
      </header>
      <div className="layout">
        <nav aria-label="Primary navigation">
          <p className="nav-caption">Workspace</p>
          {(Object.keys(labels) as Route[])
            .filter(
              (key) =>
                key !== "trace-detail" &&
                (!isDemo || key === "overview" || key === "traces" || key === "comparison"),
            )
            .map((key) => (
              <a
                className={activeRoute === key ? "nav-link active" : "nav-link"}
                aria-current={activeRoute === key ? "page" : undefined}
                href={`#${key}`}
                key={key}
              >
                {labels[key]}
              </a>
            ))}
        </nav>
        <main className="content">
          <p className="eyebrow">{isDemo ? "Public demo" : labels[activeRoute]}</p>
          <h1>
            {activeRoute === "overview"
              ? "Lifecycle evidence at a glance"
              : activeRoute === "onboarding"
                ? "Connect your first transaction flow"
                : activeRoute === "payments"
                  ? "Controlled x402 payment activity"
                  : activeRoute === "reports"
                    ? "Export lifecycle evidence"
                    : labels[activeRoute]}
          </h1>
          <p className="lede">Understand what landed, what succeeded, and what remains unknown.</p>
          {activeRoute === "onboarding" ? (
            <Onboarding />
          ) : activeRoute === "payments" || activeRoute === "reports" ? (
            workspace
          ) : api ? (
            <DashboardApiContext.Provider value={api}>{workspace}</DashboardApiContext.Provider>
          ) : (
            <DashboardAccess
              onConnect={(token) => setAccess({ mode: "private", token })}
              onTryDemo={() => {
                window.location.hash = "demo";
                setAccess({ mode: "demo" });
              }}
            />
          )}
        </main>
      </div>
    </div>
  );
}

function ReportExports() {
  const api = React.useMemo(
    () => new LandfallApiClient({ baseUrl: import.meta.env["VITE_LANDFALL_API_URL"] ?? "" }),
    [],
  );
  const [projectId, setProjectId] = React.useState("");
  const [token, setToken] = React.useState("");
  const [title, setTitle] = React.useState("Lifecycle evidence export");
  const [privacyProfile, setPrivacyProfile] = React.useState<"internal" | "shareable">("shareable");
  const [reports, setReports] = React.useState<ReportResponse[] | null>(null);
  const [message, setMessage] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const load = async () => {
    setReports(await api.listReports(projectId.trim(), token));
  };
  const create = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setMessage(null);
    try {
      await api.createReport(projectId.trim(), token, title.trim(), privacyProfile);
      await load();
      setMessage("Report created and stored in PostgreSQL.");
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : "Report export failed");
    } finally {
      setBusy(false);
    }
  };
  const download = async (report: ReportResponse, format: "json" | "html") => {
    setBusy(true);
    setMessage(null);
    try {
      const bytes = await api.downloadReport(projectId.trim(), report.report_id, format, token);
      const blob = new Blob([new TextDecoder().decode(bytes)], {
        type: format === "json" ? "application/json" : "text/html",
      });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `landfall-report-${report.report_id}.${format}`;
      link.click();
      URL.revokeObjectURL(url);
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : "Report download failed");
    } finally {
      setBusy(false);
    }
  };
  return (
    <section aria-label="report exports">
      <section className="state-card">
        <p className="muted">
          Create an immutable report from the traces currently stored for one project. A shareable
          report hides trace IDs; an internal report retains them.
        </p>
        <form className="setup-form" onSubmit={(event) => void create(event)}>
          <label>
            Project ID
            <input
              required
              value={projectId}
              onChange={(event) => setProjectId(event.target.value)}
              placeholder="UUID"
            />
          </label>
          <label>
            Administrator token
            <input
              required
              type="password"
              value={token}
              onChange={(event) => setToken(event.target.value)}
              autoComplete="off"
            />
          </label>
          <label>
            Report title
            <input
              required
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              maxLength={240}
            />
          </label>
          <label>
            Privacy profile
            <select
              value={privacyProfile}
              onChange={(event) =>
                setPrivacyProfile(event.target.value as "internal" | "shareable")
              }
            >
              <option value="shareable">Shareable — redact trace IDs</option>
              <option value="internal">Internal — include trace IDs</option>
            </select>
          </label>
          <button disabled={busy} type="submit">
            {busy ? "Working…" : "Create report"}
          </button>
        </form>
        <button
          className="secondary-button"
          disabled={busy || !projectId.trim() || !token}
          type="button"
          onClick={() =>
            void load().catch((reason: unknown) =>
              setMessage(reason instanceof Error ? reason.message : "Report listing failed"),
            )
          }
        >
          Load existing reports
        </button>
        {message && (
          <p className="setup-message" role="status">
            {message}
          </p>
        )}
      </section>
      {reports !== null && (
        <section className="state-card">
          <div className="list-heading">
            <h2>Stored reports</h2>
            <span className="muted">{reports.length} reports</span>
          </div>
          {reports.length === 0 ? (
            <p className="empty-state">No reports have been created for this project.</p>
          ) : (
            <ul className="recommendation-list">
              {reports.map((report) => (
                <li key={report.report_id}>
                  <strong>{report.title}</strong>
                  <span>
                    {report.privacy_profile} · {report.trace_count} traces ·{" "}
                    {new Date(report.created_at).toLocaleString()}
                  </span>
                  <p>
                    <button
                      className="secondary-button"
                      disabled={busy}
                      type="button"
                      onClick={() => void download(report, "json")}
                    >
                      Download JSON
                    </button>{" "}
                    <button
                      className="secondary-button"
                      disabled={busy}
                      type="button"
                      onClick={() => void download(report, "html")}
                    >
                      Download HTML
                    </button>
                  </p>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </section>
  );
}

function X402PaymentAudit() {
  const api = React.useMemo(
    () => new LandfallApiClient({ baseUrl: import.meta.env["VITE_LANDFALL_API_URL"] ?? "" }),
    [],
  );
  const [projectId, setProjectId] = React.useState("");
  const [token, setToken] = React.useState("");
  const [records, setRecords] = React.useState<X402PaymentAuditRecord[] | null>(null);
  const [message, setMessage] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const load = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setMessage(null);
    try {
      setRecords(await api.getX402PaymentAudit(projectId.trim(), token));
    } catch (reason) {
      setRecords(null);
      setMessage(reason instanceof Error ? reason.message : "Payment audit request failed");
    } finally {
      setBusy(false);
    }
  };
  return (
    <section aria-label="x402 payment audit">
      <section className="state-card">
        <p className="muted">
          Use a project administrator token to read this project’s payment ledger. It is used only
          for this request and is not saved by the browser.
        </p>
        <form
          className="setup-form"
          onSubmit={(event) => {
            void load(event);
          }}
        >
          <label>
            Project ID
            <input
              required
              value={projectId}
              onChange={(event) => setProjectId(event.target.value)}
              placeholder="UUID"
            />
          </label>
          <label>
            Administrator token
            <input
              required
              type="password"
              value={token}
              onChange={(event) => setToken(event.target.value)}
              autoComplete="off"
            />
          </label>
          <button disabled={busy} type="submit">
            {busy ? "Loading…" : "Load payment activity"}
          </button>
        </form>
        {message && (
          <p className="setup-message" role="alert">
            {message}
          </p>
        )}
      </section>
      {records !== null && (
        <section className="state-card">
          <div className="list-heading">
            <h2>Recent decisions</h2>
            <span className="muted">{records.length} records</span>
          </div>
          {records.length === 0 ? (
            <p className="empty-state">No x402 payment decisions have been recorded yet.</p>
          ) : (
            <div className="payment-table" role="table" aria-label="x402 payment decisions">
              {records.map((record) => (
                <div className="payment-row" role="row" key={record.audit_id}>
                  <div role="cell">
                    <strong>{record.merchant_origin}</strong>
                    <span>
                      {record.agent_id} · {record.network} · {record.asset} · {record.amount_atomic}{" "}
                      atomic units
                    </span>
                    <small>
                      {record.reason_code}
                      {record.settlement_reference
                        ? ` · receipt: ${record.settlement_reference}`
                        : ""}
                    </small>
                  </div>
                  <span className={`status-label payment-${record.decision}`} role="cell">
                    {record.decision}
                  </span>
                  <time role="cell">{new Date(record.decided_at).toLocaleString()}</time>
                </div>
              ))}
            </div>
          )}
        </section>
      )}
    </section>
  );
}

function Onboarding() {
  const api = React.useMemo(
    () => new LandfallApiClient({ baseUrl: import.meta.env["VITE_LANDFALL_API_URL"] ?? "" }),
    [],
  );
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
  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setMessage(null);
    try {
      await action();
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : "Request failed");
    } finally {
      setBusy(false);
    }
  };
  const copy = async (value: string) => {
    await navigator.clipboard.writeText(value);
    setMessage("Copied to clipboard. Store the token in your secret manager now.");
  };
  return (
    <section className="onboarding" aria-label="Landfall setup">
      <p className="muted">
        This wizard writes real configuration through the control-plane API. Tokens are shown only
        when created and are not saved in the browser.
      </p>
      <form
        className="state-card setup-form"
        onSubmit={(event) => {
          event.preventDefault();
          void run(async () => {
            const result = await api.createProject(bootstrapToken, projectName, "dashboard-owner");
            setProjectId(result.project.project_id);
            setAdminToken(result.initial_token.token);
            setMessage(
              `Project ${result.project.name} created. Copy and store the administrator token before proceeding.`,
            );
          });
        }}
      >
        <div>
          <p className="eyebrow">1 · Project</p>
          <h2>Create a project</h2>
        </div>
        <label>
          Bootstrap token
          <input
            required
            type="password"
            value={bootstrapToken}
            onChange={(event) => setBootstrapToken(event.target.value)}
            autoComplete="off"
          />
        </label>
        <label>
          Project name
          <input
            required
            value={projectName}
            onChange={(event) => setProjectName(event.target.value)}
            placeholder="Acme payments"
          />
        </label>
        <button disabled={busy} type="submit">
          Create project
        </button>
      </form>
      {projectId && (
        <>
          <section className="state-card token-reveal">
            <p className="eyebrow">Save now</p>
            <h2>Project administrator token</h2>
            <code>{adminToken}</code>
            <button
              className="secondary-button"
              onClick={() => void copy(adminToken)}
              type="button"
            >
              Copy token
            </button>
            <p className="muted">It controls this project. Landfall cannot display it again.</p>
          </section>
          <section className="state-card setup-form">
            <div>
              <p className="eyebrow">Workspace access</p>
              <h2>Create a dashboard token</h2>
            </div>
            {dashboardToken ? (
              <>
                <code>{dashboardToken.token}</code>
                <button
                  className="secondary-button"
                  onClick={() => void copy(dashboardToken.token)}
                  type="button"
                >
                  Copy dashboard token
                </button>
                <p className="muted">
                  Use it in Overview, Traces, and Comparison. It is shown only once.
                </p>
              </>
            ) : (
              <button
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    const token = await api.createToken(projectId, adminToken, "dashboard-reader", [
                      "traces:read",
                      "diagnostics:read",
                    ]);
                    setDashboardToken(token);
                    setMessage(
                      "Dashboard token created. Copy it now; it will not be displayed again.",
                    );
                  })
                }
                type="button"
              >
                Create dashboard token
              </button>
            )}
          </section>
          <form
            className="state-card setup-form"
            onSubmit={(event) => {
              event.preventDefault();
              void run(async () => {
                const result = await api.createEnvironment(
                  projectId,
                  adminToken,
                  environmentName,
                  cluster,
                );
                setEnvironmentId(result.environment_id);
                setMessage(`Environment ${result.name} created.`);
              });
            }}
          >
            <div>
              <p className="eyebrow">2 · Environment</p>
              <h2>Add an environment</h2>
            </div>
            <label>
              Name
              <input
                required
                value={environmentName}
                onChange={(event) => setEnvironmentName(event.target.value)}
              />
            </label>
            <label>
              Cluster
              <input
                required
                value={cluster}
                onChange={(event) => setCluster(event.target.value)}
              />
            </label>
            <button disabled={busy || Boolean(environmentId)} type="submit">
              Create environment
            </button>
          </form>
        </>
      )}
      {environmentId && (
        <form
          className="state-card setup-form"
          onSubmit={(event) => {
            event.preventDefault();
            void run(async () => {
              await api.createRoute(projectId, environmentId, adminToken, routeName, endpoint);
              setMessage(
                "RPC route connected. Landfall will use it for eligible observation jobs.",
              );
            });
          }}
        >
          <div>
            <p className="eyebrow">3 · Observation</p>
            <h2>Connect Solana RPC</h2>
          </div>
          <label>
            Route name
            <input
              required
              value={routeName}
              onChange={(event) => setRouteName(event.target.value)}
            />
          </label>
          <label>
            HTTPS endpoint
            <input
              required
              type="url"
              value={endpoint}
              onChange={(event) => setEndpoint(event.target.value)}
            />
          </label>
          <button disabled={busy} type="submit">
            Connect route
          </button>
          <p className="muted">The endpoint is never shown again in the dashboard.</p>
        </form>
      )}
      {environmentId && (
        <section className="state-card setup-form">
          <div>
            <p className="eyebrow">4 · Instrumentation</p>
            <h2>Create an SDK token</h2>
          </div>
          {sdkToken ? (
            <>
              <code>{sdkToken.token}</code>
              <button
                className="secondary-button"
                onClick={() => void copy(sdkToken.token)}
                type="button"
              >
                Copy SDK token
              </button>
              <pre className="command">{`LANDFALL_TOKEN=${sdkToken.token}\n# Configure your SDK collector with this token.`}</pre>
            </>
          ) : (
            <button
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  const token = await api.createToken(projectId, adminToken, "sdk-production", [
                    "ingest:write",
                  ]);
                  setSdkToken(token);
                  setMessage("SDK token created. Copy it now; it will not be displayed again.");
                })
              }
              type="button"
            >
              Create SDK token
            </button>
          )}
        </section>
      )}
      {message && (
        <p className="setup-message" role="status">
          {message}
        </p>
      )}
    </section>
  );
}

function OverviewMetrics() {
  const api = useDashboardApi();
  const [summary, setSummary] = React.useState<OverviewSummary | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  React.useEffect(() => {
    api
      .getOverview()
      .then(setSummary)
      .catch((reason: unknown) =>
        setError(reason instanceof Error ? reason.message : "Overview API request failed"),
      );
  }, [api]);
  if (error)
    return (
      <section className="state-card" role="alert">
        <h2>Overview unavailable</h2>
        <p className="muted">The dashboard could not read overview data: {error}</p>
      </section>
    );
  if (summary === null)
    return (
      <section className="state-card" role="status">
        <h2>Loading overview…</h2>
        <p className="muted">Calculating metrics from durable traces.</p>
      </section>
    );
  const rate = (numerator: number, denominator: number) =>
    denominator === 0 ? "—" : `${((numerator / denominator) * 100).toFixed(1)}%`;
  const metrics = [
    ["Traces", String(summary.total_traces), `last ${summary.window_hours} hours`],
    [
      "Landing rate",
      rate(summary.landed_traces, summary.total_traces),
      `${summary.landed_traces} landed`,
    ],
    [
      "Execution success",
      rate(summary.successful_executions, summary.total_traces),
      `${summary.successful_executions} successful`,
    ],
    [
      "Unknown execution",
      rate(summary.unknown_executions, summary.total_traces),
      `${summary.unknown_executions} unknown`,
    ],
  ];
  return (
    <section aria-labelledby="metrics-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Last {summary.window_hours} hours</p>
          <h2 id="metrics-title">Operational overview</h2>
        </div>
      </div>
      <div className="metric-grid">
        {metrics.map(([name, value, change]) => (
          <div className="metric-card" key={name}>
            <span>{name}</span>
            <strong>{value}</strong>
            <small>{change}</small>
          </div>
        ))}
      </div>
      <p className="muted">Updated {summary.updated_at}</p>
    </section>
  );
}

function TraceList() {
  const api = useDashboardApi();
  const [query, setQuery] = React.useState(
    () => new URLSearchParams(window.location.hash.split("?")[1] ?? "").get("q") ?? "",
  );
  const [apiTraces, setApiTraces] = React.useState<TraceListItem[] | null>(null);
  const [apiError, setApiError] = React.useState<string | null>(null);
  React.useEffect(() => {
    api
      .getTraces()
      .then(setApiTraces)
      .catch((error: unknown) =>
        setApiError(error instanceof Error ? error.message : "Trace API request failed"),
      );
  }, [api]);
  const traces =
    apiTraces?.map((trace) => ({
      id: trace.trace_id,
      flow: "trace",
      status: trace.landing_state,
      certainty: trace.execution_state === "unknown" ? "incomplete" : "confirmed",
      route: "read-model",
      time: trace.updated_at,
    })) ?? [];
  const visible = traces.filter(
    (trace) =>
      !query ||
      `${trace.id} ${trace.flow} ${trace.route}`.toLowerCase().includes(query.toLowerCase()),
  );
  function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    window.location.hash = `traces${query ? `?q=${encodeURIComponent(query)}` : ""}`;
  }
  if (apiError)
    return (
      <section className="state-card" role="alert">
        <h2>Trace list unavailable</h2>
        <p className="muted">The dashboard could not read the trace API: {apiError}</p>
      </section>
    );
  if (apiTraces === null)
    return (
      <section className="state-card" role="status">
        <h2>Loading traces…</h2>
        <p className="muted">Reading durable traces from the API.</p>
      </section>
    );
  return (
    <section aria-labelledby="trace-list-title">
      <form className="search-bar" onSubmit={submit}>
        <label htmlFor="trace-query">Search traces, signatures, or business actions</label>
        <div>
          <input
            id="trace-query"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="e.g. trace UUID"
          />
          <button type="submit">Search</button>
        </div>
      </form>
      <div className="list-heading">
        <h2 id="trace-list-title">Recent traces</h2>
        <span className="muted">
          {visible.length} of {traces.length} traces
        </span>
      </div>
      <div className="trace-table" role="table" aria-label="Trace list">
        {visible.map((trace) => (
          <a className="trace-row" role="row" href={`#traces/${trace.id}`} key={trace.id}>
            <div role="cell">
              <strong>{trace.id}</strong>
              <span>
                {trace.flow} · {trace.route}
              </span>
            </div>
            <span className={`status-label ${trace.certainty}`} role="cell">
              {trace.status}
            </span>
            <time role="cell">{trace.time}</time>
          </a>
        ))}
        {visible.length === 0 && (
          <div className="empty-state" role="status">
            No traces match this filter.
          </div>
        )}
      </div>
    </section>
  );
}

function TraceDetail() {
  const api = useDashboardApi();
  const traceId = window.location.hash.split("/")[1] ?? "";
  const [apiTrace, setApiTrace] = React.useState<ApiTraceDetail | null>(null);
  const [apiError, setApiError] = React.useState<string | null>(null);
  const [diagnostics, setDiagnostics] = React.useState<TraceDiagnostic[] | null>(null);
  const [recommendations, setRecommendations] = React.useState<TraceRecommendation[] | null>(null);
  React.useEffect(() => {
    let active = true;
    if (!traceId) {
      setApiError("A trace ID is required");
      return () => {
        active = false;
      };
    }
    Promise.all([
      api.getTraceDetail(traceId),
      api.getTraceDiagnostics(traceId),
      api.getTraceRecommendations(traceId),
    ])
      .then(([value, findings, advice]) => {
        if (active) {
          setApiTrace(value);
          setDiagnostics(findings);
          setRecommendations(advice);
        }
      })
      .catch((error: unknown) => {
        if (active)
          setApiError(error instanceof Error ? error.message : "Trace API request failed");
      });
    return () => {
      active = false;
    };
  }, [api, traceId]);
  if (apiError)
    return (
      <section className="state-card" role="alert">
        <a className="back-link" href="#traces">
          ← Back to traces
        </a>
        <h2>Trace unavailable</h2>
        <p className="muted">The dashboard could not read this trace from the API: {apiError}</p>
      </section>
    );
  if (apiTrace === null)
    return (
      <section className="state-card" role="status">
        <a className="back-link" href="#traces">
          ← Back to traces
        </a>
        <h2>Loading trace…</h2>
        <p className="muted">Reading durable trace evidence from the API.</p>
      </section>
    );
  const lifecycle = apiTrace.lifecycle_state;
  const landing = apiTrace.landing_state;
  const visibleRecommendations = recommendations
    ? [...new Map(recommendations.map((advice) => [advice.recommendation_key, advice])).values()]
    : [];
  const visibleDiagnostics = diagnostics
    ? [
        ...new Map(
          diagnostics.map((finding) => {
            const key = `${finding.claim_key}:${finding.certainty}`;
            const existing = diagnostics.filter(
              (candidate) =>
                candidate.claim_key === finding.claim_key &&
                candidate.certainty === finding.certainty,
            );
            return [key, { finding, count: existing.length }] as const;
          }),
        ).values(),
      ]
    : [];
  return (
    <section aria-labelledby="detail-title">
      <a className="back-link" href="#traces">
        ← Back to traces
      </a>
      <div className="detail-header">
        <div>
          <p className="eyebrow">Trace</p>
          <h2 id="detail-title">{apiTrace.trace_id}</h2>
        </div>
        <span className="status-label confirmed">{landing} · API</span>
      </div>
      <div className="detail-grid">
        <section className="state-card">
          <h2>Lifecycle summary</h2>
          <div className="lifecycle">
            <span>Created</span>
            <span>Signed</span>
            <span>Submitted</span>
            <span className="current">{lifecycle}</span>
          </div>
          <p className="muted">
            Execution: {apiTrace.execution_state} · Observation: {apiTrace.observation_state}
          </p>
        </section>
        <section className="state-card">
          <h2>Attempts & observations</h2>
          <ul className="detail-list">
            <li>Read model source · PostgreSQL API</li>
            <li>Landing · {landing}</li>
            <li>Application · {apiTrace.application_state}</li>
          </ul>
        </section>
        <section className="state-card">
          <h2>Diagnoses & recommendations</h2>
          {visibleDiagnostics.length ? (
            <ul className="detail-list">
              {visibleDiagnostics.map(({ finding, count }) => {
                const copy = diagnosticCopy(finding.claim_key, count);
                return (
                  <li key={`${finding.claim_key}:${finding.certainty}`}>
                    <strong>{copy.title}</strong>
                    <span>{copy.detail}</span>
                    <span
                      className={`status-label ${finding.certainty === "unknown" ? "incomplete" : "confirmed"}`}
                    >
                      {finding.certainty}
                    </span>
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="muted">No findings recorded for this trace.</p>
          )}
          {visibleRecommendations.length ? (
            <ul className="detail-list">
              {visibleRecommendations.map((advice) => {
                const copy = recommendationCopy(advice.recommendation_key, apiTrace);
                return (
                  <li key={advice.recommendation_id}>
                    <strong>{copy.title}</strong>
                    <span>{copy.detail}</span>
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="muted">No recommendations recorded for this trace.</p>
          )}
        </section>
        <section className="state-card">
          <h2>Evidence checklist</h2>
          <ul className="detail-list checklist">
            <li>{apiTrace.evidence.trace_created ? "✓" : "!"} Trace created</li>
            <li>{apiTrace.evidence.signing_completed ? "✓" : "!"} Signing completed</li>
            <li>{apiTrace.evidence.submission_completed ? "✓" : "!"} Submission completed</li>
            <li>{apiTrace.evidence.status_observed ? "✓" : "!"} On-chain status observed</li>
            <li>{apiTrace.evidence.simulation_completed ? "✓" : "!"} Simulation completed</li>
          </ul>
        </section>
      </div>
    </section>
  );
}

function ComparisonView() {
  const api = useDashboardApi();
  const [summary, setSummary] = React.useState<ComparisonSummary | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  React.useEffect(() => {
    api
      .getComparison()
      .then(setSummary)
      .catch((reason: unknown) =>
        setError(reason instanceof Error ? reason.message : "Comparison API request failed"),
      );
  }, [api]);
  if (error)
    return (
      <section className="state-card" role="alert">
        <h2>Comparison unavailable</h2>
        <p className="muted">
          {error}. Create traces in at least two environments to compare them.
        </p>
      </section>
    );
  if (summary === null)
    return (
      <section className="state-card" role="status">
        <h2>Loading comparison…</h2>
        <p className="muted">Reading cohort totals from durable traces.</p>
      </section>
    );
  const baselineRate = summary.baseline_traces
    ? summary.baseline_landed / summary.baseline_traces
    : 0;
  const candidateRate = summary.candidate_traces
    ? summary.candidate_landed / summary.candidate_traces
    : 0;
  const change = (candidateRate - baselineRate) * 100;
  return (
    <section aria-labelledby="comparison-title">
      <div className="cohort-builder">
        <p className="eyebrow">Environment cohorts</p>
        <h2 id="comparison-title">Compare observed traces</h2>
        <p className="muted">
          Baseline {summary.baseline_environment_id} · candidate {summary.candidate_environment_id}
        </p>
      </div>
      <div className="comparison-grid">
        <div className="state-card">
          <p className="eyebrow">Landing-rate change</p>
          <div className="comparison-number">
            {change >= 0 ? "+" : ""}
            {change.toFixed(1)} pp
          </div>
          <p className="muted">
            {(baselineRate * 100).toFixed(1)}% → {(candidateRate * 100).toFixed(1)}%
          </p>
        </div>
        <div className="state-card">
          <p className="eyebrow">Sample sizes</p>
          <div className="comparison-number">
            {summary.baseline_traces} → {summary.candidate_traces}
          </div>
          <p className="muted">Durable traces per environment</p>
        </div>
      </div>
    </section>
  );
}

const rootElement = document.querySelector<HTMLDivElement>("#root");

if (rootElement === null) {
  throw new Error("Dashboard root element was not found");
}

createRoot(rootElement).render(
  <StrictMode>
    <ErrorBoundary>
      <Dashboard />
    </ErrorBoundary>
  </StrictMode>,
);
