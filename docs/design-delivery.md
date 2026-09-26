# Design delivery in v0.21

Aporic v0.21 connects reference research, design decisions, implementation,
and browser review through a versioned **repository-owned design package**.
The host agent does the creative and implementation work. Aporic checks that
the declared local files exist and match their SHA-256 digests. It does not
produce a Figma file, judge visual quality, verify a human decision, or deploy.

## Workflow

1. Write a brief naming the target user, task, context, friction, desired
   experience, constraints, and observable success measure. Link the active
   Aporic task ID if one exists; the validator treats it as a declaration.
2. Inspect a small number of relevant references. Record the URL, observation
   date, useful principle, and intended adaptation. Keep screenshots or notes
   only where permitted, and distinguish observation from taste.
3. Produce at least two distinct concepts when direction is uncertain. Record
   their hierarchy, typography, color, density, imagery, interaction, and
   tradeoffs. Keep the chosen concept label and its rationale. Mark a user
   selection as `reported_user_choice` only after it actually occurs; otherwise
   use `model_proposal`.
4. Deliver a design specification with named tokens, component states,
   responsive behavior, content hierarchy, asset inventory, and interaction or
   motion notes. Include actual assets and source files when relevant. For a
   Figma-like handoff, provide dimensions, spacing, typography, colors, and
   state behavior precise enough for implementation without guessing.
5. Implement in the project's native stack. Review the rendered interface
   with Playwright CLI at desktop and narrow widths. Record viewports, main
   interactions, screenshots, accessibility and motion findings, and changes
   made after review. The review is an observation artifact, not user testing.
6. Compute each file's SHA-256, add it to the manifest, and run validation.
   Register the manifest through `aporic_evidence_add` as workspace-file
   evidence when the current Aporic session genuinely owns that workspace.
   Preserve the manifest and files in Git. Ask for human design review when
   the decision requires it; publication follows the host's authorization.

## Manifest v1

Place a JSON manifest in the workspace and use relative file paths for every
artifact. This is a compact example; all nine artifact kinds are required for
`delivery_artifacts_complete` to be true. A concept file can itself point to
images and asset files, while `asset_manifest` inventories them. Every
referenced asset that matters to the handoff should have its own entry under
`artifacts` when it fits one of the supported kinds, or its digest in the
asset manifest; the validator checks only files listed directly in `artifacts`.

```json
{
  "schema_version": 1,
  "task_id": "task-id-or-local-label",
  "objective": "Explain the product to a new contributor",
  "target_user": "Potential contributor",
  "references": [
    {
      "url": "https://example.com/reference",
      "observed_at": "2026-09-26T09:00:00+09:00",
      "useful_principle": "Strong reading order",
      "adaptation": "Short proof cards with restrained motion"
    }
  ],
  "selected_direction": {
    "concept_label": "A",
    "source": "reported_user_choice",
    "rationale": "Recorded after the user selected A"
  },
  "artifacts": [
    {
      "kind": "brief",
      "label": "Design brief",
      "path": "design/brief.md",
      "sha256": "REPLACE_WITH_64_LOWERCASE_HEX_CHARACTERS"
    }
  ]
}
```

Supported `kind` values: `brief`, `reference_notes`, `concept`, `tokens`,
`components`, `responsive`, `asset_manifest`, `implementation`, and
`browser_review`. Additional concept variants can have distinct labels.
An implementation artifact should be a stable source file or a bounded report
listing the exact source files and revision; a browser review should link its
screenshots or traces. Update hashes whenever the bytes change.

```sh
shasum -a 256 design/brief.md
aporic design validate --workspace . --manifest design/manifest.json
```

The equivalent MCP tool is `aporic_design_validate`. It is read-only and
returns the same report. `design_artifacts_complete` means all seven design
categories have matching files, a matching selected concept, and at least one
declared reference. `delivery_artifacts_complete` also requires matching
implementation and browser review categories. Neither flag means that the
reference was actually reviewed, that the user chose the concept, that the
implementation matches the specification, or that the design is useful.

The validator reads at most a 256 KiB manifest, 32 artifact entries, and
8 MiB per artifact (64 MiB total). Paths must remain inside the supplied
workspace, including after symlink resolution. Large image assets should be
inventoried with their hashes in `asset_manifest`; their bytes are outside the
validator's direct assurance. The package format is independent of the Aporic
SQLite schema, so existing evidence and replay remain unchanged.
