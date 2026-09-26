BEGIN IMMEDIATE;

CREATE TABLE government_people (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    person_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    full_name TEXT NOT NULL,
    person_sha256 TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE(project_id, full_name)
);

CREATE INDEX government_people_project ON government_people(project_id, sequence);

CREATE TABLE government_roster_state (
    project_id TEXT PRIMARY KEY REFERENCES projects(project_id),
    initialized_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE government_terms (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    term_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    person_id TEXT NOT NULL REFERENCES government_people(person_id),
    position_id TEXT NOT NULL,
    position_json TEXT NOT NULL,
    predecessor_term_id TEXT REFERENCES government_terms(term_id),
    handoff_note TEXT,
    started_at_unix_ms INTEGER NOT NULL,
    ended_at_unix_ms INTEGER,
    end_reason TEXT,
    term_sha256 TEXT NOT NULL,
    CHECK ((ended_at_unix_ms IS NULL AND end_reason IS NULL)
        OR (ended_at_unix_ms IS NOT NULL AND end_reason IS NOT NULL))
);

CREATE UNIQUE INDEX government_terms_active_position
    ON government_terms(project_id, position_id)
    WHERE ended_at_unix_ms IS NULL;
CREATE UNIQUE INDEX government_terms_active_person
    ON government_terms(project_id, person_id)
    WHERE ended_at_unix_ms IS NULL;
CREATE INDEX government_terms_project ON government_terms(project_id, sequence);

PRAGMA user_version = 22;
COMMIT;
