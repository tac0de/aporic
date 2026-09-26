BEGIN IMMEDIATE;

CREATE TABLE task_memory_uses (
    use_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    memory_id TEXT NOT NULL,
    criterion TEXT NOT NULL,
    intended_action TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(task_id, memory_id, criterion)
);

CREATE INDEX task_memory_uses_task ON task_memory_uses(task_id, created_at_unix_ms, use_id);

PRAGMA user_version = 21;
COMMIT;
