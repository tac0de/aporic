# Playwright CLI browser review

Use the installed `playwright-cli` from the host. Check `playwright-cli --help` and `playwright-cli --help <command>` for the current syntax. The CLI is separate from the `playwright test` runner; use the runner for repeatable assertions when the project has tests.

1. Start the project's local server with its normal development command. Open its local URL with `playwright-cli open <url>`.
2. Run `playwright-cli snapshot` to inspect page structure and obtain element references. Navigate the primary flow with `click`, `fill`, `press`, and other interaction commands. Use `playwright-cli screenshot --filename=<path>` for visual inspection.
3. Use `playwright-cli resize <width> <height>` for a desktop and a narrow viewport. Revisit the same core task in both. Check scrolling, overflow, focus, navigation, content wrapping, and touch-sized controls.
4. Capture meaningful intermediate states. Use `playwright-cli tracing-start` and `tracing-stop` for a failure or complex motion. Inspect console errors and network failures. Close the browser session when finished.
5. For repeatable visual checks, use Playwright Test screenshot assertions in the target project. Stabilize data, fonts, viewport, and animation state first. Review changed baselines visually; a baseline update is not a quality improvement by itself.

For motion, observe trigger, middle, and resting state. Check whether rapid repeated input, interrupted navigation, and a reduced-motion preference leave the interface understandable. Use the project's browser-context configuration or an explicit Playwright Test case to emulate reduced motion; do not claim it was checked from a static screenshot.

Official references: [Playwright CLI commands](https://playwright.dev/agent-cli/capabilities), [screenshots and snapshots](https://playwright.dev/agent-cli/commands/screenshots-pdf), [tracing](https://playwright.dev/agent-cli/commands/tracing), and [visual comparisons](https://playwright.dev/docs/test-snapshots).
