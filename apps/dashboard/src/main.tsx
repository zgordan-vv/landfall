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
  type TokenResponse,
  type X402PaymentAuditRecord,
} from "@landfall/api-client";
import "./styles.css";

type Route =
  | "overview"
  | "onboarding"
  | "traces"
  | "comparison"
  | "payments"
  | "reports"
  | "account"
  | "support"
  | "trace-detail";

type AccountSession = {
  user: { user_id: string; email: string; display_name: string };
  access_token: string;
  expires_at: string;
  workspace: { workspace_id: string; name: string; slug: string; role: string };
};

function routeFromLocation(): Route {
  const value = window.location.hash.slice(1);
  if (value === "demo") return "overview";
  if (value.startsWith("traces/") || value === "trace-detail") return "trace-detail";
  return value === "traces" ||
    value === "comparison" ||
    value === "onboarding" ||
    value === "payments" ||
    value === "reports" ||
    value === "account" ||
    value === "support"
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
        If you are evaluating Landfall, open <a href="#support">Support</a> for a quick map of
        projects, tokens, and demo limits.
      </p>
    </section>
  );
}

function AccountPortal({
  session,
  onSession,
}: {
  session: AccountSession | null;
  onSession: (value: AccountSession | null) => void;
}) {
  const [mode, setMode] = React.useState<"login" | "register">("register");
  const [email, setEmail] = React.useState("");
  const [displayName, setDisplayName] = React.useState("");
  const [workspaceName, setWorkspaceName] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [workspaces, setWorkspaces] = React.useState<AccountSession["workspace"][]>([]);
  const [selectedWorkspace, setSelectedWorkspace] = React.useState<
    AccountSession["workspace"] | null
  >(session?.workspace ?? null);
  const [projects, setProjects] = React.useState<{ project_id: string; name: string }[]>([]);
  const [members, setMembers] = React.useState<
    { user_id: string; email: string; display_name: string; role: string }[]
  >([]);
  const [message, setMessage] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const request = React.useCallback(
    async <T,>(path: string, init: RequestInit = {}): Promise<T> => {
      const response = await fetch(`${apiBaseUrl()}${path}`, {
        ...init,
        headers: {
          "content-type": "application/json",
          ...(session ? { authorization: `Bearer ${session.access_token}` } : {}),
          ...init.headers,
        },
      });
      if (!response.ok) {
        const body = (await response.json().catch(() => null)) as { message?: string } | null;
        throw new Error(body?.message ?? `Request failed: ${response.status}`);
      }
      return (response.status === 204 ? undefined : response.json()) as T;
    },
    [session],
  );
  const reloadWorkspace = React.useCallback(async () => {
    if (!session) return;
    const available = await request<AccountSession["workspace"][]>("/v1/workspaces");
    setWorkspaces(available);
    const current = selectedWorkspace
      ? available.find((workspace) => workspace.workspace_id === selectedWorkspace.workspace_id)
      : available[0];
    setSelectedWorkspace(current ?? null);
  }, [request, selectedWorkspace, session]);
  React.useEffect(() => {
    void reloadWorkspace().catch((reason: unknown) =>
      setMessage(reason instanceof Error ? reason.message : "Could not load workspaces"),
    );
  }, [reloadWorkspace]);
  React.useEffect(() => {
    if (!session || !selectedWorkspace) return;
    void Promise.all([
      request<{ project_id: string; name: string }[]>(
        `/v1/workspaces/${selectedWorkspace.workspace_id}/projects`,
      ),
      request<{ user_id: string; email: string; display_name: string; role: string }[]>(
        `/v1/workspaces/${selectedWorkspace.workspace_id}/members`,
      ),
    ])
      .then(([projectList, memberList]) => {
        setProjects(projectList);
        setMembers(memberList);
      })
      .catch((reason: unknown) =>
        setMessage(reason instanceof Error ? reason.message : "Could not load workspace details"),
      );
  }, [request, selectedWorkspace, session]);
  if (!session)
    return (
      <section className="state-card" aria-label="Account access">
        <p className="eyebrow">Landfall account</p>
        <h2>{mode === "register" ? "Create your workspace" : "Sign in"}</h2>
        <p className="muted">
          Accounts own workspaces. A workspace contains your projects, environments, team members,
          and access tokens.
        </p>
        <form
          className="setup-form"
          onSubmit={(event) => {
            event.preventDefault();
            setBusy(true);
            setMessage(null);
            const body =
              mode === "register"
                ? { email, display_name: displayName, password, workspace_name: workspaceName }
                : { email, password };
            void request<AccountSession>(`/v1/auth/${mode === "register" ? "register" : "login"}`, {
              method: "POST",
              body: JSON.stringify(body),
            })
              .then((value) => {
                onSession(value);
                setSelectedWorkspace(value.workspace);
              })
              .catch((reason: unknown) =>
                setMessage(reason instanceof Error ? reason.message : "Account request failed"),
              )
              .finally(() => setBusy(false));
          }}
        >
          {mode === "register" && (
            <>
              <label>
                Your name
                <input
                  required
                  value={displayName}
                  onChange={(event) => setDisplayName(event.target.value)}
                />
              </label>
              <label>
                Workspace name
                <input
                  required
                  value={workspaceName}
                  onChange={(event) => setWorkspaceName(event.target.value)}
                  placeholder="Acme"
                />
              </label>
            </>
          )}
          <label>
            Email
            <input
              required
              type="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              autoComplete="email"
            />
          </label>
          <label>
            Password
            <input
              required
              minLength={12}
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              autoComplete={mode === "register" ? "new-password" : "current-password"}
            />
          </label>
          <button disabled={busy} type="submit">
            {mode === "register" ? "Create account" : "Sign in"}
          </button>
        </form>
        <button
          className="secondary-button"
          onClick={() => setMode(mode === "register" ? "login" : "register")}
          type="button"
        >
          {mode === "register" ? "I already have an account" : "Create an account"}
        </button>
        {message && (
          <p className="setup-message" role="alert">
            {message}
          </p>
        )}
      </section>
    );
  const createWorkspace = async () => {
    const name = window.prompt("Workspace name");
    if (!name?.trim()) return;
    const created = await request<AccountSession["workspace"]>("/v1/workspaces", {
      method: "POST",
      body: JSON.stringify({ name }),
    });
    await reloadWorkspace();
    setSelectedWorkspace(created);
    setMessage(`Workspace ${created.name} created.`);
  };
  const createProject = async () => {
    if (!selectedWorkspace) return;
    const name = window.prompt("Project name");
    if (!name?.trim()) return;
    const created = await request<{ name: string; initial_token: string }>(
      `/v1/workspaces/${selectedWorkspace.workspace_id}/projects`,
      { method: "POST", body: JSON.stringify({ name, initial_token_name: "workspace-admin" }) },
    );
    setMessage(
      `Project ${created.name} created. Save its administrator token now: ${created.initial_token}`,
    );
    const projectList = await request<{ project_id: string; name: string }[]>(
      `/v1/workspaces/${selectedWorkspace.workspace_id}/projects`,
    );
    setProjects(projectList);
  };
  const invite = async () => {
    if (!selectedWorkspace) return;
    const inviteEmail = window.prompt("Teammate email");
    if (!inviteEmail?.trim()) return;
    const role = window.prompt("Role: admin, developer, or viewer", "developer") ?? "developer";
    const created = await request<{ invitation_token: string }>(
      `/v1/workspaces/${selectedWorkspace.workspace_id}/invitations`,
      { method: "POST", body: JSON.stringify({ email: inviteEmail, role }) },
    );
    setMessage(
      `Invitation created. Send this one-time invitation token securely: ${created.invitation_token}`,
    );
  };
  return (
    <section aria-label="Account and workspace">
      <section className="state-card">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Signed in</p>
            <h2>{session.user.display_name}</h2>
            <p className="muted">{session.user.email}</p>
          </div>
          <button
            className="secondary-button"
            onClick={() => {
              void request<void>("/v1/auth/logout", { method: "POST" }).finally(() =>
                onSession(null),
              );
            }}
            type="button"
          >
            Sign out
          </button>
        </div>
        <label>
          Workspace
          <select
            value={selectedWorkspace?.workspace_id ?? ""}
            onChange={(event) =>
              setSelectedWorkspace(
                workspaces.find((workspace) => workspace.workspace_id === event.target.value) ??
                  null,
              )
            }
          >
            {workspaces.map((workspace) => (
              <option key={workspace.workspace_id} value={workspace.workspace_id}>
                {workspace.name} · {workspace.role}
              </option>
            ))}
          </select>
        </label>
        <button
          className="secondary-button"
          onClick={() =>
            void createWorkspace().catch((reason: unknown) =>
              setMessage(reason instanceof Error ? reason.message : "Workspace creation failed"),
            )
          }
          type="button"
        >
          Create workspace
        </button>
      </section>
      {selectedWorkspace && (
        <>
          <section className="state-card">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Projects</p>
                <h2>{selectedWorkspace.name}</h2>
              </div>
              <button
                onClick={() =>
                  void createProject().catch((reason: unknown) =>
                    setMessage(
                      reason instanceof Error ? reason.message : "Project creation failed",
                    ),
                  )
                }
                type="button"
              >
                Create project
              </button>
            </div>
            {projects.length ? (
              <ul className="detail-list">
                {projects.map((project) => (
                  <li key={project.project_id}>
                    <strong>{project.name}</strong>
                    <span>{project.project_id}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="muted">No projects yet.</p>
            )}
          </section>
          <section className="state-card">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Team</p>
                <h2>Workspace members</h2>
              </div>
              {["owner", "admin"].includes(selectedWorkspace.role) && (
                <button
                  onClick={() =>
                    void invite().catch((reason: unknown) =>
                      setMessage(reason instanceof Error ? reason.message : "Invitation failed"),
                    )
                  }
                  type="button"
                >
                  Invite teammate
                </button>
              )}
            </div>
            <ul className="detail-list">
              {members.map((member) => (
                <li key={member.user_id}>
                  <strong>{member.display_name}</strong>
                  <span>
                    {member.email} · {member.role}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        </>
      )}
      {message && (
        <p className="setup-message" role="status">
          {message}
        </p>
      )}
    </section>
  );
}

function Dashboard() {
  const [route, setRoute] = React.useState<Route>(routeFromLocation);
  const [showAbout, setShowAbout] = React.useState(false);
  const [accountSession, setAccountSession] = React.useState<AccountSession | null>(null);
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
    account: "Account",
    support: "Support",
    "trace-detail": "Trace detail",
  };
  const isDemo = access?.mode === "demo";
  const activeRoute =
    isDemo && !["overview", "traces", "comparison", "support", "trace-detail"].includes(route)
      ? "overview"
      : route;
  const api = React.useMemo(
    () =>
      access === null
        ? null
        : new LandfallApiClient({
            baseUrl:
              access.mode === "demo" ? `${apiBaseUrl().replace(/\/$/, "")}/demo` : apiBaseUrl(),
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
    ) : activeRoute === "support" ? (
      <SupportCenter />
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
        <button
          className="secondary-button about-button"
          onClick={() => setShowAbout(true)}
          type="button"
        >
          About Landfall
        </button>
        {api && (
          <button className="secondary-button logout" onClick={() => setAccess(null)} type="button">
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
                (!isDemo ||
                  key === "overview" ||
                  key === "traces" ||
                  key === "comparison" ||
                  key === "support"),
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
                    : activeRoute === "support"
                      ? "Get unstuck quickly"
                      : labels[activeRoute]}
          </h1>
          <p className="lede">Understand what landed, what succeeded, and what remains unknown.</p>
          {showAbout && (
            <section className="about-card" aria-labelledby="about-landfall-title" role="dialog">
              <div className="section-heading">
                <div>
                  <p className="eyebrow">About</p>
                  <h2 id="about-landfall-title">Solana transaction observability</h2>
                </div>
                <button
                  aria-label="Close About Landfall"
                  className="secondary-button"
                  onClick={() => setShowAbout(false)}
                  type="button"
                >
                  Close
                </button>
              </div>
              <p>
                Landfall tracks a Solana transaction from signing through submission and on-chain
                confirmation, showing what succeeded, failed, or remains unknown.
              </p>
              <p className="muted">
                It helps Web3 teams diagnose transaction issues with lifecycle evidence,
                recommendations, environment comparison, and controlled x402 payment auditing.
              </p>
              {isDemo && (
                <p className="demo-boundary">
                  This public workspace is read-only and contains real Solana devnet activity. No
                  customer credentials, RPC endpoints, ingestion, or payment controls are exposed.
                </p>
              )}
            </section>
          )}
          {activeRoute === "account" ? (
            <AccountPortal session={accountSession} onSession={setAccountSession} />
          ) : activeRoute === "onboarding" ? (
            <Onboarding />
          ) : activeRoute === "support" ? (
            <SupportCenter />
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

function SupportCenter() {
  const currentUrl = typeof window === "undefined" ? "" : window.location.href;
  const supportItems = [
    {
      title: "What is a project?",
      detail:
        "A project is one application or product you want to observe. It owns environments, RPC routes, traces, dashboard tokens, SDK tokens, reports, and x402 audit records.",
    },
    {
      title: "Demo vs live workspace",
      detail:
        "Demo mode is public and read-only. A live workspace uses your own project tokens and private RPC route, so it can show your real traces and payment decisions.",
    },
    {
      title: "Which token do I need?",
      detail:
        "Use a dashboard token for Overview, Traces, and Comparison. Use an administrator token only for setup, reports, token management, private RPC routes, and x402 audit reads.",
    },
    {
      title: "Why is Comparison empty?",
      detail:
        "Comparison needs at least two environments with traces in the same project, for example devnet and mainnet-beta, or primary RPC and fallback RPC.",
    },
    {
      title: "How can I test x402 without money?",
      detail:
        "Use the payment audit page with a project administrator token after recording policy decisions in safe mode. It shows approvals, denials, and settlement references without requiring a real payment rail.",
    },
    {
      title: "How do pilot payments work?",
      detail:
        "Landfall does not run card checkout yet. For the first pilots, agree the audit scope and accept payment through Upwork, invoice, bank transfer, Wise, Payoneer, or another manual channel.",
    },
  ];
  return (
    <section aria-label="Support center">
      <section className="state-card">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Help</p>
            <h2>Fast answers for setup and demos</h2>
          </div>
          <a className="secondary-button" href="#onboarding">
            Open setup
          </a>
        </div>
        <div className="support-grid">
          {supportItems.map((item) => (
            <article className="support-card" key={item.title}>
              <h3>{item.title}</h3>
              <p>{item.detail}</p>
            </article>
          ))}
        </div>
      </section>
      <section className="state-card">
        <p className="eyebrow">Paid pilot</p>
        <h2>Manual payment acceptance</h2>
        <p className="muted">
          Use Landfall to deliver the transaction reliability audit; accept the commercial payment
          outside the product until a real billing provider is selected.
        </p>
        <ul className="detail-list support-list">
          <li>
            <strong>Before payment</strong>
            <span>
              Agree the transaction flow, target environment, audit window, deliverables, price, and
              support channel.
            </span>
          </li>
          <li>
            <strong>Accepted channels</strong>
            <span>
              Upwork contract, invoice, bank transfer, Wise, Payoneer, or crypto by explicit
              agreement. Landfall itself stores no card data.
            </span>
          </li>
          <li>
            <strong>After payment</strong>
            <span>
              Create the project, configure private RPC, install the SDK, run the audit, and export
              the lifecycle evidence report.
            </span>
          </li>
        </ul>
      </section>
      <section className="state-card">
        <p className="eyebrow">Request help</p>
        <h2>Diagnostic package</h2>
        <p className="muted">
          When you ask for help, share only non-secret context. Do not send bearer tokens, private
          RPC URLs, wallet seed phrases, or raw signed transaction bytes.
        </p>
        <ul className="detail-list support-list">
          <li>
            <strong>Current page</strong>
            <span>{currentUrl}</span>
          </li>
          <li>
            <strong>Project/environment</strong>
            <span>
              Project ID, environment name, and Solana cluster are useful. Tokens are not.
            </span>
          </li>
          <li>
            <strong>Trace</strong>
            <span>
              Trace ID, transaction signature, and visible diagnosis text are enough for triage.
            </span>
          </li>
          <li>
            <strong>What changed</strong>
            <span>
              Tell whether this is first setup, a new RPC provider, a new SDK release, or a
              payment-policy test.
            </span>
          </li>
        </ul>
      </section>
    </section>
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
            <p className="empty-state">
              No reports have been created for this project. Create the first report after traces
              exist, then download the JSON or HTML artifact for sharing.
            </p>
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
            <div className="empty-state">
              <p>No x402 payment decisions have been recorded yet.</p>
              <p>
                For a no-money test, run the x402 policy flow in safe mode so Landfall records
                approvals or denials without settlement.
              </p>
            </div>
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
  const [endpoint, setEndpoint] = React.useState("");
  const [routeId, setRouteId] = React.useState("");
  const [tokenLifetime, setTokenLifetime] = React.useState("90");
  const [tokens, setTokens] = React.useState<TokenResponse[] | null>(null);
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
  const tokenExpiry = () => {
    const days = Number(tokenLifetime);
    return Number.isInteger(days) && days > 0
      ? new Date(Date.now() + days * 24 * 60 * 60 * 1000).toISOString()
      : undefined;
  };
  return (
    <section className="onboarding" aria-label="Landfall setup">
      <section className="state-card onboarding-intro">
        <p className="eyebrow">First run</p>
        <h2>From empty install to observable transactions</h2>
        <p className="muted">
          Landfall needs one project, one Solana environment, one RPC route for observation, and one
          SDK token for your app. After that, your app sends lifecycle events and the observer
          checks what happened on-chain.
        </p>
        <div className="setup-steps" aria-label="Setup checklist">
          <span>Project</span>
          <span>Dashboard token</span>
          <span>Environment</span>
          <span>Private RPC</span>
          <span>SDK token</span>
        </div>
        <p className="demo-boundary">
          Just evaluating? Use <a href="#demo">Try live demo</a> on the access screen. Real setup
          below requires an operator bootstrap token.
        </p>
      </section>
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
          <p className="muted">
            A project represents one application or product. It keeps that product's traces,
            environments, tokens, reports, and x402 audit records together.
          </p>
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
            <label>
              Token lifetime
              <select
                value={tokenLifetime}
                onChange={(event) => setTokenLifetime(event.target.value)}
              >
                <option value="30">30 days</option>
                <option value="90">90 days (recommended)</option>
                <option value="365">1 year</option>
                <option value="never">No expiration</option>
              </select>
            </label>
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
                    const token = await api.createToken(
                      projectId,
                      adminToken,
                      "dashboard-reader",
                      ["traces:read", "diagnostics:read"],
                      tokenExpiry(),
                    );
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
              const route = await api.createRoute(
                projectId,
                environmentId,
                adminToken,
                routeName,
                endpoint,
              );
              setRouteId(route.route_id);
              const verification = await api.verifyRoute(
                projectId,
                environmentId,
                route.route_id,
                adminToken,
              );
              setMessage(
                verification.reachable
                  ? `Encrypted RPC route connected and verified in ${verification.latency_ms} ms.`
                  : "Encrypted RPC route was saved, but its connection check failed. Verify its URL and provider access.",
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
              placeholder="Your private HTTPS Solana RPC URL"
            />
          </label>
          <button disabled={busy} type="submit">
            Connect route
          </button>
          <p className="muted">
            The endpoint is encrypted with AES-256-GCM before storage, never shown again, and
            decrypted only by the observer worker.
          </p>
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
              <pre className="command">{`# Store this value in your application's secret manager\nexport LANDFALL_TOKEN=${sdkToken.token}\nnpm install @landfall/sdk\n\nimport { LandfallSdk } from "@landfall/sdk";\nconst landfall = new LandfallSdk({\n  collectorUrl: "${window.location.origin}",\n  ingestToken: process.env.LANDFALL_TOKEN,\n});`}</pre>
            </>
          ) : (
            <button
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  const token = await api.createToken(
                    projectId,
                    adminToken,
                    "sdk-production",
                    ["ingest:write"],
                    tokenExpiry(),
                  );
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
      {projectId && (
        <section className="state-card setup-form">
          <div>
            <p className="eyebrow">Token lifecycle</p>
            <h2>Access tokens</h2>
          </div>
          <p className="muted">
            New dashboard and SDK tokens use the selected lifetime. Revoke a token immediately if it
            is exposed.
          </p>
          <button
            disabled={busy}
            onClick={() =>
              void run(async () => setTokens(await api.listTokens(projectId, adminToken)))
            }
            type="button"
          >
            Refresh token inventory
          </button>
          {tokens?.length ? (
            <ul className="detail-list">
              {tokens.map((token) => (
                <li key={token.token_id}>
                  <strong>
                    {token.name} · {token.token_prefix}
                  </strong>
                  <span>
                    {token.scopes.join(", ")} · last used:{" "}
                    {token.last_used_at ? new Date(token.last_used_at).toLocaleString() : "never"} ·{" "}
                    {token.revoked_at ? "revoked" : "active"}
                  </span>
                  {!token.revoked_at && (
                    <button
                      className="secondary-button"
                      onClick={() =>
                        void run(async () => {
                          await api.revokeToken(projectId, token.token_id, adminToken);
                          setTokens(await api.listTokens(projectId, adminToken));
                          setMessage(`Token ${token.name} revoked.`);
                        })
                      }
                      type="button"
                    >
                      Revoke
                    </button>
                  )}
                </li>
              ))}
            </ul>
          ) : tokens ? (
            <p className="muted">No tokens found.</p>
          ) : null}
          {routeId && (
            <p className="muted">
              Current route is protected by encryption key v1 and can be re-verified by reconnecting
              it if its provider configuration changes.
            </p>
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
            {traces.length === 0
              ? "No traces have been recorded yet. Install the SDK in your app or run the demo ingestion flow, then refresh this page."
              : "No traces match this filter."}
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
          {error}. Create traces in at least two environments in the same project, then return here
          to compare landing rate, execution success, and evidence coverage.
        </p>
        <p>
          <a className="secondary-button" href="#onboarding">
            Add another environment
          </a>{" "}
          <a className="secondary-button" href="#support">
            Learn how comparison works
          </a>
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
