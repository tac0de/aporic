use std::{collections::BTreeSet, path::Path, process::Output};

use crate::{
    bounded::MAX_GIT_OUTPUT_BYTES,
    domain::{
        GitObserveRequest, GitPathChange, GitSnapshotDraft, GitWorktreeObservation,
        GovernanceFinding, GovernanceSeverity,
    },
    git_process::run_hardened_git,
    store::{Error, Result},
};

const MAX_CHANGED_PATHS: usize = 512;
const MAX_WORKTREES: usize = 128;
const MAX_REMOTES: usize = 64;
pub(crate) const GIT_POLICY_VERSION: u32 = 1;

pub(crate) fn capture(request: &GitObserveRequest) -> Result<GitSnapshotDraft> {
    let workspace = std::fs::canonicalize(&request.workspace)?;
    if !workspace.is_dir() {
        return Err(Error::Invalid("workspace must be a directory".to_owned()));
    }
    if let Some(base_ref) = request.base_ref.as_deref() {
        validate_ref(base_ref)?;
    }

    let repository_root = required_text(&workspace, &["rev-parse", "--show-toplevel"])?;
    let repository_root = std::fs::canonicalize(repository_root.trim())?;
    let head_commit = optional_text(&repository_root, &["rev-parse", "--verify", "HEAD"])?;
    let head_tree = if head_commit.is_some() {
        optional_text(&repository_root, &["rev-parse", "--verify", "HEAD^{tree}"])?
    } else {
        None
    };
    let branch = optional_text(
        &repository_root,
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
    )?;
    let detached = head_commit.is_some() && branch.is_none();
    let upstream_ref = optional_text(
        &repository_root,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    )?;
    let (ahead_count, behind_count) = if head_commit.is_some() && upstream_ref.is_some() {
        parse_ahead_behind(&required_text(
            &repository_root,
            &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        )?)?
    } else {
        (None, None)
    };

    let selected_base = select_base_ref(
        &repository_root,
        request.base_ref.as_deref(),
        upstream_ref.as_deref(),
        branch.as_deref(),
    )?;
    let base_commit = match selected_base.as_deref() {
        Some(base) => optional_text(
            &repository_root,
            &["rev-parse", "--verify", &format!("{base}^{{commit}}")],
        )?,
        None => None,
    };
    let merge_base = match (head_commit.as_deref(), base_commit.as_deref()) {
        (Some(_), Some(base)) => optional_text(&repository_root, &["merge-base", "HEAD", base])?,
        _ => None,
    };

    let status = required_bytes(
        &repository_root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignore-submodules=all",
        ],
    )?;
    let (mut changed_paths, staged_count, unstaged_count, untracked_count, mut truncated) =
        parse_status(&status);
    if let Some(merge_base) = merge_base.as_deref() {
        let committed = required_bytes(
            &repository_root,
            &[
                "diff",
                "--name-status",
                "-z",
                "--find-renames",
                merge_base,
                "HEAD",
                "--",
            ],
        )?;
        let (paths, paths_truncated) = parse_name_status(&committed);
        changed_paths.extend(paths);
        truncated |= paths_truncated;
    }
    changed_paths.sort_by(|left, right| {
        (&left.path, &left.source, &left.status).cmp(&(&right.path, &right.source, &right.status))
    });
    changed_paths.dedup();
    if changed_paths.len() > MAX_CHANGED_PATHS {
        changed_paths.truncate(MAX_CHANGED_PATHS);
        truncated = true;
    }

    let worktrees = parse_worktrees(&required_text(
        &repository_root,
        &["worktree", "list", "--porcelain"],
    )?);
    let remotes = parse_bounded_lines(&required_text(&repository_root, &["remote"])?, MAX_REMOTES);
    let local_branch_count = parse_bounded_lines(
        &required_text(
            &repository_root,
            &["for-each-ref", "--format=%(refname)", "refs/heads"],
        )?,
        usize::MAX,
    )
    .len() as u64;
    let head_parent_count = if head_commit.is_some() {
        optional_text(
            &repository_root,
            &["rev-list", "--parents", "-n", "1", "HEAD"],
        )?
        .map(|line| line.split_whitespace().count().saturating_sub(1) as u32)
    } else {
        None
    };
    let head_has_signature = if head_commit.is_some() {
        required_bytes(&repository_root, &["cat-file", "commit", "HEAD"])?
            .split(|byte| *byte == b'\n')
            .take_while(|line| !line.is_empty())
            .any(|line| line.starts_with(b"gpgsig ") || line.starts_with(b"gpgsig-sha256 "))
    } else {
        false
    };

    Ok(GitSnapshotDraft {
        workspace: workspace.to_string_lossy().into_owned(),
        repository_root: repository_root.to_string_lossy().into_owned(),
        head_commit,
        head_tree,
        branch,
        detached,
        upstream_ref,
        base_ref: selected_base,
        base_commit,
        merge_base,
        ahead_count,
        behind_count,
        dirty: staged_count + unstaged_count + untracked_count > 0,
        staged_count,
        unstaged_count,
        untracked_count,
        local_branch_count,
        head_parent_count,
        head_has_signature,
        paths_truncated: truncated,
        changed_paths,
        worktrees,
        remotes,
    })
}

