-- Control-plane entities. Store token hashes, never plaintext API tokens.
CREATE TABLE IF NOT EXISTS control.projects (
    project_id uuid PRIMARY KEY,
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 200),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS control.environments (
    environment_id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES control.projects(project_id),
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    cluster text NOT NULL CHECK (length(cluster) BETWEEN 1 AND 80),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, name)
);

CREATE TABLE IF NOT EXISTS control.routes (
    route_id uuid PRIMARY KEY,
    environment_id uuid NOT NULL REFERENCES control.environments(environment_id),
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    endpoint text NOT NULL CHECK (length(endpoint) BETWEEN 1 AND 2048),
    enabled boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (environment_id, name)
);

CREATE TABLE IF NOT EXISTS control.api_tokens (
    token_id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES control.projects(project_id),
    token_prefix text NOT NULL CHECK (length(token_prefix) BETWEEN 4 AND 32),
    token_hash bytea NOT NULL CHECK (octet_length(token_hash) BETWEEN 16 AND 128),
    scopes text[] NOT NULL DEFAULT '{}',
    expires_at timestamptz,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, token_prefix)
);

CREATE INDEX IF NOT EXISTS environments_project_idx ON control.environments (project_id);
CREATE INDEX IF NOT EXISTS routes_environment_idx ON control.routes (environment_id);
CREATE INDEX IF NOT EXISTS api_tokens_project_idx ON control.api_tokens (project_id);
