use aporic_projects::{BindRequest, Error, bind_project, load_project, observe_project};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

struct TestArea(PathBuf);

impl TestArea {
    fn new() -> Self {
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("aporic-projects-{}-{sequence}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn repo(&self) -> PathBuf {
        self.0.join("repo")
    }

    fn registry(&self) -> PathBuf {
        self.0.join("registry")
    }
}

impl Drop for TestArea {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(directory: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .status()
        .unwrap();
    assert!(status.success());
}

fn initialize_repo(area: &TestArea) {
    let repo = area.repo();
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "--quiet"]);
    git(
        &repo,
        &[
            "-c",
            "user.name=Aporic Test",
            "-c",
            "user.email=aporic@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "initial",
        ],
    );
    git(
        &repo,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/acme/demo.git",
        ],
    );
}

fn request(area: &TestArea) -> BindRequest {
    BindRequest {
        project_id: "demo".into(),
        workspace: area.repo(),
        remote_name: "origin".into(),
    }
}

#[test]
fn creates_and_loads_an_external_immutable_binding() {
    let area = TestArea::new();
    initialize_repo(&area);
    let binding = bind_project(area.registry(), request(&area)).unwrap();
    let loaded = load_project(area.registry(), "demo").unwrap();

    assert_eq!(binding, loaded);
    assert_eq!(
        binding.remote_url(),
        "https://example.invalid/acme/demo.git"
    );
    assert!(!area.repo().join(".aporic").exists());
    assert!(!observe_project(&binding).unwrap().head_changed);

    assert!(matches!(
        bind_project(area.registry(), request(&area)),
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists
    ));
}

#[test]
fn observes_head_and_dirty_changes_without_mutating_the_binding() {
    let area = TestArea::new();
    initialize_repo(&area);
    let binding = bind_project(area.registry(), request(&area)).unwrap();
    std::fs::write(area.repo().join("new.txt"), "new\n").unwrap();
    let dirty = observe_project(&binding).unwrap();
    assert!(dirty.dirty);
    assert!(!dirty.head_changed);

    git(&area.repo(), &["add", "new.txt"]);
    git(
        &area.repo(),
        &[
            "-c",
            "user.name=Aporic Test",
            "-c",
            "user.email=aporic@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "next",
        ],
    );
    let changed = observe_project(&binding).unwrap();
    assert!(changed.head_changed);
    assert!(!changed.dirty);
    assert_eq!(load_project(area.registry(), "demo").unwrap(), binding);
}

#[test]
fn registry_inside_the_governed_workspace_is_rejected() {
    let area = TestArea::new();
    initialize_repo(&area);
    let registry = area.repo().join("state");

    assert!(matches!(
        bind_project(registry, request(&area)),
        Err(Error::InvalidBinding(
            "registry must remain outside the governed workspace"
        ))
    ));
    assert!(!area.repo().join("state").exists());
}

#[test]
fn changed_remote_fails_closed() {
    let area = TestArea::new();
    initialize_repo(&area);
    let binding = bind_project(area.registry(), request(&area)).unwrap();
    git(
        &area.repo(),
        &[
            "remote",
            "set-url",
            "origin",
            "https://example.invalid/acme/other.git",
        ],
    );

    assert!(matches!(
        observe_project(&binding),
        Err(Error::InvalidBinding("remote URL changed"))
    ));
}

#[test]
fn non_git_workspace_and_missing_remote_are_rejected() {
    let area = TestArea::new();
    std::fs::create_dir(area.repo()).unwrap();
    assert!(matches!(
        bind_project(area.registry(), request(&area)),
        Err(Error::Git {
            operation: "find root"
        })
    ));

    git(&area.repo(), &["init", "--quiet"]);
    git(
        &area.repo(),
        &[
            "-c",
            "user.name=Aporic Test",
            "-c",
            "user.email=aporic@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "initial",
        ],
    );
    assert!(matches!(
        bind_project(area.registry(), request(&area)),
        Err(Error::Git {
            operation: "read remote URL"
        })
    ));
}

#[test]
fn embedded_http_credentials_are_rejected() {
    let area = TestArea::new();
    initialize_repo(&area);
    git(
        &area.repo(),
        &[
            "remote",
            "set-url",
            "origin",
            "https://user:secret@example.invalid/acme/demo.git",
        ],
    );

    assert!(matches!(
        bind_project(area.registry(), request(&area)),
        Err(Error::InvalidBinding(
            "remote URL must not contain embedded credentials"
        ))
    ));
}
