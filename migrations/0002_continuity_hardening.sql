ALTER TABLE sessions ADD COLUMN last_activity_at_unix_ms INTEGER;
UPDATE sessions
SET last_activity_at_unix_ms = COALESCE(closed_at_unix_ms, opened_at_unix_ms);

ALTER TABLE sessions ADD COLUMN abandoned INTEGER NOT NULL DEFAULT 0;

ALTER TABLE records ADD COLUMN supersedes_record_id TEXT REFERENCES records(record_id);
ALTER TABLE records ADD COLUMN verifies_effect_id TEXT REFERENCES records(record_id);

CREATE UNIQUE INDEX records_supersedes_once
    ON records(supersedes_record_id)
    WHERE supersedes_record_id IS NOT NULL;

CREATE UNIQUE INDEX records_verifies_effect_once
    ON records(verifies_effect_id)
    WHERE verifies_effect_id IS NOT NULL;

CREATE INDEX records_supersession_lookup
    ON records(supersedes_record_id, created_at_unix_ms DESC);

PRAGMA user_version = 2;
