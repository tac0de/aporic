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
    last_activity_at_unix_ms INTEGER NOT NULL,
    abandoned INTEGER NOT NULL DEFAULT 0 CHECK(abandoned IN (0, 1)),
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
    supersedes_record_id TEXT REFERENCES records(record_id),
    verifies_effect_id TEXT REFERENCES records(record_id),
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS records_session_created
    ON records(session_id, created_at_unix_ms DESC);

CREATE UNIQUE INDEX IF NOT EXISTS records_supersedes_once
    ON records(supersedes_record_id)
    WHERE supersedes_record_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS records_verifies_effect_once
    ON records(verifies_effect_id)
    WHERE verifies_effect_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS tasks (
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
    completion_proofs_json TEXT NOT NULL DEFAULT '[]',
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS tasks_project_status
    ON tasks(project_id, status, updated_at_unix_ms DESC);

CREATE INDEX IF NOT EXISTS tasks_active_lease
    ON tasks(project_id, lease_expires_at_unix_ms)
    WHERE status = 'leased';

CREATE TABLE IF NOT EXISTS evidence_artifacts (
    evidence_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    kind TEXT NOT NULL CHECK(kind IN ('workspace_file', 'command_result', 'external_source', 'user_statement', 'model_assessment')),
    grade TEXT NOT NULL CHECK(grade IN ('direct', 'reported', 'model_only')),
    locator TEXT NOT NULL,
    summary TEXT NOT NULL,
    content_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS evidence_session_created
    ON evidence_artifacts(session_id, created_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS claims (
    claim_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    status TEXT NOT NULL CHECK(status IN ('observed', 'verified', 'inferred', 'assumed', 'intended', 'unknown')),
    statement TEXT NOT NULL,
    material INTEGER NOT NULL CHECK(material IN (0, 1)),
    supersedes_claim_id TEXT REFERENCES claims(claim_id),
    created_at_unix_ms INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS claims_supersedes_once
    ON claims(supersedes_claim_id) WHERE supersedes_claim_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS claims_session_created
    ON claims(session_id, created_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS claim_evidence (
    claim_id TEXT NOT NULL REFERENCES claims(claim_id),
    evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    PRIMARY KEY (claim_id, evidence_id)
);

CREATE TABLE IF NOT EXISTS verification_specs (
    spec_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    program TEXT NOT NULL,
    args_json TEXT NOT NULL,
    workspace_relative_cwd TEXT NOT NULL,
    expected_exit_code INTEGER NOT NULL,
    timeout_seconds INTEGER NOT NULL,
    artifact_paths_json TEXT NOT NULL,
    canonical_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS verification_specs_session_created
    ON verification_specs(session_id, created_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS execution_runs (
    run_id TEXT PRIMARY KEY,
    spec_id TEXT NOT NULL REFERENCES verification_specs(spec_id),
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    status TEXT NOT NULL CHECK(status IN ('running', 'succeeded', 'failed', 'timed_out', 'interrupted')),
    attempt INTEGER NOT NULL,
    retry_of_run_id TEXT REFERENCES execution_runs(run_id),
    started_at_unix_ms INTEGER NOT NULL,
    finished_at_unix_ms INTEGER
);
CREATE UNIQUE INDEX IF NOT EXISTS execution_one_running_per_spec
    ON execution_runs(spec_id) WHERE status = 'running';
CREATE INDEX IF NOT EXISTS execution_runs_session_started
    ON execution_runs(session_id, started_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS execution_receipts (
    receipt_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL UNIQUE REFERENCES execution_runs(run_id),
    command_spec_sha256 TEXT NOT NULL,
    resolved_executable TEXT,
    exit_code INTEGER,
    termination TEXT NOT NULL,
    stdout_sha256 TEXT NOT NULL,
    stdout_bytes INTEGER NOT NULL,
    stderr_sha256 TEXT NOT NULL,
    stderr_bytes INTEGER NOT NULL,
    git_head_before TEXT,
    git_head_after TEXT,
    worktree_state_before_sha256 TEXT,
    worktree_state_after_sha256 TEXT,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS receipt_artifacts (
    artifact_id TEXT PRIMARY KEY,
    receipt_id TEXT NOT NULL REFERENCES execution_receipts(receipt_id),
    workspace_relative_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    byte_length INTEGER NOT NULL,
    UNIQUE(receipt_id, workspace_relative_path)
);

CREATE TABLE IF NOT EXISTS claim_receipts (
    claim_id TEXT NOT NULL REFERENCES claims(claim_id),
    receipt_id TEXT NOT NULL REFERENCES execution_receipts(receipt_id),
    PRIMARY KEY (claim_id, receipt_id)
);

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

PRAGMA user_version = 5;
