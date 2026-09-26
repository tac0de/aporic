# Restore destination correction — stage 1

## Scope and status

Investigation only. Implement in `crates/aporic/src/recovery.rs` and
`crates/aporic/tests/recovery.rs` in the next session; neither file was edited.
Read `TASK.md`, recovery source/tests, `Store::validate_backup` and error types,
and the CLI restore entry point. No network, sibling workspace, operator-file,
commit, push, or deployment access. Aporic MCP was unavailable in the exposed
tools; this file provides the continuity handoff. This copy has no usable Git
repository (`git status --short` failed).

## Observed bug and current behavior

`restore_to` validates the source first, then checks `destination.exists()`.
That check follows symlinks and returns false for a dangling link. Restoration
copies into a uniquely named sibling temporary file, syncs it, validates it,
opens it to apply migrations, validates again, then calls `fs::rename` onto
the destination. On Unix this can replace the dangling symlink itself.

The three existing recovery tests cover successful restoration (schema 25 and
session recall), refusal of a second restore to the same database, retention,
and rejection of an unrelated SQLite database. They do not cover symlinks or
assert preservation of existing destination contents.

## Proposed implementation

Replace only the initial existence guard with a match on
`fs::symlink_metadata(destination)`:

- `Ok(_)`: return the existing `Error::Conflict` and unchanged
  `restore destination … already exists` message for any directory entry.
- `Err(error)` with `error.kind() == io::ErrorKind::NotFound`: continue.
- Any other error: propagate through `Error::Io` (`return Err(error.into())`).

Do not use `.is_ok()` alone: that would silently treat permission and other
metadata failures as absence. Keep source validation before this guard, and
preserve temporary copy, validation, migration, cleanup, and outcome behavior.
`fs` and `io` are already imported; no new dependency is needed.

## Acceptance checks for implementation

1. Add a Unix-gated regression using `std::os::unix::fs::symlink`, a valid
   backup made with `Hub::backup_to`, and a destination link to a nonexistent
   target. Confirm the fixture is a symlink via `symlink_metadata`, call
   `restore_to`, and require `Error::Conflict` / `already exists`. Verify the
   link type and exact `read_link` value are unchanged and the target is still
   absent. This test should fail on the baseline and pass after the fix.
2. Test preservation of an existing regular file with sentinel bytes, an
   existing directory (including a child), and a live symlink (including its
   target bytes). Each must reject a valid backup without modifying the entry.
   Gate symlink cases with `#[cfg(unix)]`, following the repository convention.
3. Keep the existing successful restore, repeat-restore refusal, session recall,
   invalid-backup rejection, and retention tests passing. Check restoration to
   a fresh destination under a missing parent still works. An invalid backup
   must not create destination/parent directories; source validation must still
   take precedence when the destination already exists.
4. Run `CARGO_NET_OFFLINE=true cargo test --offline --locked -p aporic --test recovery`
   and `cargo fmt --all -- --check`. Record actual results, not intended results.
   Review the final changes to ensure product edits touch only the two allowed
   files. Do not broaden to `Store::backup_to` or retention behavior.

## Unresolved risks and boundaries

- The proposed guard fixes entries present at inspection time. There remains
  an existing check-to-rename race: another process can create a destination
  after inspection and Unix rename can replace it. Atomic no-clobber publication
  would require a separate implementation decision; do not claim this guard
  makes concurrent restoration safe.
- `symlink_metadata` does not follow the final component but does resolve parent
  components. Parent symlink policy and unusual path spellings are unchanged.
- Non-NotFound metadata errors should be propagated. Permission tests can be
  unreliable when run with elevated privileges; a symlink-loop parent can be a
  Unix alternative if explicit error-propagation coverage is needed.
- Unix symlink tests do not mechanically verify Windows behavior. Existing
  migration logic remains untouched; the current recovery success test covers
  schema 25, not restoration/migration of every older schema.

## Verification evidence

Baseline command:
`CARGO_NET_OFFLINE=true cargo test --offline --locked -p aporic --test recovery`
passed all 3 tests (0 failures). Dependencies were available offline.

Confirmed the bug with the resulting `target/debug/aporic` binary in a temporary
directory under this workspace's `target/`: set `APORIC_DATABASE` to an isolated
source path, run `backup --to BACKUP`, create `DESTINATION -> MISSING_TARGET`,
then run `restore --from BACKUP --to DESTINATION`. Before: `exists=false`,
`is_symlink=true`. Restore exited 0 with schema versions 25/25 and byte length
872448. After: destination was a regular file, `is_symlink=false`, and the link
target remained absent. The temporary fixture was removed automatically.

No fix or new regression test was implemented in stage 1. Build artifacts remain
under `target/`. Final SHA-256 checks matched both original product files.

Product-file SHA-256 values captured before verification:

```text
32ea6000b7ffe826bb553b6ef69ec40b9870b012ac0e23f054eea7a1b1742e7c  crates/aporic/src/recovery.rs
aa4a0a0a3a9c8d79e953123ee8943c6cbbdb58f7b60e065542a39abd27fdc373  crates/aporic/tests/recovery.rs
```
