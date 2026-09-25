BEGIN IMMEDIATE;

CREATE TABLE role_appointments (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    appointment_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    task_id TEXT REFERENCES tasks(task_id),
    role_id TEXT NOT NULL CHECK(role_id IN ('executive.prime_minister', 'portfolio.steward', 'delivery.worker', 'oversight.inspector')),
    role_version INTEGER NOT NULL CHECK(role_version = 1),
    assignee_id TEXT NOT NULL,
    model_hint TEXT,
    capability_refs_json TEXT NOT NULL DEFAULT '[]',
    appointment_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    revoked_at_unix_ms INTEGER
);

CREATE INDEX role_appointments_project ON role_appointments(project_id, created_at_unix_ms DESC);
CREATE INDEX role_appointments_task ON role_appointments(task_id, role_id, created_at_unix_ms DESC);

PRAGMA user_version = 15;
COMMIT;
