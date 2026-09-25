# Aporic clean-room collaboration

Aporic is developed from its current product record and observed evidence.
Historical Git commits and tags are context, not current design authority.

## Current boundaries

- Use Rust for product logic and Bash only as a thin integration layer unless
  the user explicitly changes that direction.
- Until Aporic is an operating service, integrations must be advisory and
  fail-open: they may observe or record work but must not deny tools, require
  grants, prompt for Aporic approval, or otherwise narrow host permissions.
- Preserve useful history through append-only records and deterministic replay.
  Do not wire authorization or policy experiments into the active development
  workflow while integrations remain advisory.
- Keep model roles, metaphors, and host integrations outside any future
  deterministic authorization kernel.
- Do not reactivate, migrate, publish, or deploy material recovered from Git
  history without a separate explicit instruction.

## Collaboration

- Decisions are provisional directions supported by their recorded context,
  not timeless answers. Preserve meaningful revisions and counterarguments.
- Before material changes, state the intended scope and acceptance checks.
- Preserve unrelated user work and keep changes reversible.
- Unless the human explicitly requests otherwise or an external block prevents
  it, completed implementation work includes mechanical verification, a commit,
  fast-forward integration into `main`, pushing `origin/main`, and deletion of
  every non-main local and remote branch.
