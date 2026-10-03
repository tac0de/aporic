use std::{collections::BTreeMap, fs};

use aporic::{
    delivery::{
        self, DeliveryBriefRequest, DeliveryCheck, DeliveryFile, DeliveryImpactRequest,
        DeliveryManifest, DeliveryNode, DeliveryValidateRequest, ExecutionManifest, NodeKind,
        ScenarioManifest,
    },
    delivery_profiles::{self, ProfileSelection},
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn node(id: &str, kind: NodeKind, depends: &[&str]) -> DeliveryNode {
    DeliveryNode {
        id: id.to_owned(),
        kind,
        description: format!("Inventory {id}"),
        depends_on: depends.iter().map(|id| (*id).to_owned()).collect(),
        file: None,
        scenario: None,
    }
}
fn fixture(profile: &str) -> (TempDir, DeliveryManifest) {
    let area = tempfile::tempdir().unwrap();
    let mut nodes = Vec::new();
    let mut checks = Vec::new();
    for branch in ["a", "b"] {
        let id = |suffix: &str| format!("{branch}.{suffix}");
        nodes.push(node(&id("requirement"), NodeKind::Requirement, &[]));
        let mut scenario = node(&id("scenario"), NodeKind::Scenario, &[&id("requirement")]);
        scenario.scenario = Some(ScenarioManifest {
            requirements: vec![id("requirement")],
            preconditions: vec!["Session exists".into()],
            actions: vec!["Submit input".into()],
            expected: vec!["Input is stored".into()],
            failure_recovery: vec!["Retry once after temporary failure".into()],
        });
        nodes.push(scenario);
        nodes.push(node(&id("design"), NodeKind::Design, &[&id("scenario")]));
        nodes.push(node(
            &id("implementation"),
            NodeKind::Implementation,
            &[&id("design"), &id("source")],
        ));
        for (suffix, kind) in [
            ("source", NodeKind::Source),
            ("dependency", NodeKind::Dependency),
            ("build", NodeKind::Build),
            ("environment", NodeKind::Environment),
        ] {
            let path = format!("{branch}-{suffix}.txt");
            let bytes = format!("{branch} {suffix} version 1");
            fs::write(area.path().join(&path), &bytes).unwrap();
            let mut item = node(&id(suffix), kind, &[]);
            item.file = Some(DeliveryFile {
                path,
                sha256: sha(bytes.as_bytes()),
            });
            nodes.push(item);
        }
        nodes.push(node(
            &id("test"),
            NodeKind::Test,
            &[
                &id("implementation"),
                &id("dependency"),
                &id("build"),
                &id("environment"),
            ],
        ));
        checks.push(DeliveryCheck {
            id: id("check"),
            command_id: "test".into(),
            scenario_ids: vec![id("scenario")],
            test_id: id("test"),
            categories: delivery_profiles::definitions(profile, 1)
                .unwrap()
                .into_iter()
                .map(|category| category.id)
                .collect(),
            input_sha256: BTreeMap::new(),
            run_id: Some(format!("run-{branch}")),
            result_path: Some(format!("result-{branch}.json")),
        });
    }
    let mut manifest = DeliveryManifest {
        schema_version: 1,
        project_id: "fixture".into(),
        revision: "v1".into(),
        execution: ExecutionManifest {
            stack: vec!["Rust".into()],
            architecture: vec!["Local application".into()],
            commands: BTreeMap::from([("test".into(), vec!["cargo".into(), "test".into()])]),
            command_cwds: BTreeMap::new(),
            supported_environments: vec!["Linux".into()],
            config_refs: vec!["API_TOKEN".into()],
            constraints: vec!["Advisory reports".into()],
            quality_targets: vec!["Repeatable checks".into()],
        },
        profiles: vec![ProfileSelection {
            id: profile.into(),
            version: 1,
            not_applicable: Vec::new(),
        }],
        nodes,
        checks,
    };
    refresh_inputs(&mut manifest);
    write_manifest(&area, "manifest.json", &manifest);
    (area, manifest)
}
fn refresh_inputs(manifest: &mut DeliveryManifest) {
    for index in 0..manifest.checks.len() {
        manifest.checks[index].input_sha256 =
            delivery::check_input_fingerprints(manifest, &manifest.checks[index]).unwrap();
    }
}
fn write_manifest(area: &TempDir, path: &str, manifest: &DeliveryManifest) {
    fs::write(
        area.path().join(path),
        serde_json::to_vec_pretty(manifest).unwrap(),
    )
    .unwrap();
}
fn request(area: &TempDir) -> DeliveryValidateRequest {
    DeliveryValidateRequest {
        workspace: area.path().display().to_string(),
        manifest: "manifest.json".into(),
    }
}
fn impact_request(area: &TempDir) -> DeliveryImpactRequest {
    DeliveryImpactRequest {
        workspace: area.path().display().to_string(),
        previous: "previous.json".into(),
        current: "manifest.json".into(),
    }
}

#[test]
fn valid_inventory_cannot_establish_runner_execution_or_test_adequacy() {
    for profile in ["general", "web_game", "web_platform"] {
        let (area, _) = fixture(profile);
        let report = delivery::validate(&request(&area)).unwrap();
        assert!(report.valid, "{:?} {:?}", report.gaps, report.profile_gaps);
        assert!(
            report
                .checks
                .iter()
                .all(|c| c.coverage_complete && c.inputs_current && !c.execution_verified)
        );
        assert!(
            report
                .assurance_boundary
                .contains("not proof of semantic test adequacy")
        );
    }
}

#[test]
fn snapshots_must_include_every_prerequisite_and_the_test_itself() {
    let (area, baseline) = fixture("general");
    for removed in [
        "a.test",
        "a.source",
        "a.dependency",
        "a.build",
        "a.environment",
        "a.requirement",
    ] {
        let mut manifest = baseline.clone();
        manifest.checks[0].input_sha256.remove(removed);
        write_manifest(&area, "manifest.json", &manifest);
        let report = delivery::validate(&request(&area)).unwrap();
        assert!(!report.valid, "accepted omitted {removed}");
        assert!(report.checks[0].coverage_complete);
        assert!(!report.checks[0].inputs_current);
        assert!(
            report.checks[0]
                .gaps
                .iter()
                .any(|g| g.contains("complete test prerequisite closure"))
        );
    }
    let mut manifest = baseline;
    manifest.checks[0]
        .input_sha256
        .insert("b.source".into(), "0".repeat(64));
    write_manifest(&area, "manifest.json", &manifest);
    assert!(!delivery::validate(&request(&area)).unwrap().valid);
}

#[test]
fn essential_prerequisites_need_file_backing_not_only_declared_labels() {
    for suffix in ["source", "dependency", "build", "environment"] {
        let (area, mut manifest) = fixture("general");
        manifest
            .nodes
            .iter_mut()
            .find(|n| n.id == format!("a.{suffix}"))
            .unwrap()
            .file = None;
        refresh_inputs(&mut manifest);
        write_manifest(&area, "manifest.json", &manifest);
        let report = delivery::validate(&request(&area)).unwrap();
        assert!(!report.checks[0].coverage_complete);
        assert!(
            report.checks[0]
                .gaps
                .iter()
                .any(|g| g.contains("file-backed"))
        );
    }
}

#[test]
fn graph_and_schema_corruption_is_rejected() {
    let (area, baseline) = fixture("general");
    let mut malformed = Vec::new();
    let mut manifest = baseline.clone();
    manifest.schema_version = 2;
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.nodes.push(manifest.nodes[0].clone());
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.nodes[0].depends_on.push("missing".into());
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.nodes[0].depends_on.push("a.test".into());
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.checks.push(manifest.checks[0].clone());
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.checks[0].test_id = "a.source".into();
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.checks[0].scenario_ids = vec!["a.requirement".into()];
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.nodes[1].scenario = None;
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.nodes[0].scenario = baseline.nodes[1].scenario.clone();
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.execution.commands.insert("empty".into(), vec![]);
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest.execution.config_refs = vec!["TOKEN=secret".into()];
    malformed.push(manifest);
    let mut manifest = baseline.clone();
    manifest
        .execution
        .command_cwds
        .insert("missing".into(), ".".into());
    malformed.push(manifest);
    for manifest in malformed {
        write_manifest(&area, "manifest.json", &manifest);
        assert!(delivery::validate(&request(&area)).is_err());
    }
    let mut json = serde_json::to_value(&baseline).unwrap();
    json["unrecognized_field"] = true.into();
    fs::write(
        area.path().join("manifest.json"),
        serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
    assert!(delivery::validate(&request(&area)).is_err());
}

#[test]
fn incomplete_trace_and_profile_categories_are_explicit_gaps() {
    for profile in ["general", "web_game", "web_platform"] {
        let (area, mut manifest) = fixture(profile);
        let omitted = manifest.checks[0].categories.pop().unwrap();
        manifest.checks[1]
            .categories
            .retain(|category| category != &omitted);
        write_manifest(&area, "manifest.json", &manifest);
        let report = delivery::validate(&request(&area)).unwrap();
        assert!(!report.valid);
        assert!(
            report
                .profile_gaps
                .iter()
                .any(|gap| gap.message.contains(&omitted))
        );
        manifest.nodes[1]
            .scenario
            .as_mut()
            .unwrap()
            .expected
            .clear();
        manifest.checks[0].scenario_ids.clear();
        write_manifest(&area, "manifest.json", &manifest);
        let report = delivery::validate(&request(&area)).unwrap();
        assert!(
            report
                .gaps
                .iter()
                .any(|gap| gap.message.contains("expected"))
        );
        assert!(
            report
                .gaps
                .iter()
                .any(|gap| gap.message.contains("no mapped check"))
        );
    }
}

#[test]
fn categories_are_package_mappings_and_do_not_claim_every_scenario_has_every_category() {
    let (area, mut manifest) = fixture("web_game");
    manifest.checks[1].categories.clear();
    write_manifest(&area, "manifest.json", &manifest);
    assert!(delivery::validate(&request(&area)).unwrap().valid);
}

#[test]
fn invalid_inventory_empty_execution_and_blank_scenario_steps_do_not_pass() {
    let (area, mut manifest) = fixture("general");
    manifest.nodes[1].scenario.as_mut().unwrap().actions = vec!["  ".into()];
    manifest.execution.stack.clear();
    write_manifest(&area, "manifest.json", &manifest);
    let report = delivery::validate(&request(&area)).unwrap();
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.message.contains("execution.stack"))
    );
    assert!(report.gaps.iter().any(|g| g.message.contains("actions")));
    manifest.nodes.clear();
    manifest.checks.clear();
    manifest.profiles.clear();
    write_manifest(&area, "manifest.json", &manifest);
    assert!(!delivery::validate(&request(&area)).unwrap().valid);
}

