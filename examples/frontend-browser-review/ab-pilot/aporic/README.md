# Review board pilot

A self-contained local review board example. Open `index.html` directly in a browser; it uses no build step, packages, assets, or network requests.

## User scenario

A reviewer sees three proposals and progress at 0/3. Each card opens a titled modal with three proposal-specific checks. The reviewer completes all checks before **Mark reviewed** becomes available. Completion updates the card and progress, and the **All**, **To review**, and **Reviewed** filters show the relevant proposals.

## Design and verification contract

- Desktop: 1440 × 900; mobile: 390 × 844. Check horizontal overflow and visible controls.
- Keyboard: focus a proposal button and press Enter to open; Escape closes the modal and returns focus. Native `<dialog>` supplies modal focus handling.
- Accessibility: semantic headings, named controls, grouped checks, button state, and accessible progress value.
- Motion: transitions are effectively removed for `prefers-reduced-motion: reduce`.
- State lives in memory; reloading starts a fresh local review session.

The layout uses an editorial heading, compact proposal cards, and a visible progress panel. Filtering only changes which cards are shown; it preserves the underlying reviewed state.

## Observed verification (2026-09-26)

- `node --check app.js` passed.
- Playwright CLI rendered desktop at 1440 × 900 and mobile at 390 × 844; document width equaled viewport width in both. The mobile modal measured 352 px within a 390 px viewport.
- Enter opened a titled modal; Escape closed it and returned focus to its proposal button. Three named checkboxes were exposed. **Mark reviewed** stayed disabled after zero and two checks, then enabled after all three.
- Completing a review changed progress from 0/3 to 1/3. All showed three cards, To review two, and Reviewed one.
- The mobile page and modal were inspected visually. Reduced-motion emulation gave a computed transition duration of 0.00001 s.