pub(crate) fn assess_governance(
    draft: &GitSnapshotDraft,
    successful_receipt_bound: bool,
) -> Vec<GovernanceFinding> {
    let mut findings = Vec::new();
    if draft.head_commit.is_none() {
        finding(
            &mut findings,
            "unborn_head",
            GovernanceSeverity::Critical,
            "repository has no HEAD commit",
            true,
        );
    }
    if draft.detached {
        finding(
            &mut findings,
            "detached_head",
            GovernanceSeverity::Warning,
            "HEAD is detached from a local branch",
            true,
        );
    }
    if draft.dirty {
        finding(
            &mut findings,
            "dirty_worktree",
            GovernanceSeverity::Warning,
            &format!(
                "{} staged, {} unstaged, {} untracked paths observed",
                draft.staged_count, draft.unstaged_count, draft.untracked_count
            ),
            false,
        );
    }
    if draft.upstream_ref.is_none() {
        finding(
            &mut findings,
            "upstream_not_observed",
            GovernanceSeverity::Info,
            "no local upstream tracking ref was observed",
            false,
        );
    }
    if draft.base_ref.is_some() && draft.base_commit.is_none() {
        finding(
            &mut findings,
            "base_ref_unresolved",
            GovernanceSeverity::Critical,
            "the selected comparison ref did not resolve to a local commit",
            true,
        );
    } else if draft.base_commit.is_some()
        && draft.head_commit.is_some()
        && draft.merge_base.is_none()
    {
        finding(
            &mut findings,
            "merge_base_not_found",
            GovernanceSeverity::Critical,
            "HEAD and the selected base have no observed merge base",
            true,
        );
    }
    if draft.behind_count.unwrap_or_default() > 0 {
        finding(
            &mut findings,
            "behind_local_tracking_ref",
            GovernanceSeverity::Warning,
            &format!(
                "HEAD is {} commit(s) behind the locally cached tracking ref",
                draft.behind_count.unwrap_or_default()
            ),
            true,
        );
    }
    if draft.ahead_count.unwrap_or_default() > 0 && draft.behind_count.unwrap_or_default() > 0 {
        finding(
            &mut findings,
            "diverged_from_local_tracking_ref",
            GovernanceSeverity::Critical,
            "HEAD and its locally cached tracking ref have diverged",
            true,
        );
    }
    if !draft.head_has_signature && draft.head_commit.is_some() {
        finding(
            &mut findings,
            "head_signature_not_present",
            GovernanceSeverity::Info,
            "HEAD contains no embedded Git signature; signer trust was not evaluated",
            false,
        );
    }
    if !successful_receipt_bound && draft.head_commit.is_some() {
        finding(
            &mut findings,
            "no_successful_receipt_bound_to_head",
            GovernanceSeverity::Warning,
            "no successful Aporic execution receipt is bound to this HEAD commit",
            true,
        );
    }
    let sensitive = draft
        .changed_paths
        .iter()
        .filter(|change| change.governance_sensitive)
        .map(|change| change.path.as_str())
        .collect::<BTreeSet<_>>();
    if !sensitive.is_empty() {
        finding(
            &mut findings,
            "governance_sensitive_paths_changed",
            GovernanceSeverity::Warning,
            &format!(
                "sensitive paths changed: {}",
                sensitive.into_iter().collect::<Vec<_>>().join(", ")
            ),
            true,
        );
    }
    if draft.paths_truncated {
        finding(
            &mut findings,
            "changed_path_inventory_truncated",
            GovernanceSeverity::Warning,
            "changed-path inventory exceeded its deterministic bound",
            true,
        );
    }
    findings
}

