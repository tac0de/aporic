BEGIN IMMEDIATE;

CREATE INDEX IF NOT EXISTS events_session_delegation_stream
    ON events(stream_id, sequence)
    WHERE kind IN ('session_delegation_assessed', 'session_delegation_reported');

PRAGMA user_version = 27;

COMMIT;
