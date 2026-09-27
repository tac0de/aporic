CREATE TABLE initiatives (
    initiative_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX initiatives_project_created
    ON initiatives(project_id, created_at_unix_ms DESC);

PRAGMA user_version = 28;
