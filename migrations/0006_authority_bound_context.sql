ALTER TABLE records
ADD COLUMN origin_channel TEXT NOT NULL DEFAULT 'legacy'
CHECK(origin_channel IN ('legacy', 'mcp_agent', 'local_runner', 'codex_hook'));

ALTER TABLE records
ADD COLUMN influence_class TEXT NOT NULL DEFAULT 'historical_context'
CHECK(influence_class IN ('verified_fact', 'historical_context', 'untrusted_content'));

CREATE INDEX records_context_selection
    ON records(session_id, influence_class, created_at_unix_ms DESC);

PRAGMA user_version = 6;
