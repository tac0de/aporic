use aporic_roles::{Error, HostPolicy, MAX_ROLE_MARKDOWN_BYTES, RoutingTier, load_role};
use serde_json::json;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

struct TestPackage(PathBuf);

impl TestPackage {
    fn new(document: serde_json::Value, instructions: &str) -> Self {
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("aporic-role-{}-{sequence}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("role.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        std::fs::write(path.join("role.md"), instructions).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestPackage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn document() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "role_id": "generalist",
        "role_version": 1,
        "capabilities": ["process.execute", "workspace.read", "workspace.write"],
        "routing": {"default": "balanced", "ceiling": "deep"},
        "limits": {"max_tool_calls": 32, "max_parallel_tasks": 1},
        "may_delegate": false,
        "report": {"format": "decision_card", "max_items": 4}
    })
}

fn host() -> HostPolicy {
    HostPolicy {
        capabilities: ["workspace.read", "workspace.write"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>(),
        default_routing: RoutingTier::Balanced,
        routing_ceiling: RoutingTier::Balanced,
        max_tool_calls: 12,
        max_parallel_tasks: 4,
        may_delegate: true,
    }
}

#[test]
fn role_can_only_narrow_the_host_policy() {
    let package = TestPackage::new(document(), "# Generalist\n\nStay within scope.\n");
    let profile = load_role(package.path(), &host()).unwrap();

    assert_eq!(
        profile.capabilities(),
        ["workspace.read", "workspace.write"]
    );
    assert_eq!(profile.unavailable_capabilities(), ["process.execute"]);
    assert_eq!(profile.default_routing(), RoutingTier::Balanced);
    assert_eq!(profile.routing_ceiling(), RoutingTier::Balanced);
    assert_eq!(profile.max_tool_calls(), 12);
    assert_eq!(profile.max_parallel_tasks(), 1);
    assert!(!profile.may_delegate());
}

#[test]
fn role_limits_win_when_they_are_lower() {
    let package = TestPackage::new(document(), "# Generalist\n");
    let mut permissive = host();
    permissive.max_tool_calls = 100;
    permissive.routing_ceiling = RoutingTier::Deep;
    permissive.default_routing = RoutingTier::Deep;

    let profile = load_role(package.path(), &permissive).unwrap();
    assert_eq!(profile.max_tool_calls(), 32);
    assert_eq!(profile.max_parallel_tasks(), 1);
    assert_eq!(profile.default_routing(), RoutingTier::Balanced);
    assert_eq!(profile.routing_ceiling(), RoutingTier::Deep);
}

#[test]
fn fingerprint_is_deterministic_and_covers_both_files() {
    let package = TestPackage::new(document(), "# Generalist\n");
    let first = load_role(package.path(), &host()).unwrap();
    let second = load_role(package.path(), &host()).unwrap();
    assert_eq!(first.source_sha256(), second.source_sha256());
    assert_eq!(first.profile_sha256(), second.profile_sha256());

    std::fs::write(package.path().join("role.md"), "# Generalist\n\nChanged.\n").unwrap();
    let changed = load_role(package.path(), &host()).unwrap();
    assert_ne!(first.source_sha256(), changed.source_sha256());
    assert_ne!(first.profile_sha256(), changed.profile_sha256());
}

#[test]
fn effective_host_limits_are_part_of_the_profile_fingerprint() {
    let package = TestPackage::new(document(), "# Generalist\n");
    let first = load_role(package.path(), &host()).unwrap();
    let mut changed_host = host();
    changed_host.max_tool_calls = 11;
    let changed = load_role(package.path(), &changed_host).unwrap();

    assert_eq!(first.source_sha256(), changed.source_sha256());
    assert_ne!(first.profile_sha256(), changed.profile_sha256());
}

#[test]
fn unknown_json_fields_and_duplicate_capabilities_fail_closed() {
    let mut unknown = document();
    unknown["unexpected"] = json!(true);
    let package = TestPackage::new(unknown, "# Generalist\n");
    assert!(matches!(
        load_role(package.path(), &host()),
        Err(Error::Json(_))
    ));

    let mut duplicate = document();
    duplicate["capabilities"] = json!(["workspace.read", "workspace.read"]);
    let package = TestPackage::new(duplicate, "# Generalist\n");
    assert!(matches!(
        load_role(package.path(), &host()),
        Err(Error::InvalidPackage("duplicate role capability"))
    ));
}

#[test]
fn invalid_routing_and_oversized_markdown_fail_closed() {
    let mut invalid = document();
    invalid["routing"] = json!({"default": "deep", "ceiling": "economy"});
    let package = TestPackage::new(invalid, "# Generalist\n");
    assert!(matches!(
        load_role(package.path(), &host()),
        Err(Error::InvalidPackage(
            "default routing exceeds the role ceiling"
        ))
    ));

    let package = TestPackage::new(document(), &"x".repeat(MAX_ROLE_MARKDOWN_BYTES + 1));
    assert!(matches!(
        load_role(package.path(), &host()),
        Err(Error::InvalidPackage("role.md exceeds the byte limit"))
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_role_files_fail_closed() {
    use std::os::unix::fs::symlink;

    let package = TestPackage::new(document(), "# Generalist\n");
    let target = package.path().join("instructions-target.md");
    std::fs::write(&target, "# Replacement\n").unwrap();
    std::fs::remove_file(package.path().join("role.md")).unwrap();
    symlink(&target, package.path().join("role.md")).unwrap();

    assert!(matches!(
        load_role(package.path(), &host()),
        Err(Error::InvalidPackage("role.md must be a real regular file"))
    ));
}