#[test]
fn stale_and_missing_file_bytes_cannot_receive_current_input_credit() {
    let (area, _) = fixture("general");
    fs::write(area.path().join("a-source.txt"), "changed bytes").unwrap();
    fs::remove_file(area.path().join("b-source.txt")).unwrap();
    let report = delivery::validate(&request(&area)).unwrap();
    assert!(!report.valid);
    assert!(report.checks.iter().all(|c| !c.inputs_current));
    assert!(report.nodes.iter().filter(|n| !n.current).count() == 2);
}

#[test]
fn paths_reject_traversal_absolute_paths_and_symlink_escape() {
    let (area, baseline) = fixture("general");
    for path in [
        "../outside",
        "/tmp/outside",
        "a/../../outside",
        "a\\outside",
        "./a-source.txt",
    ] {
        let mut manifest = baseline.clone();
        manifest.nodes[4].file.as_mut().unwrap().path = path.into();
        write_manifest(&area, "manifest.json", &manifest);
        assert!(
            delivery::validate(&request(&area)).is_err(),
            "accepted {path}"
        );
    }
    let mut req = request(&area);
    req.manifest = "../manifest.json".into();
    assert!(delivery::validate(&req).is_err());
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), area.path().join("escape")).unwrap();
        let mut manifest = baseline;
        manifest.checks[0].result_path = Some("escape/not-yet-created.json".into());
        write_manifest(&area, "manifest.json", &manifest);
        assert!(delivery::validate(&request(&area)).is_err());
        // A dangling symlink is still an escaping path; exists() alone would
        // incorrectly skip it while searching for the nearest existing parent.
        std::os::unix::fs::symlink(outside.path().join("absent"), area.path().join("broken"))
            .unwrap();
        manifest.checks[0].result_path = Some("broken/output.json".into());
        write_manifest(&area, "manifest.json", &manifest);
        assert!(delivery::validate(&request(&area)).is_err());
    }
}

