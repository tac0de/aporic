BEGIN IMMEDIATE;

CREATE TABLE office_appointments (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    office_appointment_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    office_id TEXT NOT NULL CHECK(office_id = 'product.experiment'),
    office_version INTEGER NOT NULL CHECK(office_version = 1),
    role_appointment_id TEXT NOT NULL REFERENCES role_appointments(appointment_id),
    assignee_id TEXT NOT NULL,
    appointment_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    revoked_at_unix_ms INTEGER
);

CREATE UNIQUE INDEX office_appointments_active_head
    ON office_appointments(session_id, office_id)
    WHERE revoked_at_unix_ms IS NULL;
CREATE INDEX office_appointments_project
    ON office_appointments(project_id, created_at_unix_ms DESC);

CREATE TABLE product_cells (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    cell_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    task_id TEXT NOT NULL UNIQUE REFERENCES tasks(task_id),
    office_appointment_id TEXT NOT NULL REFERENCES office_appointments(office_appointment_id),
    title TEXT NOT NULL,
    problem_statement TEXT NOT NULL,
    hypothesis TEXT NOT NULL,
    success_measures_json TEXT NOT NULL,
    members_json TEXT NOT NULL,
    cell_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX product_cells_project
    ON product_cells(project_id, created_at_unix_ms DESC);

PRAGMA user_version = 17;
COMMIT;
