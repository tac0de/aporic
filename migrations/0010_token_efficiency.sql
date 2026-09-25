BEGIN IMMEDIATE;

CREATE TABLE token_usage_receipts (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    receipt_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    session_id TEXT REFERENCES sessions(session_id),
    scope_kind TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    model TEXT,
    source_kind TEXT NOT NULL CHECK(source_kind IN (
        'host_reported', 'local_tokenizer', 'conservative_byte_upper_bound', 'unknown'
    )),
    input_tokens INTEGER,
    output_tokens INTEGER,
    cached_input_tokens INTEGER,
    reasoning_tokens INTEGER,
    context_bytes INTEGER,
    outcome TEXT NOT NULL CHECK(outcome IN (
        'unverified', 'verified_success', 'verified_failure'
    )),
    verification_ref TEXT,
    receipt_sha256 TEXT NOT NULL,
    recorded_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX token_usage_project_sequence
    ON token_usage_receipts(project_id, sequence DESC);
CREATE INDEX token_usage_scope
    ON token_usage_receipts(project_id, scope_kind, scope_id);

PRAGMA user_version = 10;

COMMIT;
