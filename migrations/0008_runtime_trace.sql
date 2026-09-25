BEGIN IMMEDIATE;

CREATE TABLE runtime_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    exposure_id TEXT REFERENCES memory_exposures(exposure_id),
    host_provider TEXT NOT NULL,
    event_kind TEXT NOT NULL CHECK(event_kind IN (
        'session_start', 'user_prompt', 'pre_tool', 'post_tool',
        'tool_failure', 'permission_request', 'stop', 'session_end', 'unknown'
    )),
    host_session_hmac TEXT,
    host_turn_hmac TEXT,
    host_tool_call_hmac TEXT,
    tool_name TEXT,
    capability_class TEXT NOT NULL CHECK(capability_class IN (
        'read', 'write', 'execute', 'network', 'external_mutation',
        'delegation', 'unknown'
    )),
    outcome_status TEXT NOT NULL CHECK(outcome_status IN (
        'proposed', 'succeeded', 'failed', 'unknown'
    )),
    input_hmac TEXT,
    input_bytes INTEGER NOT NULL DEFAULT 0,
    output_hmac TEXT,
    output_bytes INTEGER NOT NULL DEFAULT 0,
    latency_ms INTEGER,
    hook_schema_version TEXT,
    shadow_disposition TEXT NOT NULL CHECK(shadow_disposition IN (
        'observe', 'warn', 'would_ask', 'would_deny'
    )),
    shadow_reasons_json TEXT NOT NULL DEFAULT '[]',
    duplicate_of_event_id TEXT REFERENCES runtime_events(event_id),
    received_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX runtime_events_project_sequence
    ON runtime_events(project_id, sequence DESC);
CREATE INDEX runtime_events_correlation
    ON runtime_events(project_id, host_session_hmac, host_turn_hmac, host_tool_call_hmac);
CREATE INDEX runtime_events_kind_status
    ON runtime_events(project_id, event_kind, outcome_status);

CREATE TABLE capability_observations (
    observation_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    host_provider TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    capability_class TEXT NOT NULL CHECK(capability_class IN (
        'read', 'write', 'execute', 'network', 'external_mutation',
        'delegation', 'unknown'
    )),
    first_seen_at_unix_ms INTEGER NOT NULL,
    last_seen_at_unix_ms INTEGER NOT NULL,
    event_count INTEGER NOT NULL,
    succeeded_count INTEGER NOT NULL,
    failed_count INTEGER NOT NULL,
    UNIQUE(project_id, host_provider, tool_name, capability_class)
);

CREATE TRIGGER runtime_event_capability_projection
AFTER INSERT ON runtime_events
WHEN new.tool_name IS NOT NULL BEGIN
    INSERT INTO capability_observations(
        observation_id, project_id, host_provider, tool_name, capability_class,
        first_seen_at_unix_ms, last_seen_at_unix_ms, event_count,
        succeeded_count, failed_count
    ) VALUES (
        new.event_id, new.project_id, new.host_provider, new.tool_name,
        new.capability_class, new.received_at_unix_ms, new.received_at_unix_ms, 1,
        CASE WHEN new.outcome_status = 'succeeded' THEN 1 ELSE 0 END,
        CASE WHEN new.outcome_status = 'failed' THEN 1 ELSE 0 END
    ) ON CONFLICT(project_id, host_provider, tool_name, capability_class) DO UPDATE SET
        last_seen_at_unix_ms = excluded.last_seen_at_unix_ms,
        event_count = capability_observations.event_count + 1,
        succeeded_count = capability_observations.succeeded_count +
            CASE WHEN new.outcome_status = 'succeeded' THEN 1 ELSE 0 END,
        failed_count = capability_observations.failed_count +
            CASE WHEN new.outcome_status = 'failed' THEN 1 ELSE 0 END;
END;

PRAGMA user_version = 8;

COMMIT;
