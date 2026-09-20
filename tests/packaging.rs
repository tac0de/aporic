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
