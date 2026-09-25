# Game systems and production

Make the core loop playable before adding breadth. Specify inputs, simulation state, transitions, feedback, and win/fail or progression rules. Keep tunable values and content data easy to change during playtests. Separate simulation behavior from rendering and audio where it improves determinism and testability; do not introduce an abstraction without a concrete need.

Check input remapping or alternatives, readable state feedback, pause and resume, save/load, accessibility options, and progression recovery where the game needs them. Test the actual target platform, controller or keyboard path, display size, and performance budget. Audio, animation, and effects should communicate action timing and consequence rather than conceal missing feedback.

For assets and tools, establish naming, import settings, ownership, source provenance, and reproducible builds before the content set grows. Watch memory, loading, frame pacing, and content iteration time. Use the project's engine profiler and build pipeline for claims about runtime performance.

References: [Unity Game Designer Playbook](https://unity.com/resources/game-designer-playbook) for prototyping topics and [Xbox Accessibility Guidelines](https://learn.microsoft.com/en-us/xbox/accessibility/guidelines) for game-specific accessibility questions. Use the applicable platform requirements for the actual release target.
