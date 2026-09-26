BEGIN IMMEDIATE;

CREATE TABLE task_brief_receipts (
    receipt_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    idempotency_key TEXT NOT NULL UNIQUE,
    template_id TEXT NOT NULL,
    template_version INTEGER NOT NULL,
    template_sha256 TEXT NOT NULL,
    context_policy_sha256 TEXT NOT NULL,
    selected_item_ids_json TEXT NOT NULL,
    brief_sha256 TEXT NOT NULL,
    brief_bytes INTEGER NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX task_brief_receipts_task
    ON task_brief_receipts(task_id, created_at_unix_ms, receipt_id);

PRAGMA user_version = 23;
COMMIT;
