BEGIN IMMEDIATE;

CREATE TABLE rustc_input_snapshots (
    run_id TEXT PRIMARY KEY REFERENCES execution_runs(run_id),
    workspace_relative_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    captured_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE rust_repair_cases (
    case_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    diagnostic_code TEXT NOT NULL,
    rustc_version TEXT NOT NULL,
    edition TEXT NOT NULL,
    diagnostic_evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    reproduction_evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    reproduction_run_id TEXT NOT NULL REFERENCES execution_runs(run_id),
    reference_evidence_id TEXT REFERENCES evidence_artifacts(evidence_id),
    created_at_unix_ms INTEGER NOT NULL
);
CREATE INDEX rust_repair_cases_project_code ON rust_repair_cases(project_id, diagnostic_code, created_at_unix_ms DESC);

CREATE TABLE rust_repair_lessons (
    case_id TEXT PRIMARY KEY REFERENCES rust_repair_cases(case_id),
    rule TEXT NOT NULL,
    applicability TEXT NOT NULL,
    counterexample TEXT NOT NULL,
    test_run_id TEXT NOT NULL REFERENCES execution_runs(run_id),
    learned_at_unix_ms INTEGER NOT NULL
);

PRAGMA user_version = 30;
COMMIT;
