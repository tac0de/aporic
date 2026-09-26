# Paired pilot scoring rubric

Freeze this rubric before inspecting either arm's result. Both arms receive
the same feature request and start at commit `5dc8fbd` in isolated checkouts.
An independent reviewer sees anonymized artifacts as X and Y.

## Acceptance gates

Score each item `pass`, `fail`, or `unverified`, with a browser command,
screenshot, or code location. `unverified` does not count as a pass.

1. Local static page loads without external assets or packages and shows three
   proposals with progress `0/3`.
2. Each proposal opens a dialog with an accessible title and three named
   checkboxes.
3. The completion action is disabled before all three checkboxes are checked
   and enabled afterward.
4. Completing the first review changes its visible status and progress to
   `1/3`.
5. All, To review, and Reviewed filters show the expected proposals after that
   change.
6. At 1440×900 and 390×844, content fits the viewport without horizontal
   overflow or clipped controls.
7. Keyboard Enter opens the proposal dialog; Escape closes it and restores
   focus to the opening control.
8. With `prefers-reduced-motion: reduce`, animated transitions are disabled.

`accepted_task = true` only if all eight gates pass. Record console errors and
failed requests separately; any error affecting the task is a gate failure.

## Quality scores

For each dimension, give 0, 1, or 2 and a short observable reason:

| Dimension | 0 | 1 | 2 |
| --- | --- | --- | --- |
| Visual hierarchy | Main action/status hard to locate | Understandable with some competing emphasis | Main action, status, and proposal content are clear at both sizes |
| Responsive layout | Controls overlap or clip | Usable with awkward spacing | Controls and dialog remain comfortably usable at both sizes |
| Keyboard/accessibility | Missing labels or focus behavior beyond acceptance gates | Basic operation works with minor semantic issues | Labels, focus, and status updates are coherent end to end |
| Evidence quality | No reproducible verification | Some checks with gaps | Reproducible commands and observed results for both viewports and interaction |

Report the four scores individually, their sum out of 8, and exact defects.
Do not infer which arm produced an artifact from style or file naming.

## Effort and limitations

Collect elapsed wall time, model token use if available, tool-call count if
available, and corrective iterations from host logs rather than self-estimates.
Mark missing values unavailable. This single task can test whether the protocol
works and reveal concrete differences; it cannot estimate a reliable average
Aporic effect. Do not select a winning workflow from this pilot alone.
