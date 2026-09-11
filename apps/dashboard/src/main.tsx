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
      <main className="content"><p className="eyebrow">{labels[route]}</p><h1>{route === "overview" ? "Lifecycle evidence at a glance" : labels[route]}</h1><p className="lede">Understand what landed, what succeeded, and what remains unknown.</p>{route === "overview" && <OnboardingHealth />}{route !== "overview" && <section className="state-card" aria-live="polite"><span className="status-dot" aria-hidden="true" />Fixture mode is ready</section>}</main>
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

const rootElement = document.querySelector<HTMLDivElement>("#root");

if (rootElement === null) {
  throw new Error("Dashboard root element was not found");
}

createRoot(rootElement).render(
  <StrictMode>
    <ErrorBoundary><Dashboard /></ErrorBoundary>
  </StrictMode>,
);
