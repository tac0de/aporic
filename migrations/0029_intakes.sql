CREATE TABLE intakes (
    intake_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    source_project TEXT NOT NULL,
    reproduction_evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX intakes_project_created ON intakes(project_id, created_at_unix_ms DESC);
CREATE INDEX intakes_task ON intakes(task_id);

PRAGMA user_version = 29;
