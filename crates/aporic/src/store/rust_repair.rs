//! Rust repair examples are task-scoped observations, not executable advice.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    Error, Result, Store, append_event, canonical_workspace, duplicate_result, load_task,
    require_open_session, require_text, unix_millis, validate_relative_path,
};
use crate::bounded::sha256_file_bounded;
use crate::domain::{
    CommandSpec, RustRepairCase, RustRepairLearnOutcome, RustRepairLearnRequest, RustRepairLesson,
    RustRepairOpenOutcome, RustRepairOpenRequest, RustRepairSearchRequest, RustRepairSearchResult,
    RustcInputSnapshot, TaskStatus,
};

const OPEN_EVENT: &str = "rust_repair_opened";
const LEARN_EVENT: &str = "rust_repair_learned";
const NOTICE: &str = "The local runner recorded an exit-zero process named cargo with exact test arguments for one workspace state. Executable authenticity, test adequacy, and the proposed causal rule are not established by this receipt; lessons remain advisory.";

pub(super) fn capture_rustc_input(
    transaction: &Transaction<'_>,
    spec: &CommandSpec,
    run_id: &str,
    now: i64,
) -> Result<Option<RustcInputSnapshot>> {
    let program_name = Path::new(&spec.program)
        .file_name()
        .and_then(|name| name.to_str());
    if !matches!(program_name, Some("rustc" | "rustc.exe"))
        || spec.workspace_relative_cwd != "."
        || spec.args.len() != 4
        || !spec.args[0].starts_with("--edition=")
        || spec.args[1] != "--crate-type=lib"
        || spec.args[2] != "--error-format=json"
    {
        return Ok(None);
    }
    let source = &spec.args[3];
    if validate_relative_path("rustc source", source).is_err() {
        return Ok(None);
    }
    let Ok(path) = fs::canonicalize(Path::new(&spec.workspace).join(source)) else {
        return Ok(None);
    };
    if !path.starts_with(&spec.workspace) {
        return Ok(None);
    }
    let Ok((sha256, _)) = sha256_file_bounded(&path, 1_000_000, "rustc source") else {
        return Ok(None);
    };
    let snapshot = RustcInputSnapshot {
        run_id: run_id.into(),
        workspace_relative_path: source.clone(),
        sha256,
        captured_at_unix_ms: now,
    };
    transaction.execute(
        "INSERT INTO rustc_input_snapshots (run_id, workspace_relative_path, sha256, captured_at_unix_ms) VALUES (?1, ?2, ?3, ?4)",
        params![snapshot.run_id, snapshot.workspace_relative_path, snapshot.sha256, now],
    )?;
    Ok(Some(snapshot))
}

