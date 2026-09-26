# Independent blinded frontend review

The reviewer inspected only the frozen pilot rubric and anonymous X/Y HTML, CSS, and JavaScript. No provenance mapping, original checkout, README, Aporic record, author report, or treatment label was inspected. Both artifacts were left unchanged. Findings below compare artifacts, not workflows.

## Result

| Artifact | Gates passed | Gates failed | Unverified | accepted_task | Quality / 8 |
| --- | ---: | ---: | ---: | --- | ---: |
| X | 8 | 0 | 0 | true | 7 |
| Y | 8 | 0 | 0 | true | 7 |

The acceptance result is tied. X has a confirmed completed-dialog display defect outside the eight gates. Both artifacts have proposal button accessible names that do not contain their full visible action labels. The quality total is tied; this single task cannot establish an average treatment effect or select a winning workflow.

## Reproduction and common method

Start the two static servers from the unchanged anonymous artifacts:

```sh
python3 -m http.server 8781 --bind 127.0.0.1 --directory /private/tmp/aporic-ab-blind/X
python3 -m http.server 8782 --bind 127.0.0.1 --directory /private/tmp/aporic-ab-blind/Y
```

Then, from `/private/tmp/aporic-ab-blind`, run:

```sh
playwright-cli -s=blind-x open http://127.0.0.1:8781
playwright-cli -s=blind-y open http://127.0.0.1:8782
playwright-cli -s=blind-x run-code --filename=/private/tmp/aporic-ab-blind/audit.js
playwright-cli -s=blind-y run-code --filename=/private/tmp/aporic-ab-blind/audit.js
```

The same `audit.js` runs at 1440×900 and 390×844. It reloads to fresh state at each size; opens all three proposals by Enter; records the dialog accessibility snapshot; checks the completion button at 0, 1, 2, and 3 checked boxes and after unchecking one; closes by Escape and verifies focus; completes proposal one; checks all filters; reopens the completed proposal; captures page/dialog screenshots; and emulates reduced motion. Network and console listeners cover both fresh reloads and interactions. Browser operations use Playwright CLI, not the Codex in-app browser.

Evidence files in this directory:

- `audit.js`: exact shared browser script.
- `X-audit.log`, `Y-audit.log`: final Playwright command output, including returned observations and script.
- `X-audit.json`, `Y-audit.json`: extracted structured observations.
- For each artifact, `{X,Y}-{1440,390}-{initial,dialog,completed}.png`: screenshots. Initial/completed images are full-page captures; dialog captures use the exact viewport. Initial and dialog screenshots for both sizes and both artifacts were visually inspected.

The first harness execution used console.log, which the CLI did not return; the harness was changed to return structured data and applied equally to both artifacts. A subsequent focus check read state before the asynchronous native dialog close event finished, producing a transient misleading X result. The final shared harness waits two animation frames after close, then records stable focus. This also opened all three correct dialogs. These were reviewer harness corrections, not artifact repairs.

## Acceptance gates

All gates below were exercised at both specified viewport sizes unless the gate is source-only in part.

| # | Gate | X | Y | Direct evidence |
| --- | --- | --- | --- | --- |
| 1 | Static local page, three proposals, initial 0/3, no external dependencies | pass | pass | Both JSON files: `views[].initial.titles` has three titles; initial progressbar value is `0`; ARIA snapshots and initial screenshots show `0/3`. Requests consist only of each local root, `styles.css`, and `app.js`. HTML lines 8–10 in X and 7–10 in Y use an inline data favicon and local assets. Source has no package/import requirement. |
| 2 | Every proposal has an accessible dialog title and three named checkboxes | pass | pass | `views[].dialogs[].snapshot` includes each proposal title as the dialog name and exactly three named checkbox roles. X titles: A quieter start; Signals, not noise; Room to reflect. Y titles: Quiet confidence; Open by nature; Made of moments. X `index.html:50`, `app.js:41`; Y `index.html:62`, `app.js:44`. |
| 3 | Pending-review completion disabled until all three are checked, then enabled | pass | pass | Every dialog's `disabled` sequence is `[true,true,true,false]`, and `disabledAfterUncheck` is true. This gate is assessed for a pending review; reopening a completed review is discussed separately. X `app.js:66`; Y `app.js:37`. |
| 4 | Completing first proposal changes status and progress to 1/3 | pass | pass | `views[].completed.progress` is `1` and firstStatus is `Reviewed`; completed screenshots show the result. X `app.js:80`; Y `app.js:63`. |
| 5 | All, To review, Reviewed filters return expected proposals | pass | pass | `views[].filters`: All = all three; To review = second and third; Reviewed = first. Filter counts display 03/02/01. X `app.js:16,87`; Y `app.js:17,69`. |
| 6 | Both viewport sizes have no horizontal overflow or clipped controls | pass | pass | At desktop, document scrollWidth/clientWidth = 1440/1440; at mobile = 390/390. All recorded control horizontal-overflow lists are empty. Every open dialog fits inside the viewport. Visual review found readable controls, intact labels, and no overlap. Normal vertical page scrolling is present in both artifacts and is not treated as clipping. |
| 7 | Enter opens; Escape closes and restores opener focus | pass | pass | Final `views[].dialogs[].restoredFocus` is true for all six opens per artifact. Each is opened by `opener.focus()` then keyboard Enter; Escape closes; native close handling settles before measurement. X `app.js:73`; Y `app.js:59`. |
| 8 | Reduced-motion animated transitions disabled | pass | pass | Emulated reduced-motion query matches at both sizes. Progress transition duration computes to `1e-05s` (0.01 ms); animation duration also computes to 0.01 ms; scroll behavior is `auto`. Both `styles.css:1` end with the global reduced-motion override. This is an effectively instantaneous transition, accepted as disabling perceptible animation; it is not a literal zero-duration declaration. |

