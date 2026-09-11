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
      <main className="content"><p className="eyebrow">{labels[route]}</p><h1>{route === "overview" ? "Lifecycle evidence at a glance" : labels[route]}</h1><p className="lede">Understand what landed, what succeeded, and what remains unknown.</p><section className="state-card" aria-live="polite"><span className="status-dot" aria-hidden="true" />Fixture mode is ready<div className="metric-grid"><div><strong>—</strong><span>Landing rate</span></div><div><strong>—</strong><span>Execution success</span></div><div><strong>—</strong><span>Data quality</span></div></div></section></main>
    </div>
  </div>;
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
