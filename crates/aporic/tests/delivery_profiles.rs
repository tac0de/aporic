use std::{collections::BTreeSet, fs, path::Path, process::Command};

use aporic::delivery::{
    DeliveryManifest, DeliveryValidateRequest, check_input_fingerprints, validate,
};
use aporic::delivery_profiles::{
    ProfileExclusion, ProfileSelection, definitions, required_categories,
};

fn selection(id: &str, category: &str, reason: &str) -> ProfileSelection {
    ProfileSelection {
        id: id.into(),
        version: 1,
        not_applicable: vec![ProfileExclusion {
            category: category.into(),
            reason: reason.into(),
        }],
    }
}

#[test]
fn versioned_profiles_inherit_required_categories_without_duplicates() {
    let general: BTreeSet<_> = definitions("general", 1)
        .unwrap()
        .into_iter()
        .map(|category| category.id)
        .collect();
    let web: BTreeSet<_> = definitions("web", 1)
        .unwrap()
        .into_iter()
        .map(|category| category.id)
        .collect();
    assert_eq!(general.len(), 4);
    assert_eq!(web.len(), 10);
    assert!(general.is_subset(&web));
    for (id, count) in [("web_game", 17), ("web_platform", 15)] {
        let categories = definitions(id, 1).unwrap();
        let ids: BTreeSet<_> = categories
            .iter()
            .map(|category| category.id.clone())
            .collect();
        assert_eq!(ids.len(), count);
        assert_eq!(ids.len(), categories.len());
        assert!(web.is_subset(&ids));
        assert!(
            categories
                .iter()
                .all(|category| !category.description.is_empty())
        );
    }
    assert!(definitions("unknown", 1).is_err());
    assert!(definitions("web", 0).is_err());
    assert!(definitions("web", 2).is_err());
}

#[test]
fn required_domain_categories_are_distinct_and_explicit() {
    for (profile, expected) in [
        (
            "web_game",
            vec![
                "state",
                "input",
                "save",
                "time",
                "visibility",
                "audio",
                "performance",
            ],
        ),
        (
            "web_platform",
            vec![
                "permissions",
                "integrity",
                "idempotency",
                "concurrency",
                "integration_failure",
            ],
        ),
    ] {
        let ids: BTreeSet<_> = definitions(profile, 1)
            .unwrap()
            .into_iter()
            .map(|category| category.id)
            .collect();
        for category in expected {
            assert!(ids.contains(&format!("{profile}.{category}")));
        }
    }
}

#[test]
fn exclusions_need_known_unique_categories_and_nonblank_reasons() {
    let allowed = selection(
        "web_game",
        "web_game.audio",
        "This game has no audio assets or playback.",
    );
    let required = required_categories(&allowed).unwrap();
    assert_eq!(required.len(), 16);
    assert!(
        required
            .iter()
            .all(|category| category.id != "web_game.audio")
    );
    assert!(required_categories(&selection("web_game", "web_game.audio", " \n\t")).is_err());
    assert!(
        required_categories(&selection(
            "web_game",
            "web_platform.permissions",
            "No platform"
        ))
        .is_err()
    );
    let mut duplicate = allowed.clone();
    duplicate
        .not_applicable
        .push(allowed.not_applicable[0].clone());
    assert!(required_categories(&duplicate).is_err());
    for category in definitions("general", 1).unwrap() {
        assert!(
            required_categories(&selection(
                "web_game",
                &category.id,
                "Reported out of scope"
            ))
            .is_err()
        );
    }
}

