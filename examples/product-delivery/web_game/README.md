# Declarative web_game delivery package

Validate from the repository root with workspace `.` and manifest `examples/product-delivery/web_game/manifest.json`. The package includes inherited general and web categories plus every web_game category.

Each category has a requirement, scenario, design, implementation reference and test contract. Shared source, dependency, asset, build and environment declarations are prerequisite nodes. `source.rs` illustrates a small Rust model; the remaining behaviors are specifications.

The declared `product/Cargo.toml` command is an intended product interface, not an existing build. Replace it and the partial source with real product artifacts before execution. Every check intentionally omits `run_id` and `result_path`, so validation must report unbound checks and no verified execution credit. Saving these files has not run the product, tests or browser.

To test omission, remove a check mapping for any category; the profile then has a missing required-category mapping. To test staleness, alter a referenced file without updating its recorded digest. Both cases should appear as gaps rather than success.
