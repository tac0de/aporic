BEGIN IMMEDIATE;

ALTER TABLE claims ADD COLUMN subject_key TEXT;
ALTER TABLE evidence_artifacts ADD COLUMN canonical_locator TEXT;

CREATE INDEX claims_subject_key ON claims(subject_key) WHERE subject_key IS NOT NULL;

-- v7 backfilled already-superseded rows with the source creation time as
-- updated_at. Later live supersession writes used the successor time. Normalize
-- this rebuildable projection so field-level audits have one deterministic rule.
UPDATE memory_items
SET updated_at_unix_ms = valid_until_unix_ms
WHERE source_kind IN ('record', 'claim')
  AND lifecycle_state = 'superseded'
  AND valid_until_unix_ms IS NOT NULL
  AND updated_at_unix_ms = created_at_unix_ms;

PRAGMA user_version = 26;
COMMIT;
