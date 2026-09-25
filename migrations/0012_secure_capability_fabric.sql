BEGIN IMMEDIATE;

CREATE TABLE capability_manifests (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    capability_id TEXT NOT NULL,
    version TEXT NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    provider_kind TEXT NOT NULL CHECK(provider_kind IN (
        'built_in', 'external_artifact', 'workspace_manifest'
    )),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    effect_class TEXT NOT NULL CHECK(effect_class IN (
        'observe', 'record_local', 'verify_local', 'external_effect', 'privileged'
    )),
    reads_private_data INTEGER NOT NULL CHECK(reads_private_data IN (0, 1)),
    sees_untrusted_content INTEGER NOT NULL CHECK(sees_untrusted_content IN (0, 1)),
    uses_network INTEGER NOT NULL CHECK(uses_network IN (0, 1)),
    requires_credentials INTEGER NOT NULL CHECK(requires_credentials IN (0, 1)),
    idempotent INTEGER NOT NULL CHECK(idempotent IN (0, 1)),
    reversible INTEGER NOT NULL CHECK(reversible IN (0, 1)),
    input_schema_json TEXT NOT NULL,
    output_schema_json TEXT NOT NULL,
    evidence_contract TEXT NOT NULL,
    implementation_sha256 TEXT,
    manifest_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(project_id, capability_id, version)
);

CREATE INDEX capability_manifests_project_sequence
    ON capability_manifests(project_id, sequence DESC);

CREATE TABLE capability_state_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    state_event_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    capability_id TEXT NOT NULL,
    version TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN (
        'registered', 'available', 'disabled', 'deprecated', 'revoked'
    )),
    reason TEXT NOT NULL,
    event_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    FOREIGN KEY(project_id, capability_id, version)
        REFERENCES capability_manifests(project_id, capability_id, version)
);

CREATE INDEX capability_state_project_sequence
    ON capability_state_events(project_id, sequence DESC);

CREATE TABLE experiment_campaigns (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    campaign_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    title TEXT NOT NULL,
    problem TEXT NOT NULL,
    target_user TEXT NOT NULL,
    desired_outcome TEXT NOT NULL,
    hypothesis TEXT NOT NULL,
    git_snapshot_id TEXT NOT NULL REFERENCES git_snapshots(snapshot_id),
    bound_base_commit TEXT NOT NULL,
    bound_base_tree TEXT NOT NULL,
    max_variants INTEGER NOT NULL CHECK(max_variants BETWEEN 2 AND 16),
    max_token_budget INTEGER CHECK(max_token_budget IS NULL OR max_token_budget > 0),
    campaign_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX experiment_campaigns_project_sequence
    ON experiment_campaigns(project_id, sequence DESC);

CREATE TABLE experiment_criteria (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    criterion_id TEXT NOT NULL UNIQUE,
    campaign_id TEXT NOT NULL REFERENCES experiment_campaigns(campaign_id),
    contract_revision INTEGER NOT NULL CHECK(contract_revision > 0),
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('hard_gate', 'pareto_dimension')),
    comparison TEXT NOT NULL CHECK(comparison IN (
        'must_pass', 'minimize', 'maximize', 'gte', 'lte'
    )),
    threshold INTEGER,
    unit TEXT NOT NULL,
    criterion_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(campaign_id, contract_revision, name)
);

CREATE INDEX experiment_criteria_campaign_revision
    ON experiment_criteria(campaign_id, contract_revision, sequence ASC);

CREATE TABLE experiment_variants (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    variant_id TEXT NOT NULL UNIQUE,
    campaign_id TEXT NOT NULL REFERENCES experiment_campaigns(campaign_id),
    name TEXT NOT NULL,
    diversity_axis TEXT NOT NULL CHECK(diversity_axis IN (
        'product_assumption', 'ux', 'architecture', 'data_model',
        'automation', 'cost_safety'
    )),
    approach TEXT NOT NULL,
    approach_sha256 TEXT NOT NULL,
    git_snapshot_id TEXT NOT NULL REFERENCES git_snapshots(snapshot_id),
    head_commit TEXT NOT NULL,
    head_tree TEXT NOT NULL,
    parent_variant_ids_json TEXT NOT NULL,
    variant_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(campaign_id, approach_sha256)
);

CREATE INDEX experiment_variants_campaign_sequence
    ON experiment_variants(campaign_id, sequence ASC);

CREATE TABLE experiment_measurements (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    measurement_id TEXT NOT NULL UNIQUE,
    campaign_id TEXT NOT NULL REFERENCES experiment_campaigns(campaign_id),
    variant_id TEXT NOT NULL REFERENCES experiment_variants(variant_id),
    criterion_id TEXT NOT NULL REFERENCES experiment_criteria(criterion_id),
    value INTEGER NOT NULL,
    evidence_id TEXT REFERENCES evidence_artifacts(evidence_id),
    claim_id TEXT REFERENCES claims(claim_id),
    measurement_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(variant_id, criterion_id)
);

CREATE INDEX experiment_measurements_campaign_sequence
    ON experiment_measurements(campaign_id, sequence ASC);

CREATE TABLE experiment_decisions (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    decision_id TEXT NOT NULL UNIQUE,
    campaign_id TEXT NOT NULL REFERENCES experiment_campaigns(campaign_id),
    kind TEXT NOT NULL CHECK(kind IN (
        'advance', 'eliminate', 'synthesize', 'abandon', 'select'
    )),
    variant_id TEXT REFERENCES experiment_variants(variant_id),
    summary TEXT NOT NULL,
    deliberation_id TEXT REFERENCES deliberations(deliberation_id),
    deliberation_decision_id TEXT REFERENCES deliberation_decisions(decision_id),
    qualified INTEGER NOT NULL CHECK(qualified IN (0, 1)),
    decision_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX experiment_decisions_campaign_sequence
    ON experiment_decisions(campaign_id, sequence ASC);

CREATE TABLE security_assessments (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    assessment_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    provider_capability_id TEXT NOT NULL,
    provider_version TEXT NOT NULL,
    source_scan_id TEXT NOT NULL,
    git_snapshot_id TEXT NOT NULL REFERENCES git_snapshots(snapshot_id),
    target_commit TEXT NOT NULL,
    target_tree TEXT NOT NULL,
    coverage TEXT NOT NULL CHECK(coverage IN ('complete', 'partial', 'unknown')),
    reportable_critical INTEGER NOT NULL CHECK(reportable_critical >= 0),
    reportable_high INTEGER NOT NULL CHECK(reportable_high >= 0),
    reportable_medium INTEGER NOT NULL CHECK(reportable_medium >= 0),
    reportable_low INTEGER NOT NULL CHECK(reportable_low >= 0),
    manifest_evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    findings_evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    coverage_evidence_id TEXT NOT NULL REFERENCES evidence_artifacts(evidence_id),
    assessment_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(project_id, source_scan_id)
);

CREATE INDEX security_assessments_project_sequence
    ON security_assessments(project_id, sequence DESC);

PRAGMA user_version = 12;

COMMIT;
