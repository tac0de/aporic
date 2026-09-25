BEGIN IMMEDIATE;

UPDATE records
SET influence_class = 'untrusted_content'
WHERE origin_channel = 'mcp_agent'
  AND influence_class = 'historical_context';

CREATE TABLE memory_items (
    memory_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    source_kind TEXT NOT NULL CHECK(source_kind IN (
        'record', 'claim', 'task', 'handoff', 'execution'
    )),
    source_id TEXT NOT NULL,
    memory_class TEXT NOT NULL CHECK(memory_class IN (
        'episodic', 'semantic', 'procedural', 'gotcha', 'unknown'
    )),
    content TEXT NOT NULL,
    origin_channel TEXT NOT NULL CHECK(origin_channel IN (
        'legacy', 'mcp_agent', 'local_runner', 'codex_hook'
    )),
    influence_class TEXT NOT NULL CHECK(influence_class IN (
        'verified_fact', 'historical_context', 'untrusted_content'
    )),
    source_status TEXT,
    lifecycle_state TEXT NOT NULL CHECK(lifecycle_state IN (
        'active', 'superseded', 'quarantined', 'tombstoned'
    )),
    valid_from_unix_ms INTEGER NOT NULL,
    valid_until_unix_ms INTEGER,
    applicability_json TEXT NOT NULL DEFAULT '{}',
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    UNIQUE(source_kind, source_id)
);

CREATE INDEX memory_items_project_lifecycle
    ON memory_items(project_id, lifecycle_state, memory_class, updated_at_unix_ms DESC);
CREATE INDEX memory_items_source
    ON memory_items(source_kind, source_id);

CREATE TABLE memory_edges (
    edge_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    from_memory_id TEXT NOT NULL REFERENCES memory_items(memory_id),
    to_memory_id TEXT NOT NULL REFERENCES memory_items(memory_id),
    relation TEXT NOT NULL CHECK(relation IN (
        'supersedes', 'supports', 'contradicts', 'derived_from', 'caused_by', 'applies_to'
    )),
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(from_memory_id, to_memory_id, relation)
);

