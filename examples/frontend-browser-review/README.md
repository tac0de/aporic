# Frontend browser review case

This is one local example for Aporic's native `frontend@v2` procedure and its
first module, `frontend.browser_review`. The page is sample UI; it does not
write to Aporic or persist a review. The Aporic procedure records the separate
task, file evidence, and review status.

## Run the page

From the repository root:

```sh
python3 -m http.server 4173 --bind 127.0.0.1 --directory examples/frontend-browser-review
```

Open `http://127.0.0.1:4173/`. No package install or network service is
needed. The page uses local HTML, CSS, and browser state only.

## One review scenario

1. Open **A clearer first step**. Confirm **Mark reviewed** starts disabled.
2. Check the three review items. Mark the proposal reviewed.
3. Select **Reviewed** and confirm that the proposal appears with `1 / 3`
   progress. Select **To review** to see the remaining two.
4. Repeat at 390px. Use Tab and Enter to open a proposal, then Escape to close
   it and verify focus returns to its button.

The [scenario](review-evidence/scenario.md),
[design contract](review-evidence/design.md),
[implementation note](review-evidence/implementation.md), and
[observations](review-evidence/observations.md) are the case artifacts. The PNGs
in `review-evidence/` show observed states at the named viewports. They are
evidence of those rendered states, not proof of general usability.

The live Aporic MCP connection used to start this task predates the
`aporic_workflow_*` tools. The case procedure was therefore exercised with a
fresh local Aporic MCP process and a separate local test database; the
resulting task and step summary is in the observations file. This keeps the
example reproducible without changing the user's project database.
