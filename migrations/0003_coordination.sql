CREATE TABLE tasks (
    task_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    objective TEXT NOT NULL,
    acceptance_criteria_json TEXT NOT NULL,
    write_scope_json TEXT NOT NULL,
    depends_on_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('queued', 'leased', 'completed', 'cancelled')),
    lease_owner TEXT,
    lease_expires_at_unix_ms INTEGER,
    outcome_summary TEXT,
    completion_evidence_json TEXT NOT NULL DEFAULT '[]',
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX tasks_project_status
    ON tasks(project_id, status, updated_at_unix_ms DESC);

CREATE INDEX tasks_active_lease
    ON tasks(project_id, lease_expires_at_unix_ms)
    WHERE status = 'leased';

PRAGMA user_version = 3;
