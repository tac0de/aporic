BEGIN IMMEDIATE;

ALTER TABLE verification_specs
    ADD COLUMN sandbox_profile_json TEXT NOT NULL
    DEFAULT '{"enforcement":"host","workspace_access":"read_write","network_access":"inherit"}';

ALTER TABLE execution_receipts
    ADD COLUMN sandbox_backend TEXT NOT NULL DEFAULT 'none';
ALTER TABLE execution_receipts
    ADD COLUMN sandbox_enforced INTEGER NOT NULL DEFAULT 0
    CHECK(sandbox_enforced IN (0, 1));

ALTER TABLE capability_manifests
    ADD COLUMN maturity TEXT NOT NULL DEFAULT 'observe'
    CHECK(maturity IN (
        'observe', 'propose', 'sandboxed_execute',
        'connected_effect', 'persistent_routine'
    ));
ALTER TABLE capability_manifests
    ADD COLUMN manifest_digest_version INTEGER NOT NULL DEFAULT 1
    CHECK(manifest_digest_version IN (1, 2));

UPDATE capability_manifests
SET maturity = CASE effect_class
    WHEN 'observe' THEN 'observe'
    WHEN 'record_local' THEN 'propose'
    WHEN 'verify_local' THEN 'sandboxed_execute'
    ELSE 'connected_effect'
END;

PRAGMA user_version = 13;
COMMIT;
