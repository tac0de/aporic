BEGIN IMMEDIATE;

ALTER TABLE orchestration_runs ADD COLUMN role_appointment_id TEXT REFERENCES role_appointments(appointment_id);

PRAGMA user_version = 16;
COMMIT;
