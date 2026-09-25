BEGIN IMMEDIATE;

CREATE TABLE orchestration_runs (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    git_snapshot_id TEXT NOT NULL REFERENCES git_snapshots(snapshot_id),
    context_exposure_id TEXT NOT NULL REFERENCES memory_exposures(exposure_id),
    mode TEXT NOT NULL CHECK(mode IN ('blind_shadow', 'visible_advisory')),
    role_kind TEXT NOT NULL CHECK(role_kind IN ('steward', 'worker')),
    role_id TEXT NOT NULL,
    objective TEXT NOT NULL,
    model TEXT,
    reasoning_effort TEXT,
    max_input_tokens INTEGER NOT NULL CHECK(max_input_tokens BETWEEN 1 AND 10000000),
    max_output_tokens INTEGER NOT NULL CHECK(max_output_tokens BETWEEN 1 AND 1000000),
    max_duration_seconds INTEGER NOT NULL CHECK(max_duration_seconds BETWEEN 1 AND 86400),
    capability_ceiling TEXT NOT NULL DEFAULT 'propose' CHECK(capability_ceiling = 'propose'),
    status TEXT NOT NULL CHECK(status IN ('awaiting_report', 'sealed', 'evaluated')),
    bound_head_commit TEXT NOT NULL,
    bound_head_tree TEXT NOT NULL,
    context_policy_sha256 TEXT NOT NULL,
    run_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    sealed_at_unix_ms INTEGER,
    evaluated_at_unix_ms INTEGER
);

CREATE INDEX orchestration_runs_project_created
    ON orchestration_runs(project_id, created_at_unix_ms DESC, run_id DESC);
CREATE INDEX orchestration_runs_task
    ON orchestration_runs(task_id, created_at_unix_ms DESC);

CREATE TABLE advisory_role_reports (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    report_id TEXT NOT NULL UNIQUE,
    run_id TEXT NOT NULL UNIQUE REFERENCES orchestration_runs(run_id),
    source_kind TEXT NOT NULL CHECK(source_kind IN ('host_reported', 'deterministic_simulator')),
    disposition TEXT NOT NULL CHECK(disposition IN ('abstain', 'recommend', 'flag_risk', 'propose_work')),
    predicted_task_outcome TEXT NOT NULL CHECK(predicted_task_outcome IN ('completion', 'non_completion', 'uncertain')),
    summary TEXT NOT NULL,
    public_rationale TEXT NOT NULL,
    recommended_next_action TEXT,
    uncertainties_json TEXT NOT NULL,
    addressed_criteria_json TEXT NOT NULL,
    reported_input_tokens INTEGER,
    reported_output_tokens INTEGER,
    reported_duration_ms INTEGER,
    report_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE shadow_evaluations (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    evaluation_id TEXT NOT NULL UNIQUE,
    run_id TEXT NOT NULL UNIQUE REFERENCES orchestration_runs(run_id),
    actual_task_outcome TEXT NOT NULL CHECK(actual_task_outcome IN ('completed', 'cancelled')),
    prediction_match INTEGER CHECK(prediction_match IS NULL OR prediction_match IN (0, 1)),
    acceptance_criterion_count INTEGER NOT NULL,
    addressed_criterion_count INTEGER NOT NULL,
    verified_criterion_count INTEGER NOT NULL,
    git_stale INTEGER NOT NULL CHECK(git_stale IN (0, 1)),
    evidence_eligible INTEGER NOT NULL CHECK(evidence_eligible IN (0, 1)),
    evaluation_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

PRAGMA user_version = 14;
COMMIT;
