CREATE TABLE verification_specs (
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

CREATE INDEX verification_specs_session_created
    ON verification_specs(session_id, created_at_unix_ms DESC);

CREATE TABLE execution_runs (
    run_id TEXT PRIMARY KEY,
    spec_id TEXT NOT NULL REFERENCES verification_specs(spec_id),
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    status TEXT NOT NULL CHECK(status IN (
        'running', 'succeeded', 'failed', 'timed_out', 'interrupted'
    )),
    attempt INTEGER NOT NULL,
    retry_of_run_id TEXT REFERENCES execution_runs(run_id),
    started_at_unix_ms INTEGER NOT NULL,
    finished_at_unix_ms INTEGER
);

CREATE UNIQUE INDEX execution_one_running_per_spec
    ON execution_runs(spec_id) WHERE status = 'running';
CREATE INDEX execution_runs_session_started
    ON execution_runs(session_id, started_at_unix_ms DESC);

CREATE TABLE execution_receipts (
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

CREATE TABLE receipt_artifacts (
    artifact_id TEXT PRIMARY KEY,
    receipt_id TEXT NOT NULL REFERENCES execution_receipts(receipt_id),
    workspace_relative_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    byte_length INTEGER NOT NULL,
    UNIQUE(receipt_id, workspace_relative_path)
);

CREATE TABLE claim_receipts (
    claim_id TEXT NOT NULL REFERENCES claims(claim_id),
    receipt_id TEXT NOT NULL REFERENCES execution_receipts(receipt_id),
    PRIMARY KEY (claim_id, receipt_id)
);

PRAGMA user_version = 5;