fn run_git(workspace: &Path, args: &[&str]) -> Result<Output> {
    run_hardened_git(workspace, args, MAX_GIT_OUTPUT_BYTES).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidData {
            Error::Invalid("Git metadata output exceeded 2 MiB".to_owned())
        } else {
            Error::Io(error)
        }
    })
}

fn required_bytes(workspace: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = run_git(workspace, args)?;
    if !output.status.success() {
        return Err(Error::Invalid(format!(
            "Git metadata command failed: {}",
            bounded_error(&output.stderr)
        )));
    }
    Ok(output.stdout)
}

fn required_text(workspace: &Path, args: &[&str]) -> Result<String> {
    String::from_utf8(required_bytes(workspace, args)?)
        .map_err(|_| Error::Invalid("Git metadata was not valid UTF-8".to_owned()))
        .map(|text| text.trim().to_owned())
}

fn optional_text(workspace: &Path, args: &[&str]) -> Result<Option<String>> {
    let output = run_git(workspace, args)?;
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|_| Error::Invalid("Git metadata was not valid UTF-8".to_owned()))?;
    let text = text.trim();
    Ok((!text.is_empty()).then(|| text.to_owned()))
}

fn validate_ref(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 160
        || value.starts_with('-')
        || value.contains("..")
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || "-._/".contains(character)))
    {
        return Err(Error::Invalid(
            "base_ref is not a safe Git ref name".to_owned(),
        ));
    }
    Ok(())
}

fn select_base_ref(
    root: &Path,
    requested: Option<&str>,
    upstream: Option<&str>,
    branch: Option<&str>,
) -> Result<Option<String>> {
    if let Some(requested) = requested {
        return Ok(Some(requested.to_owned()));
    }
    if let Some(upstream) = upstream {
        return Ok(Some(upstream.to_owned()));
    }
    if branch != Some("main")
        && optional_text(root, &["rev-parse", "--verify", "refs/heads/main^{commit}"])?.is_some()
    {
        return Ok(Some("refs/heads/main".to_owned()));
    }
    Ok(None)
}

fn parse_ahead_behind(value: &str) -> Result<(Option<u64>, Option<u64>)> {
    let mut fields = value.split_whitespace();
    let ahead = fields.next().and_then(|field| field.parse::<u64>().ok());
    let behind = fields.next().and_then(|field| field.parse::<u64>().ok());
    if ahead.is_none() || behind.is_none() || fields.next().is_some() {
        return Err(Error::Invalid(
            "invalid Git ahead/behind metadata".to_owned(),
        ));
    }
    Ok((ahead, behind))
}

fn parse_status(bytes: &[u8]) -> (Vec<GitPathChange>, u64, u64, u64, bool) {
    let fields = bytes.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut changes = Vec::new();
    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked = 0;
    let mut index = 0;
    let mut truncated = false;
    while index < fields.len() && !fields[index].is_empty() {
        let field = fields[index];
        index += 1;
        if field.len() < 4 {
            continue;
        }
        let x = field[0] as char;
        let y = field[1] as char;
        if x == '?' && y == '?' {
            untracked += 1;
        } else {
            staged += u64::from(x != ' ');
            unstaged += u64::from(y != ' ');
        }
        let path = bounded_path(&field[3..]);
        let renamed = matches!(x, 'R' | 'C') || matches!(y, 'R' | 'C');
        let previous_path = if renamed && index < fields.len() {
            let previous = bounded_path(fields[index]);
            index += 1;
            Some(previous)
        } else {
            None
        };
        if changes.len() < MAX_CHANGED_PATHS {
            changes.push(path_change(
                "worktree",
                &format!("{x}{y}"),
                path,
                previous_path,
            ));
        } else {
            truncated = true;
        }
    }
    (changes, staged, unstaged, untracked, truncated)
}

