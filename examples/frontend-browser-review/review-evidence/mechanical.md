# Mechanical checks

Executed from the repository root on 2026-09-26:

- `node --check examples/frontend-browser-review/app.js` — passed.
- `git diff --check` — passed before staging.
- Clean Playwright session at 320px — `document.documentElement.scrollWidth`
  and `window.innerWidth` both returned `320`; console reported zero errors and
  zero warnings.
- Playwright reduced-motion emulation — the card action's computed transition
  duration returned `0s`.

The JavaScript syntax check does not validate behavior; the interaction and
keyboard checks are documented in `observations.md`.
