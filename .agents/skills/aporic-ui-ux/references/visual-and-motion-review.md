# Visual and motion critique

Use this as a set of questions, not a numeric score. Compare the rendered screen with the stated user task and visual direction. Name the observed problem, its likely user impact, and the smallest useful revision.

## Composition and visual language

- Can a first-time viewer identify the primary action and the current state within a few seconds? Does reading order match the task?
- Are type scale, line length, contrast, spacing, alignment, and grouping consistent enough to make scanning easy? Does the layout survive longer or localized copy?
- Does color convey a consistent meaning, and do non-color cues preserve that meaning? Are imagery and decoration helping comprehension or brand expression?
- Is the visual direction specific to this product and audience? If the screen looks generic, adjust one coherent system of type, color, spacing, and imagery instead of adding unrelated effects.
- At narrow widths, does the hierarchy remain intact without horizontal overflow, clipped controls, or hidden essential information?

## Interaction and motion

- Does each action produce visible feedback, a recoverable error, or a clear next step? Can the user operate the primary flow by keyboard?
- What does each animation explain: cause, destination, state change, or emphasis? Remove movement with no useful role.
- Are duration and distance proportional to the change? Does motion preserve continuity when interrupted or repeated?
- With `prefers-reduced-motion: reduce`, are nonessential interaction animations suppressed while state changes remain perceivable? Follow the [W3C reduced-motion technique](https://www.w3.org/WAI/WCAG22/Techniques/css/C39).

Use actual task observation or user research for usability claims. A visual review can identify plausible issues but cannot establish conversion, comprehension, or preference on its own.