#[test]
fn profile_selection_round_trips_and_rejects_unknown_fields() {
    let selected = selection(
        "web_platform",
        "web_platform.integration_failure",
        "No external integrations are implemented.",
    );
    let serialized = serde_json::to_string(&selected).unwrap();
    assert_eq!(
        serde_json::from_str::<ProfileSelection>(&serialized).unwrap(),
        selected
    );
    let defaulted: ProfileSelection =
        serde_json::from_str(r#"{"id":"general","version":1}"#).unwrap();
    assert!(defaulted.not_applicable.is_empty());
    assert!(
        serde_json::from_str::<ProfileSelection>(r#"{"id":"web","version":1,"approved":true}"#)
            .is_err()
    );
}

fn example(profile: &str) -> (tempfile::TempDir, DeliveryManifest) {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let fixture = if profile == "web_platform" {
        "web_platform"
    } else {
        "web_game"
    };
    let manifest_path =
        repository.join(format!("examples/product-delivery/{fixture}/manifest.json"));
    let mut manifest: DeliveryManifest =
        serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
    let workspace = tempfile::tempdir().unwrap();
    for file in manifest.nodes.iter().filter_map(|node| node.file.as_ref()) {
        let destination = workspace.path().join(&file.path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(repository.join(&file.path), destination).unwrap();
    }
    manifest.profiles = vec![ProfileSelection {
        id: profile.into(),
        version: 1,
        not_applicable: vec![],
    }];
    let categories: BTreeSet<_> = definitions(profile, 1)
        .unwrap()
        .into_iter()
        .map(|category| category.id)
        .collect();
    for check in &mut manifest.checks {
        check
            .categories
            .retain(|category| categories.contains(category));
    }
    (workspace, manifest)
}

fn validate_example(
    workspace: &Path,
    manifest: &DeliveryManifest,
) -> aporic::delivery::DeliveryValidationReport {
    fs::write(
        workspace.join("manifest.json"),
        serde_json::to_vec_pretty(manifest).unwrap(),
    )
    .unwrap();
    validate(&DeliveryValidateRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        manifest: "manifest.json".into(),
    })
    .unwrap()
}

#[test]
fn checked_in_packages_have_complete_traces_but_no_execution_credit() {
    for profile in ["web_game", "web_platform"] {
        let (workspace, manifest) = example(profile);
        for check in &manifest.checks {
            assert_eq!(
                check.input_sha256,
                check_input_fingerprints(&manifest, check).unwrap(),
                "fixture fingerprints differ for {}",
                check.id
            );
        }
        let report = validate_example(workspace.path(), &manifest);
        assert!(report.profile_gaps.is_empty(), "{:?}", report.profile_gaps);
        assert!(
            report.nodes.iter().all(|node| node.current),
            "stale fixture nodes for {profile}: {:?}",
            report.nodes
        );
        assert!(
            report
                .checks
                .iter()
                .all(|check| check.coverage_complete && check.inputs_current)
        );
        assert!(report.checks.iter().all(|check| !check.execution_verified));
    }
}

#[test]
fn autocrlf_checkout_preserves_hash_bound_example_bytes() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for profile in ["web_game", "web_platform"] {
        let (workspace, manifest) = example(profile);
        fs::copy(
            repository.join(".gitattributes"),
            workspace.path().join(".gitattributes"),
        )
        .unwrap();
        let git = |args: &[&str]| {
            let output = Command::new("git")
                .current_dir(workspace.path())
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        git(&["init", "--quiet"]);
        git(&["-c", "core.autocrlf=false", "add", "."]);
        let paths: BTreeSet<_> = manifest
            .nodes
            .iter()
            .filter_map(|node| node.file.as_ref().map(|file| &file.path))
            .collect();
        for path in paths {
            fs::remove_file(workspace.path().join(path)).unwrap();
        }
        git(&[
            "-c",
            "core.autocrlf=true",
            "checkout-index",
            "--all",
            "--force",
        ]);
        let report = validate_example(workspace.path(), &manifest);
        assert!(
            report.nodes.iter().all(|node| node.current),
            "checkout changed hash-bound {profile} bytes: {:?}",
            report.nodes
        );
        assert!(report.checks.iter().all(|check| check.inputs_current));
        assert!(report.checks.iter().all(|check| !check.execution_verified));
    }
}

#[test]
fn every_required_category_omission_is_detected_for_each_profile() {
    for profile in ["general", "web", "web_game", "web_platform"] {
        let (workspace, manifest) = example(profile);
        assert!(
            validate_example(workspace.path(), &manifest)
                .profile_gaps
                .is_empty()
        );
        for category in definitions(profile, 1).unwrap() {
            let mut partial = manifest.clone();
            for check in &mut partial.checks {
                check.categories.retain(|id| id != &category.id);
            }
            let report = validate_example(workspace.path(), &partial);
            assert!(
                report
                    .profile_gaps
                    .iter()
                    .any(|gap| gap.message.contains(&category.id)),
                "missing {} mapping was not reported",
                category.id
            );
        }
    }
}

#[test]
fn changed_shared_source_invalidates_inputs_for_each_profile() {
    for profile in ["general", "web", "web_game", "web_platform"] {
        let (workspace, manifest) = example(profile);
        let source = manifest
            .nodes
            .iter()
            .find(|node| node.id == "source")
            .unwrap();
        fs::write(
            workspace.path().join(&source.file.as_ref().unwrap().path),
            "// changed after the declared source digest\n",
        )
        .unwrap();
        let report = validate_example(workspace.path(), &manifest);
        assert!(
            report
                .nodes
                .iter()
                .any(|node| node.id == "source" && !node.current)
        );
        assert!(
            report
                .checks
                .iter()
                .all(|check| !check.inputs_current && !check.execution_verified)
        );
    }
}
