ALTER TABLE tasks ADD COLUMN completion_proofs_json TEXT NOT NULL DEFAULT '[]';

CREATE TABLE evidence_artifacts (
    evidence_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    kind TEXT NOT NULL CHECK(kind IN (
        'workspace_file', 'command_result', 'external_source',
        'user_statement', 'model_assessment'
    )),
    grade TEXT NOT NULL CHECK(grade IN ('direct', 'reported', 'model_only')),
    locator TEXT NOT NULL,
    summary TEXT NOT NULL,
    content_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX evidence_session_created
    ON evidence_artifacts(session_id, created_at_unix_ms DESC);

CREATE TABLE claims (
    claim_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    status TEXT NOT NULL CHECK(status IN (
        'observed', 'verified', 'inferred', 'assumed', 'intended', 'unknown'
    )),
    statement TEXT NOT NULL,
    material INTEGER NOT NULL CHECK(material IN (0, 1)),
    supersedes_claim_id TEXT REFERENCES claims(claim_id),
    created_at_unix_ms INTEGER NOT NULL
);

CREATE UNIQUE INDEX claims_supersedes_once
    ON claims(supersedes_claim_id)
    WHERE supersedes_claim_id IS NOT NULL;

CREATE INDEX claims_session_created
    ON claims(session_id, created_at_unix_ms DESC);

CREATE TABLE claim_evidence (
    claim_id TEXT NOT NULL REFERENCES claims(claim_id),
    evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    PRIMARY KEY (claim_id, evidence_id)
);

PRAGMA user_version = 4;
