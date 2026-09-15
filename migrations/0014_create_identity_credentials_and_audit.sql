-- Interactive account credentials, revocable browser sessions, invitations,
-- and a durable actor log. Only hashes of passwords, sessions and invitations
-- are persisted.
CREATE TABLE IF NOT EXISTS identity.password_credentials (
    user_id uuid PRIMARY KEY REFERENCES identity.users(user_id) ON DELETE CASCADE,
    password_hash text NOT NULL CHECK (length(password_hash) BETWEEN 40 AND 512),
    changed_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS identity.sessions (
    session_id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES identity.users(user_id) ON DELETE CASCADE,
    token_hash bytea NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS sessions_active_user_idx
    ON identity.sessions (user_id, expires_at) WHERE revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS control.workspace_invitations (
    invitation_id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL REFERENCES control.workspaces(workspace_id) ON DELETE CASCADE,
    email text NOT NULL CHECK (length(email) BETWEEN 3 AND 320),
    role text NOT NULL CHECK (role IN ('admin', 'developer', 'viewer')),
    token_hash bytea NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    expires_at timestamptz NOT NULL,
    accepted_at timestamptz,
    revoked_at timestamptz,
    created_by uuid NOT NULL REFERENCES identity.users(user_id),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS workspace_invitations_lookup_idx
    ON control.workspace_invitations (workspace_id, lower(email))
    WHERE accepted_at IS NULL AND revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS control.audit_log (
    audit_id uuid PRIMARY KEY,
    workspace_id uuid REFERENCES control.workspaces(workspace_id) ON DELETE SET NULL,
    project_id uuid REFERENCES control.projects(project_id) ON DELETE SET NULL,
    actor_user_id uuid REFERENCES identity.users(user_id) ON DELETE SET NULL,
    action text NOT NULL CHECK (length(action) BETWEEN 3 AND 120),
    target_type text NOT NULL CHECK (length(target_type) BETWEEN 3 AND 80),
    target_id uuid,
    details jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS audit_log_workspace_idx
    ON control.audit_log (workspace_id, created_at DESC, audit_id DESC);
