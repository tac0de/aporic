# Aporic repository workflow

## Kernel completion boundary

After an authorized kernel implementation passes its focused and full checks,
commit it, package and reinstall the local Codex plugin, and migrate the active
store non-destructively. Push, tag, release, publication, and deployment remain
separate human-authorized boundaries.

## Upgrade-safe ordering

- Never raise `.aporic/policy.json` above the schema understood by the hook
  binary loaded in the current Codex task.
- Keep the active policy backward-compatible while editing, testing,
  committing, packaging, and installing a newer plugin.
- A plugin reinstall does not replace hooks already loaded by the current task.
  Activate a newer policy schema only in a fresh task running the new plugin,
  after that binary validates the candidate policy.
- If a policy upgrade is part of a task that will end after installation, leave
  the active policy unchanged and stage the candidate as
  `.aporic/policy.next.json`. A fresh task using the new plugin must validate
  and activate that candidate before other kernel work, then remove the
  candidate in the same commit. Never ask the user to repair or hand-edit
  policy files.
- Preserve the pre-migration event store as a rollback snapshot. Swap in a
  validated migrated store only as the final stateful action of the old-plugin
  task, because its loaded hooks cannot read the newer event schema.

## Review focus

For lifecycle or authorization changes, independently verify that evaluation,
admission, and one-shot consumption all use the same effective policy.