### Geometry samples

| Artifact | Size | First dialog bounds (left, top, right, bottom), CSS px |
| --- | --- | --- |
| X | 1440×900 | 435, 169.33, 1005, 730.66 |
| X | 390×844 | 19, 132.11, 371, 711.88 |
| Y | 1440×900 | 475, 143.03, 965, 756.95 |
| Y | 390×844 | 19, 124.19, 371, 719.81 |

### Console and network

Both artifacts: zero observed console errors, zero uncaught page errors, zero failed requests during the final shared audit. Both made exactly the expected local page/CSS/JS requests per reload. No external asset requests were observed.

## Quality scores

| Dimension | X | Observable reason for X | Y | Observable reason for Y |
| --- | ---: | --- | ---: | --- |
| Visual hierarchy | 2 | Progress, filter state, proposal titles, statuses and rust-colored review actions are distinct at both widths. Cards form a consistent scan order. | 2 | Progress panel, selected filter, proposal titles/statuses and action row are clear at both widths. Decorative panels increase page length but do not obscure card content. |
| Responsive layout | 2 | Three columns become one; readable spacing and dialog controls fit comfortably. Mobile dialog has an intact stacked action area. | 2 | Three columns become one; full-width cards and checkbox rows remain comfortable. Dialog and button remain within the mobile viewport. |
| Keyboard/accessibility | 1 | Named dialog/checks, focus indicators, Escape restoration, pressed filter semantics and live progress work. Visible/accessible action-name mismatch and the completed-dialog defect below prevent a fully coherent end-to-end score. | 1 | Native checkbox/dialog operation, Escape and completion focus restoration, pressed filters and live progress work. Proposal visible/accessible name mismatch and stale reviewed-action wording are minor semantic issues. |
| Evidence quality | 2 | Shared reproducible CLI script, structured observations, both viewport sizes, all three dialogs, filters, motion and screenshots. | 2 | Identical evidence coverage and scrutiny. |
| **Sum** | **7/8** | | **7/8** | |

Evidence-quality scores refer to the independent evidence produced by this review. Author-generated verification and its quality are unavailable within the permitted anonymous source-only boundary; these scores must not be presented as an assessment of author diligence or treatment-specific evidence production.

## Exact defects and observations

### X: completed review still shows disabled completion action

After completing proposal one, selecting All, and reopening its review, all three checkboxes are checked and disabled. `#mark-reviewed` has a `hidden` attribute but is still visible and remains in the accessibility snapshot as a disabled “Mark reviewed” button. This occurs at both viewport sizes (`views[].reviewedDialog.markVisible=true`, `markHiddenAttribute=""`).

`X/app.js:60` sets `markButton.hidden` for a completed review. The `.submit-button{display:flex}` declaration in `X/styles.css:1` overrides the user-agent hidden presentation, and there is no matching `.submit-button[hidden]{display:none}` rule. The dialog also retains “Before you mark this reviewed” and “Confirm all three points below” without a reviewed-state explanation. This is confusing when revisiting a completed review, but does not break first-time completion, filters, or the eight specified pending-review gates.

X recreates the proposal cards on completion (`app.js:18,85`), so the settled close handler focuses the active filter instead of the removed opener (`app.js:73–78`). The final audit observes `All 03` focused. This is an intentional fallback in source and does not fail the Escape-restoration gate, which passes for unchanged openers. Y preserves and refocuses the proposal button after completion.

### Both: accessible action names omit the full visible label

X's pending button visibly says “Review proposal” but has an accessible name such as “Review A quieter start” (`X/app.js:26–27`). Its completed “View review” name is contained in “View review for …”, which is coherent.

Y's button visibly says “Explore proposal” but its accessible name is “Review Quiet confidence” (and equivalents; `Y/index.html:45,49,53`). The mismatch can impede speech input based on the visible label. On completion, Y retains the same “Review …” accessible action name even though the dialog now describes an already reviewed proposal. Its dialog correctly hides the completed action and displays “You have already reviewed this proposal.”

Neither mismatch makes the proposal control unnamed or prevents ordinary keyboard use, so these are quality deductions rather than gate failures.

### Non-defect browser observation

Tab sequences in both native dialogs briefly report BODY as activeElement when focus cycles through browser chrome, then return to the close control. No background page control was focused by the observed sequence. This was not treated as an application focus-trap defect.

## Effort, fairness, and limits

The same script, viewport pair, proposal count, interaction sequence, screenshot categories, source review scope, and follow-up completed-dialog inspection were applied to X and Y. Runs used separate local ports and browser sessions. No corrective change was made to either artifact.

Author elapsed wall time: unavailable. Author model token use: unavailable. Author tool-call count: unavailable. Author corrective iterations: unavailable. No host production logs were supplied inside the blinded evidence boundary, and these values are not estimated from source, visual style, or the reviewer's own test runs.

Review coverage is Chromium through the installed Playwright CLI, the two requested dimensions, DOM accessibility snapshots, keyboard actions and visual inspection. It does not include a screen reader or additional browser engines. The frozen rubric does not require those. The near-zero reduced-motion interpretation is stated explicitly above so a stricter literal-zero interpretation can be applied consistently to both artifacts if the experiment owner chooses it.
