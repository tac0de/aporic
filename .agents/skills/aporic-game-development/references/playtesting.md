# Playtesting and evidence

Set one learning goal for each session: for example whether a player notices the exit, understands a new verb, recovers after failure, or chooses between routes. Record build and level version, player familiarity, input device, and any instructions given. Avoid explaining the intended solution during the attempt.

Distinguish observation from interpretation. “Player turned back three times at the junction” is an observation; “the junction is too complex” is a hypothesis. Use a short follow-up question to test the interpretation. Preserve contradictory findings and avoid turning one player's preference into a universal rule.

For browser games, use Playwright CLI to inspect loading, controls, layout, console errors, and reproducible flows; browser automation cannot replace human playtesting. For native games, use the engine's play mode or a built executable and capture only the evidence needed to revisit a finding. Recheck the whole loop after a level or mechanic revision because a local fix can change pacing elsewhere.

Summarize what changed, what the session actually showed, and what remains unknown. Progression and telemetry metrics can locate friction; they do not by themselves establish enjoyment or accessibility.
