BEGIN IMMEDIATE;

CREATE TABLE deliberations (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    deliberation_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    title TEXT NOT NULL,
    question TEXT NOT NULL,
    git_snapshot_id TEXT NOT NULL REFERENCES git_snapshots(snapshot_id),
    bound_head_commit TEXT,
    bound_head_tree TEXT,
    deliberation_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX deliberations_project_sequence
    ON deliberations(project_id, sequence DESC);

CREATE TABLE deliberation_nodes (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL UNIQUE,
    deliberation_id TEXT NOT NULL REFERENCES deliberations(deliberation_id),
    kind TEXT NOT NULL CHECK(kind IN (
        'question', 'premise', 'claim', 'objection', 'counterexample',
        'falsifier', 'value_constraint', 'material_unknown', 'proposal', 'revision'
    )),
    statement TEXT NOT NULL,
    material INTEGER NOT NULL CHECK(material IN (0, 1)),
    evidence_id TEXT REFERENCES evidence_artifacts(evidence_id),
    claim_id TEXT REFERENCES claims(claim_id),
    node_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX deliberation_nodes_graph_sequence
    ON deliberation_nodes(deliberation_id, sequence ASC);

CREATE TABLE deliberation_edges (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    edge_id TEXT NOT NULL UNIQUE,
    deliberation_id TEXT NOT NULL REFERENCES deliberations(deliberation_id),
    source_node_id TEXT NOT NULL REFERENCES deliberation_nodes(node_id),
    target_node_id TEXT NOT NULL REFERENCES deliberation_nodes(node_id),
    kind TEXT NOT NULL CHECK(kind IN (
        'support', 'attack', 'undercut', 'dependency', 'falsification', 'revision'
    )),
    edge_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(deliberation_id, source_node_id, target_node_id, kind)
);

CREATE INDEX deliberation_edges_graph_sequence
    ON deliberation_edges(deliberation_id, sequence ASC);

CREATE TABLE deliberation_decisions (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    decision_id TEXT NOT NULL UNIQUE,
    deliberation_id TEXT NOT NULL REFERENCES deliberations(deliberation_id),
    proposal_node_id TEXT NOT NULL REFERENCES deliberation_nodes(node_id),
    summary TEXT NOT NULL,
    open_material_issues INTEGER NOT NULL,
    decision_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX deliberation_decisions_graph_sequence
    ON deliberation_decisions(deliberation_id, sequence ASC);

PRAGMA user_version = 11;

COMMIT;
