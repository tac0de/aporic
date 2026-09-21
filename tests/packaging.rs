#[test]
fn packaged_hooks_include_turn_scoped_intent_fidelity() {
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let hooks: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo.join("packaging/codex-plugin/hooks/hooks.json")).unwrap(),
    )
    .unwrap();

    let handlers = hooks["hooks"]["UserPromptSubmit"].as_array().unwrap();
    assert_eq!(handlers.len(), 1);
    assert!(handlers[0].get("matcher").is_none());
    assert_eq!(
        handlers[0]["hooks"][0]["command"],
        "\"${PLUGIN_ROOT}/bin/aporic\" codex-global-user-prompt-submit"
    );
    assert_eq!(
        hooks["hooks"]["PostToolUse"][0]["hooks"][0]["command"],
        "\"${PLUGIN_ROOT}/bin/aporic\" codex-global-post-tool-use"
    );
    assert_eq!(
        hooks["hooks"]["SessionEnd"][0]["hooks"][0]["command"],
        "\"${PLUGIN_ROOT}/bin/aporic\" codex-global-session-end"
    );
}

#[test]
fn packager_builds_and_copies_the_structural_wasm_analyzer() {
    let script = include_str!("../scripts/package-codex-plugin.sh");
    assert!(script.contains("--target wasm32-unknown-unknown --release --locked"));
    assert!(script.contains("analyzers/structural.wasm"));
    assert!(script.contains("aporic_structural_analyzer.wasm"));
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn packager_explains_missing_rust_toolchain_before_creating_output() {
    use std::path::PathBuf;
    use std::process::Command;

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_dir = std::env::temp_dir().join(format!(
        "aporic-package-missing-rust-{}",
        std::process::id()
    ));
    assert!(!output_dir.exists());

    let output = Command::new("/bin/sh")
        .arg(repo.join("scripts/package-codex-plugin.sh"))
        .arg(&output_dir)
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("packager should start");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("cargo and rustc must be available in PATH"));
    assert!(
        stderr.contains("Homebrew rustup detected")
            || stderr.contains("install Rust 1.89 or newer with rustup")
    );
    assert!(!output_dir.exists());
}
