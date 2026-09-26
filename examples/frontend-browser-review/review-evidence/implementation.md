# Implemented slice

- `index.html` defines the semantic page, filter controls, proposal container,
  review dialog, and checklist.
- `styles.css` provides desktop and narrow layouts, visible keyboard focus,
  and reduced-motion handling.
- `app.js` renders three fixed proposals and keeps reviewed state in memory.
  It enables completion only after all three checks, updates progress and
  filters, and manages dialog focus.

The example makes no Aporic API calls, has no account or data store, and
contains no external assets. A reload resets sample review state by design.
