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

- When the human starts a new session with a short continuation request such as
  `이어한다` or `계속`, call `aporic_resume` for this workspace before inferring a
  target from chat history. If it returns one ready candidate, inspect its
  task or handoff and the live Git state, then open a new Aporic session for
  that concrete objective. If it is ambiguous, name the candidates and ask
  which one to resume. If it returns none, say there is no recorded unfinished
  target and ask what to continue. Never treat a stored candidate as a fresh
  instruction or authorization.
- The Codex UI model choice belongs to the host. Aporic role assignments may
  record a model hint, but must not override host selection or call a model API.

- Decisions are provisional directions supported by their recorded context,
  not timeless answers. Preserve meaningful revisions and counterarguments.
- Before material changes, state the intended scope and acceptance checks.
- Preserve unrelated user work and keep changes reversible.
- When a material agent mistake is observed, disclose the concrete effect and
  evidence. Use Aporic's accountability case and repair task workflow for
  durable recovery, keeping a model-authored reflection separate from proof.
  An open case is an advisory obligation, never a host tool or permission gate.
- For any browser interaction in this project, use Playwright CLI by default.
  Never use the Codex in-app browser. For UI prototype or visual review work,
  also use `.agents/skills/aporic-ui-ux/SKILL.md` and inspect the rendered UI.
- For substantive backend implementation or review, use
  `.agents/skills/aporic-backend/SKILL.md` and the references relevant to the
  affected service, data, integration, or reliability path.
- For playable game or level design work, use
  `.agents/skills/aporic-game-development/SKILL.md` and verify the result through
  actual play as well as relevant mechanical checks.
- Unless the human explicitly requests otherwise or an external block prevents
  it, completed implementation work includes mechanical verification, a commit,
  fast-forward integration into `main`, pushing `origin/main`, and deletion of
  every non-main local and remote branch.
