BEGIN IMMEDIATE;

CREATE TABLE prompt_trials (
    trial_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    brief_receipt_id TEXT NOT NULL REFERENCES task_brief_receipts(receipt_id),
    idempotency_key TEXT NOT NULL UNIQUE,
    template_id TEXT NOT NULL,
    response_sha256 TEXT NOT NULL,
    response_evidence_id TEXT REFERENCES evidence_artifacts(evidence_id),
    response_file_verified INTEGER NOT NULL CHECK(response_file_verified IN (0, 1)),
    reported_model TEXT,
    assessments_json TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX prompt_trials_task ON prompt_trials(task_id, created_at_unix_ms, trial_id);
CREATE INDEX prompt_trials_receipt ON prompt_trials(brief_receipt_id, created_at_unix_ms);

PRAGMA user_version = 24;
COMMIT;
