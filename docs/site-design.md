# Aporic introduction page design

This is the bright editorial direction selected for local review. `index.html` is
the page, `styles.css` contains the design tokens and responsive rules, and
`site.js` handles only the mobile menu. The page uses no external font, script,
analytics, or network asset.

## Design intent

- Audience: an individual developer using an AI coding agent across sessions.
- Primary question: what does Aporic preserve, and what does it actually do today?
- Hierarchy: product promise, evidence flow, three functions, operating boundary,
  current development status, repository link.
- Visual language: editorial typography and archival print texture. The
  generated engraving is decorative; all meaningful text and controls remain
  selectable HTML.

## Tokens and components

CSS custom properties in `styles.css` define paper, ink, cobalt, lime, borders,
spacing, page padding, type stacks, and transition easing. Components are the
header, buttons, four evidence cards, three function bands, example handoff,
principles, status callout, and footer. Breakpoints are 1120, 830, and 600 CSS
pixels; the layout also fits 320 pixels without horizontal scrolling.

The paper grain and favicon are local SVG assets. `hero-engraving.jpg` is a
generated decorative asset. Its generation prompt requested a matte near-black
archival print plate with a silver engraved half-sphere, sparse orbital lines,
fine stipple, no text, and no UI. It is used only behind real HTML cards.

## Interaction and accessibility

The site has a skip link, semantic landmarks and headings, keyboard focus
outlines, a mobile menu with `aria-expanded` and Escape support, and a
`prefers-reduced-motion` rule. The only intentional animation is short hover
feedback. Descriptive copy distinguishes the available Codex adapter from
other adapters still under exploration.

## Reference principles

- [New Layer Capital on Awwwards](https://www.awwwards.com/sites/new-layer-capital):
  bold composition with restrained interface chrome.
- [Fine Thought on Awwwards](https://www.awwwards.com/sites/fine-thought):
  typography as the main visual structure.

The page adapts those principles without copying either site.
