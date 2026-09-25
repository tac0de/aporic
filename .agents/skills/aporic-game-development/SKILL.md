---
name: aporic-game-development
description: Design, implement, and evaluate playable game prototypes and levels for Aporic product experiments. Use for game mechanics, level design, playtesting, and game production work, not ordinary web UI or backend changes.
---

# Aporic game development practice

Use this skill when an Aporic product task involves a playable game or level. Work in the chosen project's engine and native tools; do not select an engine merely because this skill exists. Aporic can preserve hypotheses, evidence, and decisions but cannot judge whether a game is fun by itself. If the target is browser based, use Playwright CLI for browser inspection; never use the Codex in-app browser.

## Establish the playable question

Identify the intended player, platform, input method, session length, and core action loop. State the experience the prototype should test, the player behavior that would support it, and the main uncertainty. Build the smallest playable slice that exposes that uncertainty. Keep rules, content, and presentation separable enough to revise one without rebuilding the others.

For space, encounter, puzzle, or mission design, read [level-design.md](references/level-design.md). For mechanics, architecture, assets, and production constraints, read [game-systems.md](references/game-systems.md). For observation and evaluation, read [playtesting.md](references/playtesting.md). Use the [UI/UX skill](../aporic-ui-ux/SKILL.md) for menus and interface work and the [backend skill](../aporic-backend/SKILL.md) when the game needs services or persistent data.

## Build, play, revise

Prototype mechanics and level layout with inexpensive art or blockout geometry. Play from the actual player camera and input, including start, failure, recovery, and completion. Check the target platform's frame pacing and controls when possible. Record where a player hesitates, gets lost, exploits a shortcut, repeats an action, or stops understanding the goal; revise the rule, cue, or layout that caused it. Do not infer playability from an editor view or a still screenshot.

Use automated tests for deterministic rules, serialization, progression gates, and reproducible regressions. Use live playtesting for comprehension, pacing, challenge, and enjoyment. Capture a short build identifier, level version, test conditions, and observations so revisions can be compared. A heatmap, completion time, or automated pass is evidence about a narrow behavior, not proof of good design.

For substantive work, tie the hypothesis and acceptance checks to the Aporic task or product cell when one exists. Store only durable decisions and representative evidence; label designer interpretation separately from observed player behavior. Report the playable change, platforms and input methods exercised, playtest findings, remaining uncertainty, and build or performance limits. Follow the repository's verification and Git integration convention for completed implementation.
