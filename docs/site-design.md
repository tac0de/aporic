# Aporic introduction page design

This is the bright editorial direction selected for local review. `index.html` is
the page, `styles.css` contains the design tokens and responsive rules, and
`site.js` handles the mobile menu, scroll progress, section reveal, and pointer
response. The page uses no external font, script, analytics, or network asset.

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
principles, status callout, and footer. The base layout is a 320px-capable
single column; `min-width` breakpoints at 601, 831, and 1121 CSS pixels add
tablet and desktop compositions.

The paper grain and favicon are local SVG assets. `hero-engraving.jpg` is a
generated decorative asset. Its generation prompt requested a matte near-black
archival print plate with a silver engraved half-sphere, sparse orbital lines,
fine stipple, no text, and no UI. It is used only behind real HTML cards.

## Interaction and accessibility

The site has a skip link, semantic landmarks and headings, keyboard focus
outlines, a mobile menu with `aria-expanded` and Escape support, and a
`prefers-reduced-motion` rule. Motion now establishes reading order: title lines
enter in sequence, the evidence cards assemble, an orbital field and scan line
suggest continuity, a cobalt ticker carries the vocabulary, and sections reveal
as they enter the viewport. A top line shows reading progress. Fine-pointer
devices get a small hero tilt; touch devices do not. Motion uses transforms and
opacity; continuous effects pause outside the viewport. The page remains
readable when JavaScript or motion is disabled.
Descriptive copy distinguishes the available Codex adapter from
other adapters still under exploration.

## Local review

Playwright CLI visual inspection covered 320, 390, 601, 831, 1121, and 1440
CSS-pixel widths. The 831-pixel hero initially made the headline too narrow;
the two-column hero now begins at 1121 pixels. At 320 and 390 pixels the page
had no horizontal overflow. The mobile menu opened and closed with Escape,
the progress line advanced on scroll, and all 11 revealed content blocks became
visible during a full-page traversal. With reduced motion enabled, all four
evidence cards and all content blocks remained visible and no animation kept
running. The browser console reported no errors. Screenshots and a short
mobile motion recording are in `../design-review/`. These observations do not
establish user comprehension or design preference.

## Reference principles

- [New Layer Capital on Awwwards](https://www.awwwards.com/sites/new-layer-capital):
  bold composition with restrained interface chrome.
- [Fine Thought on Awwwards](https://www.awwwards.com/sites/fine-thought):
  typography as the main visual structure.

The page adapts those principles without copying either site.