#[test]
fn oversized_manifest_and_regular_file_limit_are_bounded() {
    let (area, baseline) = fixture("general");
    let file = fs::File::create(area.path().join("manifest.json")).unwrap();
    file.set_len(delivery::MAX_MANIFEST_BYTES + 1).unwrap();
    assert!(delivery::validate(&request(&area)).is_err());
    write_manifest(&area, "manifest.json", &baseline);
    fs::File::create(area.path().join("a-source.txt"))
        .unwrap()
        .set_len(delivery::MAX_NODE_FILE_BYTES + 1)
        .unwrap();
    let report = delivery::validate(&request(&area)).unwrap();
    assert!(!report.valid);
    assert!(
        report
            .nodes
            .iter()
            .any(|n| n.gaps.iter().any(|g| g.contains("byte limit")))
    );
}

#[test]
fn semantic_hashes_ignore_dependency_order_but_bind_metadata_and_declared_file_hashes() {
    let (_, manifest) = fixture("general");
    let original = manifest.nodes.last().unwrap();
    let mut reordered = original.clone();
    reordered.depends_on.reverse();
    assert_eq!(
        delivery::node_fingerprint(original).unwrap(),
        delivery::node_fingerprint(&reordered).unwrap()
    );
    reordered.description.push_str(" changed");
    assert_ne!(
        delivery::node_fingerprint(original).unwrap(),
        delivery::node_fingerprint(&reordered).unwrap()
    );
    let mut explicit_root = manifest.execution.clone();
    explicit_root.command_cwds.insert("test".into(), ".".into());
    assert_eq!(
        delivery::execution_fingerprint(&manifest.execution).unwrap(),
        delivery::execution_fingerprint(&explicit_root).unwrap()
    );
}