CREATE TABLE memory_exposures (
    exposure_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    event_kind TEXT NOT NULL,
    host_session_hmac TEXT,
    host_turn_hmac TEXT,
    policy_sha256 TEXT NOT NULL,
    memory_ids_json TEXT NOT NULL,
    content_bytes INTEGER NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX memory_exposures_project_created
    ON memory_exposures(project_id, created_at_unix_ms DESC);

CREATE TABLE installation_secrets (
    name TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE VIRTUAL TABLE memory_fts USING fts5(
    memory_id UNINDEXED,
    content,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER memory_items_fts_insert AFTER INSERT ON memory_items BEGIN
    INSERT INTO memory_fts(memory_id, content) VALUES (new.memory_id, new.content);
END;

CREATE TRIGGER memory_items_fts_update AFTER UPDATE OF content ON memory_items BEGIN
    DELETE FROM memory_fts WHERE memory_id = old.memory_id;
    INSERT INTO memory_fts(memory_id, content) VALUES (new.memory_id, new.content);
END;

CREATE TRIGGER memory_items_fts_delete AFTER DELETE ON memory_items BEGIN
    DELETE FROM memory_fts WHERE memory_id = old.memory_id;
END;

INSERT INTO memory_items (
    memory_id, project_id, source_kind, source_id, memory_class, content,
    origin_channel, influence_class, source_status, lifecycle_state,
    valid_from_unix_ms, valid_until_unix_ms, applicability_json,
    created_at_unix_ms, updated_at_unix_ms
)
SELECT
    'record:' || records.record_id, sessions.project_id, 'record', records.record_id,
    CASE records.kind
        WHEN 'decision' THEN 'semantic'
        WHEN 'constraint' THEN 'semantic'
        WHEN 'material_unknown' THEN 'unknown'
        ELSE 'episodic'
    END,
    records.content, records.origin_channel, records.influence_class, records.kind,
    CASE WHEN EXISTS (
        SELECT 1 FROM records newer WHERE newer.supersedes_record_id = records.record_id
    ) THEN 'superseded' ELSE 'active' END,
    records.created_at_unix_ms,
    (SELECT newer.created_at_unix_ms FROM records newer
     WHERE newer.supersedes_record_id = records.record_id LIMIT 1),
    '{}', records.created_at_unix_ms, records.created_at_unix_ms
FROM records JOIN sessions ON sessions.session_id = records.session_id;

INSERT INTO memory_items (
    memory_id, project_id, source_kind, source_id, memory_class, content,
    origin_channel, influence_class, source_status, lifecycle_state,
    valid_from_unix_ms, valid_until_unix_ms, applicability_json,
    created_at_unix_ms, updated_at_unix_ms
)
SELECT
    'claim:' || claims.claim_id, sessions.project_id, 'claim', claims.claim_id,
    CASE WHEN claims.status = 'unknown' THEN 'unknown' ELSE 'semantic' END,
    claims.statement, 'mcp_agent',
    CASE WHEN claims.status IN ('observed', 'verified') THEN 'verified_fact'
         ELSE 'untrusted_content' END,
    claims.status,
    CASE WHEN EXISTS (
        SELECT 1 FROM claims newer WHERE newer.supersedes_claim_id = claims.claim_id
    ) THEN 'superseded' ELSE 'active' END,
    claims.created_at_unix_ms,
    (SELECT newer.created_at_unix_ms FROM claims newer
     WHERE newer.supersedes_claim_id = claims.claim_id LIMIT 1),
    '{}', claims.created_at_unix_ms, claims.created_at_unix_ms
FROM claims JOIN sessions ON sessions.session_id = claims.session_id;

INSERT INTO memory_items (
    memory_id, project_id, source_kind, source_id, memory_class, content,
    origin_channel, influence_class, source_status, lifecycle_state,
    valid_from_unix_ms, valid_until_unix_ms, applicability_json,
    created_at_unix_ms, updated_at_unix_ms
)
SELECT
    'task:' || task_id, project_id, 'task', task_id, 'procedural', objective,
    'mcp_agent', 'untrusted_content', status, 'active', created_at_unix_ms, NULL,
    json_object('write_scope', json(write_scope_json), 'depends_on', json(depends_on_json)),
    created_at_unix_ms, updated_at_unix_ms
FROM tasks;

INSERT INTO memory_items (
    memory_id, project_id, source_kind, source_id, memory_class, content,
    origin_channel, influence_class, source_status, lifecycle_state,
    valid_from_unix_ms, valid_until_unix_ms, applicability_json,
    created_at_unix_ms, updated_at_unix_ms
)
SELECT
    'handoff:' || session_id, project_id, 'handoff', session_id, 'episodic',
    summary || ' Next action: ' || next_action,
    'mcp_agent', 'untrusted_content', 'handoff', 'active',
    closed_at_unix_ms, NULL, '{}', closed_at_unix_ms, closed_at_unix_ms
FROM sessions
WHERE status = 'handoff' AND summary IS NOT NULL AND next_action IS NOT NULL;

INSERT INTO memory_items (
    memory_id, project_id, source_kind, source_id, memory_class, content,
    origin_channel, influence_class, source_status, lifecycle_state,
    valid_from_unix_ms, valid_until_unix_ms, applicability_json,
    created_at_unix_ms, updated_at_unix_ms
)
SELECT
    'execution:' || execution_runs.run_id, sessions.project_id, 'execution',
    execution_runs.run_id,
    CASE WHEN execution_runs.status = 'succeeded' THEN 'episodic' ELSE 'gotcha' END,
    'Execution ' || execution_runs.status || ' for specification ' || execution_runs.spec_id ||
        ' with termination ' || execution_receipts.termination,
    'local_runner', 'verified_fact', execution_runs.status, 'active',
    execution_runs.started_at_unix_ms, NULL,
    json_object('spec_id', execution_runs.spec_id,
                'command_spec_sha256', execution_receipts.command_spec_sha256),
    execution_runs.started_at_unix_ms, execution_receipts.created_at_unix_ms
FROM execution_runs
JOIN execution_receipts ON execution_receipts.run_id = execution_runs.run_id
JOIN sessions ON sessions.session_id = execution_runs.session_id;

INSERT INTO memory_edges (
    edge_id, project_id, from_memory_id, to_memory_id, relation, created_at_unix_ms
)
SELECT
    'record-supersedes:' || newer.record_id, sessions.project_id,
    'record:' || newer.record_id, 'record:' || older.record_id,
    'supersedes', newer.created_at_unix_ms
FROM records newer
JOIN records older ON older.record_id = newer.supersedes_record_id
JOIN sessions ON sessions.session_id = newer.session_id;

INSERT INTO memory_edges (
    edge_id, project_id, from_memory_id, to_memory_id, relation, created_at_unix_ms
)
SELECT
    'claim-supersedes:' || newer.claim_id, sessions.project_id,
    'claim:' || newer.claim_id, 'claim:' || older.claim_id,
    'supersedes', newer.created_at_unix_ms
FROM claims newer
JOIN claims older ON older.claim_id = newer.supersedes_claim_id
JOIN sessions ON sessions.session_id = newer.session_id;

CREATE TRIGGER records_memory_insert AFTER INSERT ON records BEGIN
    INSERT INTO memory_items (
        memory_id, project_id, source_kind, source_id, memory_class, content,
        origin_channel, influence_class, source_status, lifecycle_state,
        valid_from_unix_ms, applicability_json, created_at_unix_ms, updated_at_unix_ms
    ) SELECT
        'record:' || new.record_id, sessions.project_id, 'record', new.record_id,
        CASE new.kind WHEN 'decision' THEN 'semantic' WHEN 'constraint' THEN 'semantic'
             WHEN 'material_unknown' THEN 'unknown' ELSE 'episodic' END,
        new.content, new.origin_channel, new.influence_class, new.kind, 'active',
        new.created_at_unix_ms, '{}', new.created_at_unix_ms, new.created_at_unix_ms
      FROM sessions WHERE sessions.session_id = new.session_id;
    UPDATE memory_items
       SET lifecycle_state = 'superseded', valid_until_unix_ms = new.created_at_unix_ms,
           updated_at_unix_ms = new.created_at_unix_ms
     WHERE memory_id = 'record:' || new.supersedes_record_id
       AND new.supersedes_record_id IS NOT NULL;
    INSERT OR IGNORE INTO memory_edges
        (edge_id, project_id, from_memory_id, to_memory_id, relation, created_at_unix_ms)
      SELECT 'record-supersedes:' || new.record_id, sessions.project_id,
             'record:' || new.record_id, 'record:' || new.supersedes_record_id,
             'supersedes', new.created_at_unix_ms
        FROM sessions
       WHERE sessions.session_id = new.session_id AND new.supersedes_record_id IS NOT NULL;
END;

CREATE TRIGGER claims_memory_insert AFTER INSERT ON claims BEGIN
    INSERT INTO memory_items (
        memory_id, project_id, source_kind, source_id, memory_class, content,
        origin_channel, influence_class, source_status, lifecycle_state,
        valid_from_unix_ms, applicability_json, created_at_unix_ms, updated_at_unix_ms
    ) SELECT
        'claim:' || new.claim_id, sessions.project_id, 'claim', new.claim_id,
        CASE WHEN new.status = 'unknown' THEN 'unknown' ELSE 'semantic' END,
        new.statement, 'mcp_agent',
        CASE WHEN new.status IN ('observed', 'verified') THEN 'verified_fact'
             ELSE 'untrusted_content' END,
        new.status, 'active', new.created_at_unix_ms, '{}',
        new.created_at_unix_ms, new.created_at_unix_ms
      FROM sessions WHERE sessions.session_id = new.session_id;
    UPDATE memory_items
       SET lifecycle_state = 'superseded', valid_until_unix_ms = new.created_at_unix_ms,
           updated_at_unix_ms = new.created_at_unix_ms
     WHERE memory_id = 'claim:' || new.supersedes_claim_id
       AND new.supersedes_claim_id IS NOT NULL;
    INSERT OR IGNORE INTO memory_edges
        (edge_id, project_id, from_memory_id, to_memory_id, relation, created_at_unix_ms)
      SELECT 'claim-supersedes:' || new.claim_id, sessions.project_id,
             'claim:' || new.claim_id, 'claim:' || new.supersedes_claim_id,
             'supersedes', new.created_at_unix_ms
        FROM sessions
       WHERE sessions.session_id = new.session_id AND new.supersedes_claim_id IS NOT NULL;
END;

CREATE TRIGGER tasks_memory_insert AFTER INSERT ON tasks BEGIN
    INSERT INTO memory_items (
        memory_id, project_id, source_kind, source_id, memory_class, content,
        origin_channel, influence_class, source_status, lifecycle_state,
        valid_from_unix_ms, applicability_json, created_at_unix_ms, updated_at_unix_ms
    ) VALUES (
        'task:' || new.task_id, new.project_id, 'task', new.task_id, 'procedural',
        new.objective, 'mcp_agent', 'untrusted_content', new.status, 'active',
        new.created_at_unix_ms,
        json_object('write_scope', json(new.write_scope_json),
                    'depends_on', json(new.depends_on_json)),
        new.created_at_unix_ms, new.updated_at_unix_ms
    );
END;

CREATE TRIGGER tasks_memory_update AFTER UPDATE ON tasks BEGIN
    UPDATE memory_items
       SET content = new.objective, source_status = new.status,
           applicability_json = json_object('write_scope', json(new.write_scope_json),
                                             'depends_on', json(new.depends_on_json)),
           updated_at_unix_ms = new.updated_at_unix_ms
     WHERE memory_id = 'task:' || new.task_id;
END;

CREATE TRIGGER sessions_handoff_memory AFTER UPDATE OF status ON sessions
WHEN new.status = 'handoff' AND new.summary IS NOT NULL AND new.next_action IS NOT NULL BEGIN
    INSERT INTO memory_items (
        memory_id, project_id, source_kind, source_id, memory_class, content,
        origin_channel, influence_class, source_status, lifecycle_state,
        valid_from_unix_ms, applicability_json, created_at_unix_ms, updated_at_unix_ms
    ) VALUES (
        'handoff:' || new.session_id, new.project_id, 'handoff', new.session_id,
        'episodic', new.summary || ' Next action: ' || new.next_action,
        'mcp_agent', 'untrusted_content', 'handoff', 'active', new.closed_at_unix_ms,
        '{}', new.closed_at_unix_ms, new.closed_at_unix_ms
    ) ON CONFLICT(source_kind, source_id) DO UPDATE SET
        content = excluded.content, updated_at_unix_ms = excluded.updated_at_unix_ms;
END;

CREATE TRIGGER execution_terminal_memory AFTER UPDATE OF status ON execution_runs
WHEN new.status <> 'running' BEGIN
    INSERT INTO memory_items (
        memory_id, project_id, source_kind, source_id, memory_class, content,
        origin_channel, influence_class, source_status, lifecycle_state,
        valid_from_unix_ms, applicability_json, created_at_unix_ms, updated_at_unix_ms
    ) SELECT
        'execution:' || new.run_id, sessions.project_id, 'execution',
        new.run_id,
        CASE WHEN new.status = 'succeeded' THEN 'episodic' ELSE 'gotcha' END,
        'Execution ' || new.status || ' for specification ' || new.spec_id ||
            ' with termination ' || execution_receipts.termination,
        'local_runner', 'verified_fact', new.status, 'active',
        new.started_at_unix_ms,
        json_object('spec_id', new.spec_id,
                    'command_spec_sha256', execution_receipts.command_spec_sha256),
        new.started_at_unix_ms, execution_receipts.created_at_unix_ms
      FROM execution_receipts
      JOIN sessions ON sessions.session_id = new.session_id
     WHERE execution_receipts.run_id = new.run_id;
END;

PRAGMA user_version = 7;

COMMIT;
