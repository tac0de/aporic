BEGIN IMMEDIATE;

CREATE TABLE improvement_requests (
    request_id TEXT PRIMARY KEY,
    source_project_id TEXT NOT NULL REFERENCES projects(project_id),
    source_evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    core_project_id TEXT NOT NULL REFERENCES projects(project_id),
    task_id TEXT NOT NULL UNIQUE REFERENCES tasks(task_id),
    normalized_objective TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(source_project_id, core_project_id, normalized_objective)
);
CREATE INDEX improvement_requests_source ON improvement_requests(source_project_id, created_at_unix_ms DESC);
CREATE INDEX improvement_requests_core ON improvement_requests(core_project_id, created_at_unix_ms DESC);

CREATE TABLE prototype_briefs (
    brief_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    task_id TEXT NOT NULL UNIQUE REFERENCES tasks(task_id),
    target_user TEXT NOT NULL,
    core_experience_hypothesis TEXT NOT NULL,
    fidelity TEXT NOT NULL,
    reuse_boundary TEXT NOT NULL,
    technology_stack TEXT NOT NULL,
    repository_boundary TEXT NOT NULL,
    validation_method TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);
CREATE TABLE prototype_reviews (
    review_id TEXT PRIMARY KEY,
    brief_id TEXT NOT NULL REFERENCES prototype_briefs(brief_id),
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    smoke_evidence_json TEXT NOT NULL,
    rule_evidence_json TEXT NOT NULL,
    user_play_evidence_json TEXT NOT NULL,
    smoke_state TEXT NOT NULL,
    rule_state TEXT NOT NULL,
    user_value_state TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);
CREATE INDEX prototype_reviews_task ON prototype_reviews(task_id, created_at_unix_ms DESC);

PRAGMA user_version = 20;
COMMIT;