fn parse_name_status(bytes: &[u8]) -> (Vec<GitPathChange>, bool) {
    let fields = bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    let mut changes = Vec::new();
    let mut index = 0;
    let mut truncated = false;
    while index < fields.len() {
        let status = String::from_utf8_lossy(fields[index]).into_owned();
        index += 1;
        if index >= fields.len() {
            break;
        }
        let first = bounded_path(fields[index]);
        index += 1;
        let (path, previous_path) = if status.starts_with('R') || status.starts_with('C') {
            if index >= fields.len() {
                break;
            }
            let current = bounded_path(fields[index]);
            index += 1;
            (current, Some(first))
        } else {
            (first, None)
        };
        if changes.len() < MAX_CHANGED_PATHS {
            changes.push(path_change("head_vs_base", &status, path, previous_path));
        } else {
            truncated = true;
        }
    }
    (changes, truncated)
}

fn path_change(
    source: &str,
    status: &str,
    path: String,
    previous_path: Option<String>,
) -> GitPathChange {
    let governance_sensitive = is_governance_sensitive(&path)
        || previous_path
            .as_deref()
            .is_some_and(is_governance_sensitive);
    GitPathChange {
        source: source.to_owned(),
        status: status.chars().take(16).collect(),
        path,
        previous_path,
        governance_sensitive,
    }
}

fn bounded_path(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).chars().take(1_024).collect()
}

fn is_governance_sensitive(path: &str) -> bool {
    let normalized = path.trim_start_matches("./");
    normalized == "SECURITY.md"
        || normalized == "CODEOWNERS"
        || normalized.ends_with("/CODEOWNERS")
        || normalized == "Cargo.lock"
        || normalized == "AGENTS.md"
        || normalized.ends_with("/AGENTS.md")
        || normalized.starts_with(".github/")
        || normalized.starts_with(".codex/")
        || normalized.starts_with("migrations/")
}

fn parse_worktrees(value: &str) -> Vec<GitWorktreeObservation> {
    let mut observations = Vec::new();
    let mut head_commit = None;
    let mut branch = None;
    let mut detached = false;
    let mut locked = false;
    let mut prunable = false;
    for line in value.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if head_commit.is_some() || branch.is_some() || detached {
                if observations.len() < MAX_WORKTREES {
                    observations.push(GitWorktreeObservation {
                        head_commit: head_commit.take(),
                        branch: branch.take(),
                        detached,
                        locked,
                        prunable,
                    });
                }
                detached = false;
                locked = false;
                prunable = false;
            }
        } else if let Some(value) = line.strip_prefix("HEAD ") {
            head_commit = Some(value.chars().take(64).collect());
        } else if let Some(value) = line.strip_prefix("branch ") {
            branch = Some(value.chars().take(256).collect());
        } else if line == "detached" {
            detached = true;
        } else if line.starts_with("locked") {
            locked = true;
        } else if line.starts_with("prunable") {
            prunable = true;
        }
    }
    observations
}

fn parse_bounded_lines(value: &str, limit: usize) -> Vec<String> {
    value
        .lines()
        .filter(|line| !line.is_empty())
        .take(limit)
        .map(|line| line.chars().take(256).collect())
        .collect()
}

fn bounded_error(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|character| !character.is_control() || *character == '\n')
        .take(512)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn finding(
    findings: &mut Vec<GovernanceFinding>,
    code: &str,
    severity: GovernanceSeverity,
    evidence: &str,
    requires_independent_review: bool,
) {
    findings.push(GovernanceFinding {
        code: code.to_owned(),
        severity,
        evidence: evidence.to_owned(),
        requires_independent_review,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_option_shaped_and_revision_expression_refs() {
        assert!(validate_ref("main").is_ok());
        assert!(validate_ref("refs/remotes/origin/main").is_ok());
        assert!(validate_ref("--upload-pack=evil").is_err());
        assert!(validate_ref("main..evil").is_err());
        assert!(validate_ref("main^{tree}").is_err());
    }

    #[test]
    fn sensitive_paths_are_explicit_and_bounded() {
        assert!(is_governance_sensitive(".github/workflows/check.yml"));
        assert!(is_governance_sensitive("migrations/0009.sql"));
        assert!(!is_governance_sensitive("src/lib.rs"));
    }
}