#[test]
fn focused_brief_includes_execution_and_transitive_prerequisites_with_explicit_omissions() {
    let (area, _) = fixture("general");
    let request = DeliveryBriefRequest {
        workspace: area.path().display().to_string(),
        manifest: "manifest.json".into(),
        focus_ids: vec!["a.test".into()],
        max_nodes: 100,
        max_bytes: 16000,
    };
    let brief = delivery::brief(&request).unwrap();
    assert!(brief.text.contains("commands"));
    assert!(brief.selected_node_ids.contains(&"a.requirement".into()));
    assert!(brief.selected_node_ids.contains(&"a.test".into()));
    assert!(!brief.selected_node_ids.contains(&"b.test".into()));
    assert!(!brief.truncated);
    let mut limited = request.clone();
    limited.max_bytes = 256;
    limited.max_nodes = 1;
    let brief = delivery::brief(&limited).unwrap();
    assert!(brief.text.len() <= 256);
    assert!(brief.truncated);
    assert!(!brief.omitted_node_ids.is_empty());
    assert!(brief.omitted_sections.contains(&"execution".into()));
    limited.focus_ids = vec!["unknown".into()];
    assert!(delivery::brief(&limited).is_err());
}

#[test]
fn changed_input_invalidates_only_its_dependent_check_even_when_snapshots_are_stale() {
    let (area, mut manifest) = fixture("general");
    write_manifest(&area, "previous.json", &manifest);
    manifest
        .nodes
        .iter_mut()
        .find(|n| n.id == "a.source")
        .unwrap()
        .description
        .push_str(" revised contract");
    manifest.revision = "v2".into();
    // Keep old snapshots: this is a known stale input, not an unknown graph mapping.
    write_manifest(&area, "manifest.json", &manifest);
    let report = delivery::impact(&impact_request(&area)).unwrap();
    assert!(!report.unknown_mapping);
    assert_eq!(report.invalidated_check_ids, vec!["a.check"]);
    assert_eq!(report.preserved_check_ids, vec!["b.check"]);
    assert!(report.affected_node_ids.contains(&"a.test".into()));
    assert!(!report.affected_node_ids.contains(&"b.test".into()));
}

#[test]
fn stale_current_file_invalidates_its_check_without_erasing_unaffected_credit() {
    let (area, manifest) = fixture("general");
    write_manifest(&area, "previous.json", &manifest);
    fs::write(area.path().join("a-source.txt"), "replacement").unwrap();
    let report = delivery::impact(&impact_request(&area)).unwrap();
    assert!(!report.unknown_mapping);
    assert_eq!(report.invalidated_check_ids, vec!["a.check"]);
    assert_eq!(report.preserved_check_ids, vec!["b.check"]);
    assert!(report.affected_node_ids.contains(&"a.test".into()));
}

#[test]
fn unchanged_bound_checks_have_only_conditional_preservation() {
    let (area, mut manifest) = fixture("general");
    write_manifest(&area, "previous.json", &manifest);
    let report = delivery::impact(&impact_request(&area)).unwrap();
    assert_eq!(report.preserved_check_ids, vec!["a.check", "b.check"]);
    assert!(report.reasons.iter().any(|r| r.contains("conditional")));
    manifest.checks[0].run_id = None;
    write_manifest(&area, "manifest.json", &manifest);
    let report = delivery::impact(&impact_request(&area)).unwrap();
    assert_eq!(report.invalidated_check_ids, vec!["a.check"]);
    assert_eq!(report.preserved_check_ids, vec!["b.check"]);
}

