//! Product-wide traceability above individual advisory tasks.
//! Event results are the replayable state; the table only indexes project ownership.

use std::{collections::BTreeSet, fs, path::Path};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use uuid::Uuid;

use super::{
    Error, Result, Store, append_event, canonical_workspace, duplicate_result, load_task,
    require_open_session, require_text, unix_millis,
};
use crate::bounded::{MAX_EVIDENCE_FILE_BYTES, sha256_file_bounded};
use crate::domain::{
    EvidenceGrade, EvidenceKind, InitiativeArtifact, InitiativeArtifactKind,
    InitiativeArtifactRequest, InitiativeOutcome, InitiativePlan, InitiativePlanRequest,
    InitiativeStatus, InitiativeStatusRequest, InitiativeTaskLink, InitiativeTaskLinkRequest,
    TaskStatus, WorkflowStage,
};

const PLAN_EVENT: &str = "initiative_planned";
const ARTIFACT_EVENT: &str = "initiative_artifact_recorded";
const LINK_EVENT: &str = "initiative_task_linked";
const MAX_EVENTS: i64 = 512;

impl Store {
    pub fn plan_initiative(&self, request: &InitiativePlanRequest) -> Result<InitiativeOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        for (name, value) in [
            ("objective", &request.objective),
            ("target_user", &request.target_user),
            ("success_measure", &request.success_measure),
        ] {
            require_text(name, value)?;
            if value.len() > 2_000 {
                return Err(Error::Invalid(format!("{name} exceeds 2000 bytes")));
            }
        }
        if request.requirements.is_empty() || request.requirements.len() > 64 {
            return Err(Error::Invalid(
                "initiative needs 1 to 64 requirements".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        for requirement in &request.requirements {
            for (name, value) in [
                ("requirement_id", &requirement.requirement_id),
                ("statement", &requirement.statement),
                ("acceptance_criterion", &requirement.acceptance_criterion),
            ] {
                require_text(name, value)?;
                if value.len() > 2_000 {
                    return Err(Error::Invalid(format!("{name} exceeds 2000 bytes")));
                }
            }
            if !ids.insert(&requirement.requirement_id) {
                return Err(Error::Invalid("duplicate requirement_id".into()));
            }
        }
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<InitiativeOutcome, _>(
            &tx,
            &request.idempotency_key,
            PLAN_EVENT,
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
        let initiative_id = if let Some(id) = &request.initiative_id {
            require_owner(&tx, id, &project_id)?;
            let previous = read_status(&tx, id)?;
            let evidence_id = request.revision_evidence_id.as_deref().ok_or_else(|| {
                Error::Conflict(
                    "initiative revision requires reported user-statement evidence".into(),
                )
            })?;
            check_evidence(
                &tx,
                &project_id,
                evidence_id,
                EvidenceKind::UserStatement,
                EvidenceGrade::Reported,
                previous.plan.created_at_unix_ms,
            )?;
            id.clone()
        } else {
            if request.revision_evidence_id.is_some() {
                return Err(Error::Invalid(
                    "revision_evidence_id is only for revisions".into(),
                ));
            }
            let id = Uuid::now_v7().to_string();
            tx.execute(
                "INSERT INTO initiatives (initiative_id, project_id, created_at_unix_ms) VALUES (?1, ?2, ?3)",
                params![id, project_id, unix_millis()?],
            )?;
            id
        };
        require_capacity(&tx, &initiative_id)?;
        let now = unix_millis()?;
        let mut status = InitiativeStatus {
            initiative_id: initiative_id.clone(),
            plan: InitiativePlan {
                revision_id: Uuid::now_v7().to_string(),
                created_at_unix_ms: now,
                objective: request.objective.clone(),
                target_user: request.target_user.clone(),
                success_measure: request.success_measure.clone(),
                requirements: request.requirements.clone(),
            },
            artifacts: Vec::new(),
            task_links: Vec::new(),
            missing: Vec::new(),
            ready_to_claim: false,
            advisory: true,
        };
        refresh(&tx, &mut status)?;
        let outcome = InitiativeOutcome {
            status,
            duplicate: false,
        };
        append_event(
            &tx,
            &request.idempotency_key,
            &initiative_id,
            PLAN_EVENT,
            request,
            &outcome,
            now,
        )?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn record_initiative_artifact(
        &self,
        request: &InitiativeArtifactRequest,
    ) -> Result<InitiativeOutcome> {
        require_text("idempotency_key", &request.idempotency_key)?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<InitiativeOutcome, _>(
            &tx,
            &request.idempotency_key,
            ARTIFACT_EVENT,
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = session_project(&tx, &request.session_id)?;
        require_owner(&tx, &request.initiative_id, &project_id)?;
        require_capacity(&tx, &request.initiative_id)?;
        let mut status = read_status(&tx, &request.initiative_id)?;
        let evidence_time = check_evidence(
            &tx,
            &project_id,
            &request.evidence_id,
            EvidenceKind::WorkspaceFile,
            EvidenceGrade::Direct,
            status.plan.created_at_unix_ms,
        )?;
        apply_artifact(
            &mut status,
            request.kind,
            &request.evidence_id,
            evidence_time,
        );
        refresh(&tx, &mut status)?;
        let outcome = InitiativeOutcome {
            status,
            duplicate: false,
        };
        append_event(
            &tx,
            &request.idempotency_key,
            &request.initiative_id,
            ARTIFACT_EVENT,
            request,
            &outcome,
            unix_millis()?,
        )?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn link_initiative_task(
        &self,
        request: &InitiativeTaskLinkRequest,
    ) -> Result<InitiativeOutcome> {
        require_text("idempotency_key", &request.idempotency_key)?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<InitiativeOutcome, _>(
            &tx,
            &request.idempotency_key,
            LINK_EVENT,
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = session_project(&tx, &request.session_id)?;
        require_owner(&tx, &request.initiative_id, &project_id)?;
        require_capacity(&tx, &request.initiative_id)?;
        let mut status = read_status(&tx, &request.initiative_id)?;
        let requirement = status
            .plan
            .requirements
            .iter()
            .find(|item| item.requirement_id == request.requirement_id)
            .ok_or_else(|| Error::NotFound(format!("requirement {}", request.requirement_id)))?;
        let task = load_task(&tx, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        if task.project_id != project_id {
            return Err(Error::Conflict(
                "initiative and task must belong to one project".into(),
            ));
        }
        if !task
            .acceptance_criteria
            .contains(&requirement.acceptance_criterion)
        {
            return Err(Error::Conflict(
                "task must contain the requirement's exact acceptance criterion".into(),
            ));
        }
        apply_link(&mut status, &request.requirement_id, &request.task_id)?;
        refresh(&tx, &mut status)?;
        let outcome = InitiativeOutcome {
            status,
            duplicate: false,
        };
        append_event(
            &tx,
            &request.idempotency_key,
            &request.initiative_id,
            LINK_EVENT,
            request,
            &outcome,
            unix_millis()?,
        )?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn initiative_status(&self, request: &InitiativeStatusRequest) -> Result<InitiativeStatus> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id: String = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [workspace],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound("workspace project".into()))?;
        require_owner(&connection, &request.initiative_id, &project_id)?;
        let mut status = read_status(&connection, &request.initiative_id)?;
        refresh(&connection, &mut status)?;
        Ok(status)
    }
}

fn session_project(connection: &Transaction<'_>, session_id: &str) -> Result<String> {
    require_open_session(connection, session_id)?;
    Ok(connection.query_row(
        "SELECT project_id FROM sessions WHERE session_id = ?1",
        [session_id],
        |row| row.get(0),
    )?)
}

fn require_owner(connection: &Connection, initiative_id: &str, project_id: &str) -> Result<()> {
    let actual: Option<String> = connection
        .query_row(
            "SELECT project_id FROM initiatives WHERE initiative_id = ?1",
            [initiative_id],
            |row| row.get(0),
        )
        .optional()?;
    match actual {
        Some(actual) if actual == project_id => Ok(()),
        Some(_) => Err(Error::Conflict(
            "initiative belongs to another project".into(),
        )),
        None => Err(Error::NotFound(format!("initiative {initiative_id}"))),
    }
}

fn check_evidence(
    connection: &Connection,
    project_id: &str,
    evidence_id: &str,
    kind: EvidenceKind,
    grade: EvidenceGrade,
    not_before: i64,
) -> Result<i64> {
    let found: Option<(String, String, String, i64)> = connection
        .query_row(
            "SELECT s.project_id, e.kind, e.grade, e.created_at_unix_ms
         FROM evidence_artifacts e JOIN sessions s ON s.session_id = e.session_id
         WHERE e.evidence_id = ?1",
            [evidence_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    match found {
        Some((project, actual_kind, actual_grade, created))
            if project == project_id
                && actual_kind == kind.as_str()
                && actual_grade == grade.as_str()
                && created >= not_before =>
        {
            Ok(created)
        }
        _ => Err(Error::Conflict(
            "evidence must have the required kind, grade, project and revision time".into(),
        )),
    }
}

fn require_capacity(connection: &Connection, initiative_id: &str) -> Result<()> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM events WHERE stream_id = ?1
         AND kind IN (?2, ?3, ?4)",
        params![initiative_id, PLAN_EVENT, ARTIFACT_EVENT, LINK_EVENT],
        |row| row.get(0),
    )?;
    if count >= MAX_EVENTS {
        return Err(Error::Invalid("initiative event limit reached".into()));
    }
    Ok(())
}

fn read_status(connection: &Connection, initiative_id: &str) -> Result<InitiativeStatus> {
    let project_id: String = connection.query_row(
        "SELECT project_id FROM initiatives WHERE initiative_id = ?1",
        [initiative_id],
        |row| row.get(0),
    )?;
    let mut statement = connection.prepare(
        "SELECT kind, payload_json, result_json, occurred_at_unix_ms FROM events
         WHERE stream_id = ?1 AND kind IN (?2, ?3, ?4) ORDER BY sequence ASC",
    )?;
    let rows = statement
        .query_map(
            params![initiative_id, PLAN_EVENT, ARTIFACT_EVENT, LINK_EVENT],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut current = None::<InitiativeStatus>;
    for (kind, payload, result, occurred) in rows {
        let snapshot = serde_json::from_str::<InitiativeOutcome>(&result)?.status;
        if snapshot.initiative_id != initiative_id {
            return Err(Error::Conflict(
                "initiative event has a different stream identity".into(),
            ));
        }
        match kind.as_str() {
            PLAN_EVENT => {
                let request = serde_json::from_str::<InitiativePlanRequest>(&payload)?;
                if request
                    .initiative_id
                    .as_deref()
                    .is_some_and(|id| id != initiative_id)
                    || snapshot.plan.created_at_unix_ms != occurred
                    || Uuid::parse_str(&snapshot.plan.revision_id).is_err()
                    || snapshot.plan.objective != request.objective
                    || snapshot.plan.target_user != request.target_user
                    || snapshot.plan.success_measure != request.success_measure
                    || snapshot.plan.requirements != request.requirements
                {
                    return Err(Error::Conflict(
                        "initiative plan event does not match its payload".into(),
                    ));
                }
                current = Some(InitiativeStatus {
                    initiative_id: initiative_id.into(),
                    plan: snapshot.plan.clone(),
                    artifacts: Vec::new(),
                    task_links: Vec::new(),
                    missing: Vec::new(),
                    ready_to_claim: false,
                    advisory: true,
                });
            }
            ARTIFACT_EVENT => {
                let request = serde_json::from_str::<InitiativeArtifactRequest>(&payload)?;
                if request.initiative_id != initiative_id {
                    return Err(Error::Conflict(
                        "initiative artifact targets another stream".into(),
                    ));
                }
                let status = current
                    .as_mut()
                    .ok_or_else(|| Error::Conflict("initiative artifact has no plan".into()))?;
                let evidence_time = check_evidence(
                    connection,
                    &project_id,
                    &request.evidence_id,
                    EvidenceKind::WorkspaceFile,
                    EvidenceGrade::Direct,
                    status.plan.created_at_unix_ms,
                )?;
                apply_artifact(status, request.kind, &request.evidence_id, evidence_time);
            }
            LINK_EVENT => {
                let request = serde_json::from_str::<InitiativeTaskLinkRequest>(&payload)?;
                if request.initiative_id != initiative_id {
                    return Err(Error::Conflict(
                        "initiative link targets another stream".into(),
                    ));
                }
                let status = current
                    .as_mut()
                    .ok_or_else(|| Error::Conflict("initiative link has no plan".into()))?;
                let requirement = status
                    .plan
                    .requirements
                    .iter()
                    .find(|item| item.requirement_id == request.requirement_id)
                    .ok_or_else(|| {
                        Error::Conflict("initiative link names an unknown requirement".into())
                    })?;
                let task = load_task(connection, &request.task_id)?
                    .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
                if task.project_id != project_id
                    || !task
                        .acceptance_criteria
                        .contains(&requirement.acceptance_criterion)
                {
                    return Err(Error::Conflict(
                        "initiative link no longer matches its task".into(),
                    ));
                }
                apply_link(status, &request.requirement_id, &request.task_id)?;
            }
            _ => unreachable!(),
        }
        let status = current
            .as_ref()
            .expect("plan is required before initiative events");
        if snapshot.plan != status.plan
            || snapshot.artifacts != status.artifacts
            || snapshot.task_links != status.task_links
            || !snapshot.advisory
        {
            return Err(Error::Conflict(
                "initiative event result differs from deterministic replay".into(),
            ));
        }
    }
    current.ok_or_else(|| Error::NotFound(format!("initiative {initiative_id} plan")))
}

fn apply_artifact(
    status: &mut InitiativeStatus,
    kind: InitiativeArtifactKind,
    evidence_id: &str,
    created_at_unix_ms: i64,
) {
    status.artifacts.retain(|artifact| match kind {
        InitiativeArtifactKind::Planning => false,
        InitiativeArtifactKind::Design => artifact.kind == InitiativeArtifactKind::Planning,
        InitiativeArtifactKind::Integration => artifact.kind != InitiativeArtifactKind::Release,
        InitiativeArtifactKind::Release => true,
    });
    status.artifacts.retain(|artifact| artifact.kind != kind);
    status.artifacts.push(InitiativeArtifact {
        kind,
        evidence_id: evidence_id.into(),
        created_at_unix_ms,
    });
}

fn apply_link(status: &mut InitiativeStatus, requirement_id: &str, task_id: &str) -> Result<()> {
    if status
        .task_links
        .iter()
        .any(|link| link.task_id == task_id && link.requirement_id == requirement_id)
    {
        return Err(Error::Conflict(
            "task is already linked to requirement".into(),
        ));
    }
    status.task_links.push(InitiativeTaskLink {
        requirement_id: requirement_id.into(),
        task_id: task_id.into(),
    });
    status.artifacts.retain(|artifact| {
        !matches!(
            artifact.kind,
            InitiativeArtifactKind::Integration | InitiativeArtifactKind::Release
        )
    });
    Ok(())
}

fn refresh(connection: &Connection, status: &mut InitiativeStatus) -> Result<()> {
    let mut missing = Vec::new();
    let mut latest_task_time = status.plan.created_at_unix_ms;
    let mut earliest_task_time = None::<i64>;
    for requirement in &status.plan.requirements {
        let links: Vec<_> = status
            .task_links
            .iter()
            .filter(|link| link.requirement_id == requirement.requirement_id)
            .collect();
        if links.is_empty() {
            missing.push(format!(
                "requirement_unlinked:{}",
                requirement.requirement_id
            ));
        }
        for link in links {
            let task = load_task(connection, &link.task_id)?
                .ok_or_else(|| Error::NotFound(format!("task {}", link.task_id)))?;
            let proof_is_current = match super::validate_criterion_proofs(
                connection,
                &task,
                &task.completion_proofs,
            ) {
                Ok(()) => true,
                Err(Error::Conflict(_) | Error::Invalid(_) | Error::NotFound(_) | Error::Io(_)) => {
                    false
                }
                Err(error) => return Err(error),
            };
            let workflow_complete =
                super::workflow::read_status(connection, &task)?.stage == WorkflowStage::Completed;
            if task.status != TaskStatus::Completed
                || task.updated_at_unix_ms < status.plan.created_at_unix_ms
                || !proof_is_current
                || !workflow_complete
                || !task
                    .completion_proofs
                    .iter()
                    .any(|proof| proof.criterion == requirement.acceptance_criterion)
            {
                missing.push(format!(
                    "requirement_unverified:{}:{}",
                    requirement.requirement_id, link.task_id
                ));
            } else {
                latest_task_time = latest_task_time.max(task.updated_at_unix_ms);
                earliest_task_time =
                    Some(earliest_task_time.map_or(task.updated_at_unix_ms, |time| {
                        time.min(task.updated_at_unix_ms)
                    }));
            }
        }
    }
    for kind in [
        InitiativeArtifactKind::Planning,
        InitiativeArtifactKind::Design,
        InitiativeArtifactKind::Integration,
        InitiativeArtifactKind::Release,
    ] {
        let artifact = status
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == kind);
        match artifact {
            None => missing.push(format!("artifact_missing:{}", kind.as_str())),
            Some(artifact) if !artifact_current(connection, &artifact.evidence_id)? => {
                missing.push(format!("artifact_stale:{}", kind.as_str()));
            }
            Some(_) => {}
        }
    }
    let planning = status
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == InitiativeArtifactKind::Planning);
    let design = status
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == InitiativeArtifactKind::Design);
    if let (Some(planning), Some(design)) = (planning, design)
        && design.created_at_unix_ms < planning.created_at_unix_ms
    {
        missing.push("design_evidence_predates_planning".into());
    }
    if let (Some(design), Some(earliest_task_time)) = (design, earliest_task_time)
        && design.created_at_unix_ms > earliest_task_time
    {
        missing.push("design_evidence_postdates_implementation".into());
    }
    if let Some(integration) = status
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == InitiativeArtifactKind::Integration)
    {
        if integration.created_at_unix_ms < latest_task_time {
            missing.push("integration_evidence_predates_implementation".into());
        }
        if let Some(release) = status
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == InitiativeArtifactKind::Release)
            && release.created_at_unix_ms < integration.created_at_unix_ms
        {
            missing.push("release_evidence_predates_integration".into());
        }
    }
    status.ready_to_claim = missing.is_empty();
    status.missing = missing;
    Ok(())
}

fn artifact_current(connection: &Connection, evidence_id: &str) -> Result<bool> {
    let (locator, observed_locator, digest, workspace): (String, Option<String>, String, String) =
        connection.query_row(
            "SELECT e.locator, e.canonical_locator, e.content_sha256, p.workspace
         FROM evidence_artifacts e
         JOIN sessions s ON s.session_id = e.session_id
         JOIN projects p ON p.project_id = s.project_id
         WHERE e.evidence_id = ?1 AND e.kind = 'workspace_file' AND e.grade = 'direct'",
            [evidence_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
    let Ok(path) = fs::canonicalize(locator) else {
        return Ok(false);
    };
    if !path.is_file()
        || !path.starts_with(Path::new(&workspace))
        || observed_locator
            .as_deref()
            .is_some_and(|observed| path != Path::new(observed))
    {
        return Ok(false);
    }
    match sha256_file_bounded(&path, MAX_EVIDENCE_FILE_BYTES, "initiative artifact") {
        Ok((current, _)) => Ok(current == digest),
        Err(Error::Io(_) | Error::Invalid(_) | Error::Conflict(_)) => Ok(false),
        Err(error) => Err(error),
    }
}
