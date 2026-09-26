# Browser scenario

## Target and outcome

A reviewer needs to inspect one proposal and record that a basic experience
check was performed. The observable outcome is a reviewed status, progress
`1 / 3`, and the proposal visible under the Reviewed filter.

## Browser checks

- At 1440 × 900: inspect the page hierarchy, open the first proposal, verify
  the completion button is disabled until all three checklist items are set,
  complete the review, and filter to Reviewed.
- At 390 × 844: inspect the filtered page and the dialog. Confirm there is no
  horizontal overflow.
- With keyboard input: Tab to the proposal action, Enter to open, and Escape
  to close. Confirm focus starts on the first checklist item and returns to
  the proposal action.
- Check the accessibility snapshot for named regions, buttons, checkboxes,
  dialog title, and live status. Check console and failed requests.

The proposal data and checked state are deliberately local to the page. A
checked box records a sample UI action, not a verified product assessment.
