-- SaaS tenancy foundation. A user may join many workspaces; a workspace owns
-- projects, and projects continue to own their environments and API tokens.
CREATE SCHEMA IF NOT EXISTS identity;

CREATE TABLE IF NOT EXISTS identity.users (
    user_id uuid PRIMARY KEY,
    email text NOT NULL CHECK (length(email) BETWEEN 3 AND 320),
    display_name text NOT NULL CHECK (length(display_name) BETWEEN 1 AND 120),
    email_verified_at timestamptz,
    disabled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS users_email_case_insensitive_idx
    ON identity.users (lower(email));

CREATE TABLE IF NOT EXISTS control.workspaces (
    workspace_id uuid PRIMARY KEY,
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    slug text NOT NULL CHECK (slug ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$' AND length(slug) BETWEEN 3 AND 80),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (slug)
);

CREATE TABLE IF NOT EXISTS control.workspace_members (
    workspace_id uuid NOT NULL REFERENCES control.workspaces(workspace_id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES identity.users(user_id) ON DELETE CASCADE,
    role text NOT NULL CHECK (role IN ('owner', 'admin', 'developer', 'viewer')),
    joined_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, user_id)
);

CREATE INDEX IF NOT EXISTS workspace_members_user_idx
    ON control.workspace_members (user_id, workspace_id);

-- Existing self-hosted installations retain their projects during the staged
-- migration. New SaaS provisioning will require this column in the next step.
ALTER TABLE control.projects
    ADD COLUMN IF NOT EXISTS workspace_id uuid REFERENCES control.workspaces(workspace_id);

CREATE INDEX IF NOT EXISTS projects_workspace_idx
    ON control.projects (workspace_id, created_at, project_id);
