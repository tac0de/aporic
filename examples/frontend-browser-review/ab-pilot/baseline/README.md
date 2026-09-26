# Review board pilot

A self-contained, local frontend example for reviewing three creative proposals.

Open `index.html` directly in a browser, or serve this directory with `python3 -m http.server 8000` and visit `http://localhost:8000`.

Use the filters to show all proposals, proposals to review, or reviewed proposals. Open a proposal and check Concept, Execution, and Impact to enable **Mark reviewed**. Review progress is held in memory for the current page session and resets on reload. The dialog supports Escape to close and returns focus to its opening button.

No packages, external assets, or network access are required.
