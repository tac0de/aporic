BEGIN IMMEDIATE;

CREATE TABLE research_documents (
    document_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    source TEXT NOT NULL CHECK(source IN ('github', 'stackoverflow')),
    source_id TEXT NOT NULL,
    canonical_url TEXT NOT NULL,
    current_revision_sequence INTEGER,
    UNIQUE(project_id, source, source_id)
);

CREATE TABLE research_revisions (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    revision_id TEXT NOT NULL UNIQUE,
    document_id TEXT NOT NULL REFERENCES research_documents(document_id),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    author_name TEXT,
    content_license TEXT,
    published_at_unix_ms INTEGER,
    fetched_at_unix_ms INTEGER NOT NULL,
    content_sha256 TEXT NOT NULL,
    source_url TEXT NOT NULL
);

CREATE INDEX research_revisions_document ON research_revisions(document_id, sequence DESC);

CREATE VIRTUAL TABLE research_fts USING fts5(
    title, body, content='research_revisions', content_rowid='sequence'
);

CREATE TRIGGER research_revisions_fts_insert AFTER INSERT ON research_revisions BEGIN
    INSERT INTO research_fts(rowid, title, body) VALUES (new.sequence, new.title, new.body);
END;

PRAGMA user_version = 18;
COMMIT;
