use aporic::runtime;

#[test]
fn runtime_identity_contains_compiled_release_fields() {
    let identity = runtime::current();
    assert_eq!(identity.package_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(identity.source_sha256.len(), 64);
    assert!(identity.compiler.rustc_version.starts_with("rustc "));
    assert_eq!(identity.database_schema_version, Some(30));
}

#[test]
fn source_digest_changes_when_a_compile_input_changes() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path();
    let package = workspace.join("crates/aporic");
    std::fs::create_dir_all(package.join("src")).unwrap();
    std::fs::create_dir_all(workspace.join("migrations")).unwrap();
    std::fs::create_dir_all(workspace.join("canon")).unwrap();
    std::fs::write(package.join("Cargo.toml"), "[package]\nname = \"aporic\"\n").unwrap();
    std::fs::write(package.join("src/lib.rs"), "pub const VALUE: u8 = 1;\n").unwrap();
    std::fs::write(workspace.join("Cargo.lock"), "version = 4\n").unwrap();
    std::fs::write(workspace.join("Cargo.toml"), "[workspace]\n").unwrap();
    std::fs::write(workspace.join("canon/KERNEL.md"), "kernel v1\n").unwrap();
    let first = runtime::source_digest(&package, workspace).unwrap();
    std::fs::write(package.join("src/lib.rs"), "pub const VALUE: u8 = 2;\n").unwrap();
    let second = runtime::source_digest(&package, workspace).unwrap();
    assert_ne!(first, second);
    std::fs::write(workspace.join("canon/KERNEL.md"), "kernel v2\n").unwrap();
    let third = runtime::source_digest(&package, workspace).unwrap();
    assert_ne!(second, third);
}

#[test]
fn identity_command_does_not_open_or_migrate_a_database() {
    let area = tempfile::tempdir().unwrap();
    let database = area.path().join("must-not-exist.sqlite3");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_aporic"))
        .arg("identity")
        .env("APORIC_DATABASE", &database)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!database.exists());
    let identity: runtime::RuntimeIdentity = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(identity.package_version, env!("CARGO_PKG_VERSION"));
}
