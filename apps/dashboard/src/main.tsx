import * as React from "react";
import { Component, StrictMode, type ErrorInfo, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";

type Route = "overview" | "traces" | "comparison";

function routeFromLocation(): Route {
  const value = window.location.hash.slice(1);
  return value === "traces" || value === "comparison" ? value : "overview";
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

function Dashboard() {
  const [route, setRoute] = React.useState<Route>(routeFromLocation);
  React.useEffect(() => { const onHash = () => setRoute(routeFromLocation()); window.addEventListener("hashchange", onHash); return () => window.removeEventListener("hashchange", onHash); }, []);
  const labels: Record<Route, string> = { overview: "Overview", traces: "Traces", comparison: "Comparison" };
  return <div className="app-shell">
    <header className="topbar"><a className="brand" href="#overview">Landfall</a><span className="eyebrow">transaction observability</span></header>
    <div className="layout"><nav aria-label="Primary navigation"><p className="nav-caption">Workspace</p>{(Object.keys(labels) as Route[]).map((key) => <a className={route === key ? "nav-link active" : "nav-link"} aria-current={route === key ? "page" : undefined} href={`#${key}`} key={key}>{labels[key]}</a>)}</nav>
      <main className="content"><p className="eyebrow">{labels[route]}</p><h1>{route === "overview" ? "Lifecycle evidence at a glance" : labels[route]}</h1><p className="lede">Understand what landed, what succeeded, and what remains unknown.</p>{route === "overview" && <><OverviewMetrics /><OnboardingHealth /></>}{route === "traces" && <TraceList />}{route === "comparison" && <section className="state-card" aria-live="polite"><span className="status-dot" aria-hidden="true" />Fixture mode is ready</section>}</main>
    </div>
  </div>;
}

function OnboardingHealth() {
  const checks = [
    ["Project", "Demo project connected", "ready"],
    ["Environment", "Production · strict privacy", "ready"],
    ["Collector", "Schema v1 · receiving events", "ready"],
    ["Observer", "2 routes · one needs attention", "warning"],
  ] as const;
  return <>
    <section className="health-card" aria-labelledby="health-title"><div className="section-heading"><div><p className="eyebrow">Onboarding</p><h2 id="health-title">System health</h2></div><span className="badge warning">1 warning</span></div><div className="check-list">{checks.map(([label, detail, state]) => <div className="check-row" key={label}><span className={`status-dot ${state}`} aria-label={state === "ready" ? "Ready" : "Warning"} /> <div><strong>{label}</strong><span>{detail}</span></div></div>)}</div></section>
    <section className="state-card" aria-labelledby="next-title"><p className="eyebrow">First trace checklist</p><h2 id="next-title">Send one trace to validate the setup</h2><p className="muted">Use the CLI command below, then return here to see lifecycle evidence.</p><code className="command">landfall trace --environment production</code><div className="warning-box" role="status"><strong>Data-quality warning</strong><span>Observer coverage is incomplete; conclusions may remain unknown.</span></div></section>
  </>;
}

function OverviewMetrics() {
  const metrics = [["Landing rate", "92.4%", "+4.1% vs baseline"], ["Execution success", "89.7%", "−1.8% vs baseline"], ["Unknown / missing", "6.2%", "−0.9% vs baseline"]];
  return <section aria-labelledby="metrics-title"><div className="section-heading"><div><p className="eyebrow">Last 24 hours</p><h2 id="metrics-title">Operational overview</h2></div><button className="secondary-button" type="button">Metric definitions</button></div><div className="metric-grid">{metrics.map(([name, value, change]) => <div className="metric-card" key={name}><span>{name}</span><strong>{value}</strong><small>{change}</small></div>)}</div><div className="trend-grid"><div className="state-card"><h2>Latency trend</h2><div className="bars" aria-label="Latency trend from 420 to 310 milliseconds">{[42, 55, 49, 64, 58, 46, 31].map((height, index) => <span style={{ height: `${height}%` }} key={index} />)}</div><small className="muted">p95 · 420 ms → 310 ms</small></div><div className="state-card"><h2>Coverage by route</h2><div className="breakdown"><span>primary-rpc <b>94%</b></span><span>backup-rpc <b>81%</b></span><span>local-sim <b>76%</b></span></div></div></div></section>;
}

function TraceList() {
  const [query, setQuery] = React.useState(() => new URLSearchParams(window.location.hash.split("?")[1] ?? "").get("q") ?? "");
  const traces = [{ id: "tr_01HZX9", flow: "swap", status: "Landed", certainty: "confirmed", route: "primary-rpc", time: "2 min ago" }, { id: "tr_01HZX8", flow: "transfer", status: "Unknown", certainty: "incomplete", route: "backup-rpc", time: "8 min ago" }, { id: "tr_01HZX7", flow: "mint", status: "Failed", certainty: "confirmed", route: "primary-rpc", time: "14 min ago" }];
  const visible = traces.filter((trace) => !query || `${trace.id} ${trace.flow} ${trace.route}`.toLowerCase().includes(query.toLowerCase()));
  function submit(event: React.FormEvent<HTMLFormElement>) { event.preventDefault(); window.location.hash = `traces${query ? `?q=${encodeURIComponent(query)}` : ""}`; }
  return <section aria-labelledby="trace-list-title"><form className="search-bar" onSubmit={submit}><label htmlFor="trace-query">Search traces, signatures, or business actions</label><div><input id="trace-query" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="e.g. tr_01HZX9" /><button type="submit">Search</button></div></form><div className="list-heading"><h2 id="trace-list-title">Recent traces</h2><span className="muted">{visible.length} of {traces.length} fixture traces</span></div><div className="trace-table" role="table" aria-label="Trace list">{visible.map((trace) => <div className="trace-row" role="row" key={trace.id}><div role="cell"><strong>{trace.id}</strong><span>{trace.flow} · {trace.route}</span></div><span className={`status-label ${trace.certainty}`} role="cell">{trace.status}</span><time role="cell">{trace.time}</time></div>)}{visible.length === 0 && <div className="empty-state" role="status">No traces match this filter.</div>}</div><button className="secondary-button pagination" type="button">Load next page</button></section>;
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
