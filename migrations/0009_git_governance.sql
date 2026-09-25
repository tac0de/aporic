BEGIN IMMEDIATE;

CREATE TABLE git_snapshots (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    repository_root TEXT NOT NULL,
    head_commit TEXT,
    head_tree TEXT,
    branch TEXT,
    detached INTEGER NOT NULL CHECK(detached IN (0, 1)),
    upstream_ref TEXT,
    base_ref TEXT,
    base_commit TEXT,
    merge_base TEXT,
    ahead_count INTEGER,
    behind_count INTEGER,
    remote_state_fresh INTEGER NOT NULL DEFAULT 0 CHECK(remote_state_fresh IN (0, 1)),
    dirty INTEGER NOT NULL CHECK(dirty IN (0, 1)),
    staged_count INTEGER NOT NULL,
    unstaged_count INTEGER NOT NULL,
    untracked_count INTEGER NOT NULL,
    local_branch_count INTEGER NOT NULL,
    head_parent_count INTEGER,
    head_has_signature INTEGER NOT NULL CHECK(head_has_signature IN (0, 1)),
    successful_receipt_bound INTEGER NOT NULL CHECK(successful_receipt_bound IN (0, 1)),
    paths_truncated INTEGER NOT NULL CHECK(paths_truncated IN (0, 1)),
    changed_paths_json TEXT NOT NULL,
    worktrees_json TEXT NOT NULL,
    remotes_json TEXT NOT NULL,
    findings_json TEXT NOT NULL,
    policy_version INTEGER NOT NULL,
    snapshot_sha256 TEXT NOT NULL,
    captured_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX git_snapshots_project_sequence
    ON git_snapshots(project_id, sequence DESC);
CREATE INDEX git_snapshots_head
    ON git_snapshots(project_id, head_commit, captured_at_unix_ms DESC);

PRAGMA user_version = 9;

COMMIT;
