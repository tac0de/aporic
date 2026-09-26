BEGIN IMMEDIATE;

CREATE TABLE task_research_items (
    item_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    idempotency_key TEXT NOT NULL UNIQUE,
    revision_id TEXT REFERENCES research_revisions(revision_id),
    source TEXT NOT NULL CHECK(source IN ('github', 'stackoverflow', 'reddit', 'linkedin')),
    source_url TEXT NOT NULL,
    title TEXT NOT NULL,
    excerpt TEXT NOT NULL,
    content_sha256 TEXT NOT NULL,
    provenance TEXT NOT NULL CHECK(provenance IN ('aporic_api', 'host_reported')),
    relevance_note TEXT NOT NULL,
    observed_at_unix_ms INTEGER NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    CHECK ((revision_id IS NOT NULL AND provenance = 'aporic_api') OR
           (revision_id IS NULL AND provenance = 'host_reported'))
);

CREATE INDEX task_research_items_task ON task_research_items(task_id, created_at_unix_ms, item_id);

PRAGMA user_version = 25;
COMMIT;
