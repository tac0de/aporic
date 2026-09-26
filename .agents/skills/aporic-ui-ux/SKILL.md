---
name: aporic-ui-ux
description: Design, build, and critique browser UI prototypes for Aporic product experiments using Playwright CLI and evidence-backed visual and motion review. Use for UI/UX implementation or review, not for backend-only changes.
---

# Aporic UI/UX practice

Use this skill when an Aporic product task needs an interface, interaction, or motion design. Keep implementation in the target project's native stack. Aporic's product cell and evidence APIs are advisory records; this skill does not grant execution, approval, or deployment authority.

## Define the design problem

Before coding, identify the user, task, context of use, current friction, and one observable success measure. When a product cell exists, use its problem, hypothesis, and measures. Choose a visual direction that fits the product and audience; state the intended hierarchy, typography, color, density, and motion purpose. Avoid treating a fashionable style as a universal answer. If the task has no working UI yet, build a narrow, usable vertical slice before attempting broad polish.

When the visual direction is unclear, inspect a few relevant products or design references, describe what works for their context, and compare two distinct directions against the task. Adapt principles rather than copying a screen. Keep a small, coherent set of reusable type, color, spacing, and motion choices as the prototype grows.

## Build and inspect in a browser

For a full design-to-implementation delivery, follow
[design-delivery.md](../../../docs/design-delivery.md): keep a brief, reference
notes, concept choices, tokens, components and states, responsive rules, asset
inventory, implementation artifact, and browser review in a versioned design
package. Run `aporic design validate` before presenting completion. The
validator checks local integrity and completeness of declared categories;
visually inspect the rendered result and report subjective judgment separately.

Use `playwright-cli` for all browser interaction in this project; never use the Codex in-app browser. Read [browser-review.md](references/browser-review.md) for the command loop. Run the application locally, open the actual page, inspect accessibility snapshots and screenshots, exercise the main task, and capture at least desktop and narrow viewports. Inspect empty, loading, error, and completed states when those states exist. Look at the rendered result before deciding a design is finished; source code alone is insufficient evidence of visual quality.

Use [visual-and-motion-review.md](references/visual-and-motion-review.md) to critique the result. Prioritize clear hierarchy, readable content, useful feedback, coherent spacing and alignment, restrained color, and consistency with the intended visual direction. Revise the interface after each consequential observation. For motion, inspect the transition in time, not only the static end frame. Motion should clarify cause, state, or spatial relationship. Check keyboard operation and reduced-motion behavior for interactive animation.

## Record and report what was observed

For a substantive Aporic task, use the existing Aporic session and product cell when available. Save a small set of representative screenshots or traces with the prototype. Register a workspace file through `aporic_evidence_add` when it is genuinely in the session workspace; a command result or external source remains reported evidence. Keep critique, aesthetic preference, and predicted usability clearly labelled as model assessment until user research or another direct observation supports a stronger claim. Record durable design decisions and unresolved issues through `aporic_record`, without dumping the raw browser session.

Finish with the implemented change, the actual browser states and viewport sizes reviewed, motion and accessibility findings, and remaining uncertainty. Screenshot similarity or passing automation does not prove that users understand the interface.
