BEGIN IMMEDIATE;

CREATE TABLE accountability_cases (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    case_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    source_task_id TEXT NOT NULL REFERENCES tasks(task_id),
    evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    reported_assignee_id TEXT,
    expected_behavior TEXT NOT NULL,
    observed_behavior TEXT NOT NULL,
    impact TEXT NOT NULL,
    case_sha256 TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('open', 'repaired')),
    repair_task_id TEXT REFERENCES tasks(task_id),
    root_cause_hypothesis TEXT,
    prevention_change TEXT,
    plan_revision INTEGER NOT NULL DEFAULT 0,
    created_at_unix_ms INTEGER NOT NULL,
    repaired_at_unix_ms INTEGER
);

CREATE INDEX accountability_cases_project_status
    ON accountability_cases(project_id, status, created_at_unix_ms DESC);
CREATE INDEX accountability_cases_assignee
    ON accountability_cases(project_id, reported_assignee_id, created_at_unix_ms DESC);

PRAGMA user_version = 19;
COMMIT;