pub(super) fn export_input_snapshots(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<RustcInputSnapshot>> {
    let mut stmt = connection.prepare(
        "SELECT i.run_id, i.workspace_relative_path, i.sha256, i.captured_at_unix_ms FROM rustc_input_snapshots i JOIN execution_runs r ON r.run_id = i.run_id JOIN sessions s ON s.session_id = r.session_id WHERE s.project_id = ?1 ORDER BY i.captured_at_unix_ms, i.run_id",
    )?;
    stmt.query_map([project_id], |row| {
        Ok(RustcInputSnapshot {
            run_id: row.get(0)?,
            workspace_relative_path: row.get(1)?,
            sha256: row.get(2)?,
            captured_at_unix_ms: row.get(3)?,
        })
    })?
    .collect::<std::result::Result<Vec<_>, _>>()
    .map_err(Into::into)
}

impl Store {
    pub fn open_rust_repair(
        &self,
        request: &RustRepairOpenRequest,
    ) -> Result<RustRepairOpenOutcome> {
        for (name, value) in [
            ("session_id", &request.session_id),
            ("task_id", &request.task_id),
            ("rustc_version", &request.rustc_version),
            ("diagnostic_evidence_id", &request.diagnostic_evidence_id),
            (
                "reproduction_evidence_id",
                &request.reproduction_evidence_id,
            ),
            ("reproduction_run_id", &request.reproduction_run_id),
            ("idempotency_key", &request.idempotency_key),
        ] {
            require_text(name, value)?;
        }
        validate_query(
            &request.diagnostic_code,
            &request.rustc_version,
            &request.edition,
        )?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<RustRepairOpenOutcome, _>(
            &tx,
            &request.idempotency_key,
            OPEN_EVENT,
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&tx, &request.session_id)?;
        let project_id: String = tx.query_row(
            "SELECT project_id FROM sessions WHERE session_id = ?1",
            [&request.session_id],
            |row| row.get(0),
        )?;
        let task = load_task(&tx, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        if task.project_id != project_id {
            return Err(Error::Conflict(
                "repair task belongs to another project".into(),
            ));
        }
        if matches!(task.status, TaskStatus::Cancelled | TaskStatus::Completed) {
            return Err(Error::Conflict(
                "repair task must be active at intake".into(),
            ));
        }
        check_evidence(
            &tx,
            &project_id,
            &request.diagnostic_evidence_id,
            &["workspace_file"],
        )?;
        check_evidence(
            &tx,
            &project_id,
            &request.reproduction_evidence_id,
            &["workspace_file"],
        )?;
        let (diagnostic, _, diagnostic_sha) =
            read_direct_file(&tx, &request.diagnostic_evidence_id)?;
        let (_, reproduction_path, reproduction_sha) =
            read_direct_file(&tx, &request.reproduction_evidence_id)?;
        let matching_code = diagnostic.lines().any(|line| {
            serde_json::from_str::<serde_json::Value>(line).is_ok_and(|value| {
                value["level"] == "error" && value["code"]["code"] == request.diagnostic_code
            })
        });
        if !matching_code {
            return Err(Error::Invalid(
                "diagnostic file has no matching rustc JSON error code".into(),
            ));
        }
        check_reproduction_run(
            &tx,
            &project_id,
            request,
            &reproduction_path,
            &reproduction_sha,
            &diagnostic_sha,
        )?;
        if let Some(id) = &request.reference_evidence_id {
            check_evidence(&tx, &project_id, id, &["external_source"])?;
        }
        let case_id = Uuid::now_v7().to_string();
        let now = unix_millis()?;
        tx.execute(
            "INSERT INTO rust_repair_cases (case_id, project_id, task_id, diagnostic_code, rustc_version, edition, diagnostic_evidence_id, reproduction_evidence_id, reproduction_run_id, reference_evidence_id, created_at_unix_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![case_id, project_id, request.task_id, request.diagnostic_code, request.rustc_version, request.edition, request.diagnostic_evidence_id, request.reproduction_evidence_id, request.reproduction_run_id, request.reference_evidence_id, now],
        )?;
        let outcome = RustRepairOpenOutcome {
            case: load_case(&tx, &case_id)?
                .ok_or_else(|| Error::NotFound(format!("rust repair case {case_id}")))?,
            duplicate: false,
        };
        append_event(
            &tx,
            &request.idempotency_key,
            &case_id,
            OPEN_EVENT,
            request,
            &outcome,
            now,
        )?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn learn_rust_repair(
        &self,
        request: &RustRepairLearnRequest,
    ) -> Result<RustRepairLearnOutcome> {
        for (name, value) in [
            ("case_id", &request.case_id),
            ("rule", &request.rule),
            ("applicability", &request.applicability),
            ("counterexample", &request.counterexample),
            ("test_run_id", &request.test_run_id),
            ("idempotency_key", &request.idempotency_key),
        ] {
            require_text(name, value)?;
        }
        for (name, value) in [
            ("rule", &request.rule),
            ("applicability", &request.applicability),
            ("counterexample", &request.counterexample),
        ] {
            if value.len() > 2_000 {
                return Err(Error::Invalid(format!("{name} exceeds 2000 bytes")));
            }
        }
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<RustRepairLearnOutcome, _>(
            &tx,
            &request.idempotency_key,
            LEARN_EVENT,
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let case = load_case(&tx, &request.case_id)?
            .ok_or_else(|| Error::NotFound(format!("rust repair case {}", request.case_id)))?;
        if load_lesson(&tx, &request.case_id)?.is_some() {
            return Err(Error::Conflict(
                "rust repair case already has a lesson".into(),
            ));
        }
        let task = load_task(&tx, &case.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", case.task_id)))?;
        if task.status != TaskStatus::Completed || task.completion_proofs.is_empty() {
            return Err(Error::Conflict(
                "lesson requires a completed repair task with verified proofs".into(),
            ));
        }
        let (status, program, args_json, cwd, expected_exit, actual_exit, resolved, workspace): (String, String, String, String, i32, Option<i32>, Option<String>, String) = tx.query_row(
            "SELECT r.status, s.program, s.args_json, s.workspace_relative_cwd, s.expected_exit_code, e.exit_code, e.resolved_executable, p.workspace FROM execution_runs r JOIN verification_specs s ON s.spec_id = r.spec_id JOIN execution_receipts e ON e.run_id = r.run_id JOIN sessions ss ON ss.session_id = r.session_id JOIN rust_repair_cases c ON c.case_id = ?2 JOIN projects p ON p.project_id = c.project_id WHERE r.run_id = ?1 AND ss.project_id = c.project_id",
            params![request.test_run_id, request.case_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
        ).optional()?.ok_or_else(|| Error::NotFound(format!("project-scoped test run {}", request.test_run_id)))?;
        let args: Vec<String> = serde_json::from_str(&args_json)?;
        let case_sequence = event_sequence(&tx, "rust_repair_opened", &case.case_id)?;
        let test_sequence = event_sequence(&tx, "execution_run_started", &request.test_run_id)?;
        if status != "succeeded"
            || expected_exit != 0
            || actual_exit != Some(0)
            || cwd != "."
            || args != ["test", "--offline"]
            || test_sequence <= case_sequence
            || !non_workspace_command_path(&program, resolved.as_deref(), "cargo", &workspace)
        {
            return Err(Error::Conflict(
                "lesson requires a successful post-intake cargo test receipt".into(),
            ));
        }
        let mut bound_to_proof = false;
        for proof in &task.completion_proofs {
            if tx.query_row(
                "SELECT 1 FROM claim_receipts cr JOIN execution_receipts e ON e.receipt_id = cr.receipt_id WHERE cr.claim_id = ?1 AND e.run_id = ?2",
                params![proof.verified_claim_id, request.test_run_id], |row| row.get::<_, i32>(0),
            ).optional()?.is_some() {
                bound_to_proof = true;
                break;
            }
        }
        if !bound_to_proof {
            return Err(Error::Conflict(
                "cargo test receipt must back a repair task criterion proof".into(),
            ));
        }
        let now = unix_millis()?;
        tx.execute(
            "INSERT INTO rust_repair_lessons (case_id, rule, applicability, counterexample, test_run_id, learned_at_unix_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![request.case_id, request.rule, request.applicability, request.counterexample, request.test_run_id, now],
        )?;
        let outcome = RustRepairLearnOutcome {
            lesson: load_lesson(&tx, &request.case_id)?.ok_or_else(|| {
                Error::NotFound(format!("rust repair lesson {}", request.case_id))
            })?,
            duplicate: false,
        };
        append_event(
            &tx,
            &request.idempotency_key,
            &request.case_id,
            LEARN_EVENT,
            request,
            &outcome,
            now,
        )?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn search_rust_repairs(
        &self,
        request: &RustRepairSearchRequest,
    ) -> Result<RustRepairSearchResult> {
        validate_query(
            &request.diagnostic_code,
            &request.rustc_version,
            &request.edition,
        )?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id: Option<String> = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get(0),
            )
            .optional()?;
        let mut lessons = Vec::new();
        if let Some(project_id) = project_id {
            let mut stmt = connection.prepare(
                "SELECT c.case_id FROM rust_repair_cases c JOIN rust_repair_lessons l ON l.case_id = c.case_id WHERE c.project_id = ?1 AND c.diagnostic_code = ?2 AND c.rustc_version = ?3 AND c.edition = ?4 ORDER BY l.learned_at_unix_ms DESC, c.case_id DESC LIMIT ?5",
            )?;
            let ids = stmt.query_map(
                params![
                    project_id,
                    request.diagnostic_code,
                    request.rustc_version,
                    request.edition,
                    i64::from(request.limit.unwrap_or(10).clamp(1, 50))
                ],
                |row| row.get::<_, String>(0),
            )?;
            for id in ids {
                lessons.push(
                    load_lesson(&connection, &id?)?
                        .ok_or_else(|| Error::NotFound("rust repair lesson".into()))?,
                );
            }
        }
        Ok(RustRepairSearchResult {
            lessons,
            advisory: true,
            notice: NOTICE.into(),
        })
    }
}

fn validate_query(code: &str, version: &str, edition: &str) -> Result<()> {
    if code.len() != 5
        || !code.starts_with('E')
        || !code[1..].bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(Error::Invalid(
            "diagnostic_code must be a rustc E followed by four digits".into(),
        ));
    }
    require_text("rustc_version", version)?;
    if version.len() > 128 || !version.starts_with("rustc ") {
        return Err(Error::Invalid(
            "rustc_version must be a bounded rustc --version string".into(),
        ));
    }
    if !matches!(edition, "2015" | "2018" | "2021" | "2024") {
        return Err(Error::Invalid("unsupported Rust edition".into()));
    }
    Ok(())
}

fn check_evidence(
    connection: &Connection,
    project_id: &str,
    id: &str,
    allowed: &[&str],
) -> Result<()> {
    require_text("evidence_id", id)?;
    let found: (String, String) = connection.query_row(
        "SELECT s.project_id, e.kind FROM evidence_artifacts e JOIN sessions s ON s.session_id = e.session_id WHERE e.evidence_id = ?1",
        [id], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional()?.ok_or_else(|| Error::NotFound(format!("evidence {id}")))?;
    if found.0 != project_id {
        return Err(Error::Conflict(
            "Rust repair evidence belongs to another project".into(),
        ));
    }
    if !allowed.contains(&found.1.as_str()) {
        return Err(Error::Invalid(
            "Rust repair evidence has the wrong kind".into(),
        ));
    }
    Ok(())
}

fn check_reproduction_run(
    connection: &Connection,
    project_id: &str,
    request: &RustRepairOpenRequest,
    reproduction_path: &Path,
    reproduction_sha: &str,
    diagnostic_sha: &str,
) -> Result<()> {
    let row: (String, String, String, String, i32, Option<i32>, String, Option<String>, String) = connection.query_row(
        "SELECT r.status, s.program, s.args_json, s.workspace_relative_cwd, s.expected_exit_code, e.exit_code, e.stderr_sha256, e.resolved_executable, p.workspace FROM execution_runs r JOIN verification_specs s ON s.spec_id = r.spec_id JOIN execution_receipts e ON e.run_id = r.run_id JOIN sessions ss ON ss.session_id = r.session_id JOIN projects p ON p.project_id = ss.project_id WHERE r.run_id = ?1 AND p.project_id = ?2",
        params![request.reproduction_run_id, project_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?)),
    ).optional()?.ok_or_else(|| Error::NotFound(format!("project-scoped reproduction run {}", request.reproduction_run_id)))?;
    let (
        status,
        program,
        args_json,
        cwd,
        expected_exit,
        actual_exit,
        stderr_sha,
        resolved,
        workspace,
    ) = row;
    let evidence_sequence: i64 = connection.query_row(
        "SELECT sequence FROM events WHERE kind = 'evidence_added' AND json_extract(result_json, '$.evidence.evidence_id') = ?1 LIMIT 1",
        [&request.reproduction_evidence_id],
        |row| row.get(0),
    ).optional()?.ok_or_else(|| Error::Conflict("reproduction evidence has no append-only origin event".into()))?;
    let run_sequence = event_sequence(
        connection,
        "execution_run_started",
        &request.reproduction_run_id,
    )?;
    let snapshot: (String, String) = connection
        .query_row(
            "SELECT workspace_relative_path, sha256 FROM rustc_input_snapshots WHERE run_id = ?1",
            [&request.reproduction_run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| Error::Conflict("reproduction run has no captured input hash".into()))?;
    let args: Vec<String> = serde_json::from_str(&args_json)?;
    let expected_args = [
        format!("--edition={}", request.edition),
        "--crate-type=lib".into(),
        "--error-format=json".into(),
    ];
    let target = args
        .get(3)
        .ok_or_else(|| Error::Conflict("rustc reproduction needs one source file".into()))?;
    let source_path = fs::canonicalize(Path::new(&workspace).join(target))?;
    if status != "succeeded"
        || expected_exit != 1
        || actual_exit != Some(1)
        || cwd != "."
        || args.len() != 4
        || args[..3] != expected_args
        || source_path != reproduction_path
        || snapshot.0 != *target
        || snapshot.1 != reproduction_sha
        || evidence_sequence >= run_sequence
        || stderr_sha != diagnostic_sha
        || !non_workspace_command_path(&program, resolved.as_deref(), "rustc", &workspace)
    {
        return Err(Error::Conflict(
            "reproduction needs a matching local-runner rustc failure receipt and stderr digest"
                .into(),
        ));
    }
    Ok(())
}

fn non_workspace_command_path(
    program: &str,
    resolved: Option<&str>,
    command: &str,
    workspace: &str,
) -> bool {
    let expected_exe = format!("{command}.exe");
    let program_name = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str());
    let Some(resolved) = resolved else {
        return false;
    };
    let resolved_path = Path::new(resolved);
    matches!(program_name, Some(name) if name == command || name == expected_exe)
        && resolved_path.is_absolute()
        && !resolved_path.starts_with(workspace)
}

fn event_sequence(connection: &Connection, kind: &str, stream_id: &str) -> Result<i64> {
    connection
        .query_row(
            "SELECT sequence FROM events WHERE kind = ?1 AND stream_id = ?2 LIMIT 1",
            params![kind, stream_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::Conflict(format!("missing {kind} origin event for {stream_id}")))
}

fn read_direct_file(
    connection: &Connection,
    evidence_id: &str,
) -> Result<(String, PathBuf, String)> {
    let (locator, expected_digest, grade, workspace): (String, String, String, String) = connection.query_row(
        "SELECT COALESCE(e.canonical_locator, e.locator), e.content_sha256, e.grade, p.workspace FROM evidence_artifacts e JOIN sessions s ON s.session_id = e.session_id JOIN projects p ON p.project_id = s.project_id WHERE e.evidence_id = ?1",
        [evidence_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    if grade != "direct" {
        return Err(Error::Conflict(
            "Rust repair file evidence must be direct".into(),
        ));
    }
    let path = fs::canonicalize(&locator)?;
    if !path.starts_with(&workspace) {
        return Err(Error::Conflict(
            "Rust repair evidence file is outside the project workspace".into(),
        ));
    }
    let file = fs::File::open(&path)?;
    if !file.metadata()?.is_file() {
        return Err(Error::Invalid(
            "Rust repair evidence must be a regular file".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(1_000_001).read_to_end(&mut bytes)?;
    if bytes.len() > 1_000_000 {
        return Err(Error::Invalid(
            "Rust repair file exceeds 1000000 bytes".into(),
        ));
    }
    if format!("{:x}", Sha256::digest(&bytes)) != expected_digest || bytes.is_empty() {
        return Err(Error::Conflict(
            "Rust repair evidence file changed or is empty".into(),
        ));
    }
    Ok((
        String::from_utf8(bytes)
            .map_err(|_| Error::Invalid("Rust repair file must be UTF-8".into()))?,
        path,
        expected_digest,
    ))
}

fn load_case(connection: &Connection, id: &str) -> Result<Option<RustRepairCase>> {
    connection.query_row(
        "SELECT case_id, task_id, diagnostic_code, rustc_version, edition, diagnostic_evidence_id, reproduction_evidence_id, reproduction_run_id, reference_evidence_id, created_at_unix_ms FROM rust_repair_cases WHERE case_id = ?1",
        [id], |row| Ok(RustRepairCase {
            case_id: row.get(0)?, task_id: row.get(1)?, diagnostic_code: row.get(2)?,
            rustc_version: row.get(3)?, edition: row.get(4)?, diagnostic_evidence_id: row.get(5)?,
            reproduction_evidence_id: row.get(6)?, reproduction_run_id: row.get(7)?,
            reference_evidence_id: row.get(8)?, created_at_unix_ms: row.get(9)?,
        }),
    ).optional().map_err(Into::into)
}

fn load_lesson(connection: &Connection, id: &str) -> Result<Option<RustRepairLesson>> {
    let row = connection.query_row(
        "SELECT rule, applicability, counterexample, test_run_id, learned_at_unix_ms FROM rust_repair_lessons WHERE case_id = ?1",
        [id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4)?)),
    ).optional()?;
    row.map(
        |(rule, applicability, counterexample, test_run_id, learned_at_unix_ms)| {
            Ok(RustRepairLesson {
                case: load_case(connection, id)?
                    .ok_or_else(|| Error::NotFound(format!("rust repair case {id}")))?,
                rule,
                applicability,
                counterexample,
                test_run_id,
                learned_at_unix_ms,
            })
        },
    )
    .transpose()
}

pub(super) fn export_rust_repairs(
    connection: &Connection,
    project_id: &str,
) -> Result<(Vec<RustRepairCase>, Vec<RustRepairLesson>)> {
    let mut stmt = connection.prepare(
        "SELECT case_id FROM rust_repair_cases WHERE project_id = ?1 ORDER BY created_at_unix_ms, case_id",
    )?;
    let ids = stmt
        .query_map([project_id], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut cases = Vec::with_capacity(ids.len());
    let mut lessons = Vec::new();
    for id in ids {
        cases.push(
            load_case(connection, &id)?
                .ok_or_else(|| Error::NotFound(format!("rust repair case {id}")))?,
        );
        if let Some(lesson) = load_lesson(connection, &id)? {
            lessons.push(lesson);
        }
    }
    Ok((cases, lessons))
}