#[test]
fn execution_changes_profile_changes_and_unknown_mappings_invalidate_all_relevant_checks() {
    let (area, baseline) = fixture("web_game");
    write_manifest(&area, "previous.json", &baseline);
    let mut manifest = baseline.clone();
    manifest
        .execution
        .commands
        .get_mut("test")
        .unwrap()
        .push("--release".into());
    write_manifest(&area, "manifest.json", &manifest);
    let report = delivery::impact(&impact_request(&area)).unwrap();
    assert!(report.execution_changed);
    assert_eq!(report.invalidated_check_ids.len(), 2);
    let mut manifest = baseline.clone();
    manifest.profiles[0]
        .not_applicable
        .push(delivery_profiles::ProfileExclusion {
            category: "web_game.audio".into(),
            reason: "No audio output".into(),
        });
    write_manifest(&area, "manifest.json", &manifest);
    let report = delivery::impact(&impact_request(&area)).unwrap();
    assert!(report.profiles_changed);
    assert_eq!(report.invalidated_check_ids.len(), 2);
    let mut manifest = baseline;
    manifest.checks[0].scenario_ids = vec!["b.scenario".into()];
    write_manifest(&area, "manifest.json", &manifest);
    let report = delivery::impact(&impact_request(&area)).unwrap();
    assert!(report.unknown_mapping);
    assert_eq!(report.invalidated_check_ids.len(), 2);
}

#[test]
fn identical_manifest_with_wrong_snapshots_cannot_preserve_credit() {
    let (area, mut manifest) = fixture("general");
    manifest.checks[0].input_sha256.remove("a.test");
    write_manifest(&area, "previous.json", &manifest);
    write_manifest(&area, "manifest.json", &manifest);
    let report = delivery::impact(&impact_request(&area)).unwrap();
    assert!(!report.unknown_mapping);
    assert_eq!(report.invalidated_check_ids, vec!["a.check"]);
    assert_eq!(report.preserved_check_ids, vec!["b.check"]);
}

#[cfg(unix)]
#[test]
fn fifo_inputs_are_rejected_without_waiting_for_a_writer() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let (area, _) = fixture("general");
    let path = area.path().join("a-source.txt");
    fs::remove_file(&path).unwrap();
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: name is a valid NUL-terminated local path retained for the call.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let report = delivery::validate(&request(&area)).unwrap();
    assert!(!report.valid);
    assert!(
        report
            .nodes
            .iter()
            .any(|node| node.gaps.iter().any(|gap| gap.contains("regular file")))
    );
    fs::remove_file(area.path().join("manifest.json")).unwrap();
    let name = CString::new(area.path().join("manifest.json").as_os_str().as_bytes()).unwrap();
    // SAFETY: name is a valid NUL-terminated local path retained for the call.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert!(delivery::validate(&request(&area)).is_err());
}

#[test]
fn aggregate_file_budget_prevents_reading_files_after_the_budget_is_consumed() {
    let (area, _) = fixture("general");
    for suffix in ["source", "dependency", "build", "environment"] {
        fs::File::create(area.path().join(format!("a-{suffix}.txt")))
            .unwrap()
            .set_len(delivery::MAX_NODE_FILE_BYTES)
            .unwrap();
    }
    let report = delivery::validate(&request(&area)).unwrap();
    let not_read = report
        .nodes
        .iter()
        .find(|node| node.id == "b.source")
        .unwrap();
    assert!(not_read.file_sha256.is_none());
    assert!(not_read.gaps.iter().any(|gap| gap.contains("0-byte limit")));
}

#[test]
fn a_second_unfile_backed_source_cannot_hide_behind_a_valid_source() {
    let (area, mut manifest) = fixture("general");
    manifest
        .nodes
        .push(node("a.unbound-source", NodeKind::Source, &[]));
    manifest
        .nodes
        .iter_mut()
        .find(|n| n.id == "a.test")
        .unwrap()
        .depends_on
        .push("a.unbound-source".into());
    refresh_inputs(&mut manifest);
    write_manifest(&area, "manifest.json", &manifest);
    let report = delivery::validate(&request(&area)).unwrap();
    assert!(!report.checks[0].coverage_complete);
    assert!(
        report.checks[0]
            .gaps
            .iter()
            .any(|g| g.contains("unfile-backed"))
    );
}
