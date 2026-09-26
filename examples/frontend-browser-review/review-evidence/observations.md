# Browser observations

Reviewed locally with `playwright-cli` against `http://127.0.0.1:4173/` on
2026-09-26. These are observations of this example and browser session.

| Check | Observed result | Evidence |
| --- | --- | --- |
| Desktop 1440 × 900 initial | Three proposals visible; progress `0 / 3`; primary hierarchy and filters rendered. | `desktop-initial.png` full-page capture, accessibility snapshot |
| Review interaction | Completion button began disabled, enabled after three checks, and changed the first proposal to Reviewed with progress `1 / 3`. | Playwright click/check/snapshot sequence; `desktop-reviewed.png` |
| Mobile 390 × 844 | One-column cards and dialog fitted; document `scrollWidth` and viewport width were both 390px; dialog width was 352px. | `mobile-reviewed.png`, `mobile-dialog.png`, Playwright evaluation |
| Narrow 320 × 740 | Three cards remained in one column with no clipped text; `scrollWidth` and viewport width were both 320px. | `mobile-320.png`, Playwright evaluation |
| Keyboard | Tab then Enter opened the dialog with focus on `flow`; Escape closed it and returned focus to the proposal action. | Playwright keyboard/evaluation sequence |
| Accessibility structure | Snapshot exposed named filter group, proposal buttons, a titled dialog, three named checkboxes, and a status region. | Playwright accessibility snapshot |
| Reduced motion | With `prefers-reduced-motion: reduce`, the card action computed transition duration was `0s`. | Playwright media emulation and computed-style evaluation |

The first browser load reported a missing favicon; adding a local data favicon
removed the request on reload. A clean browser session reported zero console
errors and zero warnings. Automated checks and screenshots
do not establish whether real reviewers understand the proposals.

## Aporic procedure replay

A fresh `target/release/aporic mcp serve --stdio` process used a separate
temporary SQLite database. Its task `01a0dde6-733a-77c9-baf2-20b88e08ff23`
selected `frontend@v2` and listed exactly these module steps:
`browser_scenarios`, `rendered_browser_review`, and
`responsive_accessibility_review`. Each was recorded as completed with direct
workspace-file evidence from this example. The general steps were completed or
skipped with a concrete reason as allowed. The task's file-hash criterion was
proved directly and the workflow reached `completed` with an empty
`missing_for_next_stage` list.

The replay database was created at
`/var/folders/2j/vcfbswms6fq5t7rxtn4mwlg40000gn/T/aporic-frontend-case-tkrm2w3n/case.sqlite3`.
It is a local test artifact, outside this Git repository and separate from the
user's Aporic project database. The criterion proves the example's `index.html`
bytes at replay time; it does not prove that reviewers find the interface
useful.
