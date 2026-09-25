CREATE TABLE IF NOT EXISTS projects (
    project_id TEXT PRIMARY KEY,
    workspace TEXT NOT NULL UNIQUE,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    objective TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('open', 'completed', 'handoff')),
    opened_at_unix_ms INTEGER NOT NULL,
    closed_at_unix_ms INTEGER,
    summary TEXT,
    next_action TEXT
);

CREATE INDEX IF NOT EXISTS sessions_project_status
    ON sessions(project_id, status, opened_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS records (
    record_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    evidence TEXT,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS records_session_created
    ON records(session_id, created_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    idempotency_key TEXT NOT NULL UNIQUE,
    stream_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    result_json TEXT NOT NULL,
    occurred_at_unix_ms INTEGER NOT NULL
);

PRAGMA user_version = 1;
