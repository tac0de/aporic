use super::*;
use crate::domain::{
    ImprovementListRequest, ImprovementOutcome, ImprovementRequest, ImprovementSubmitRequest,
    PrototypeBrief, PrototypeBriefCreateRequest, PrototypeBriefOutcome, PrototypeEvidenceState,
    PrototypeGetRequest, PrototypeReview, PrototypeReviewOutcome, PrototypeReviewRequest,
    PrototypeStatus,
};
use std::io::Read;

impl Store {
    pub fn submit_improvement(
        &self,
        request: &ImprovementSubmitRequest,
    ) -> Result<ImprovementOutcome> {
        require_text("source_session_id", &request.source_session_id)?;
        require_text("source_evidence_id", &request.source_evidence_id)?;
        require_bounded_public_text("objective", &request.objective, 256)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.acceptance_criteria.is_empty() || request.acceptance_criteria.len() > 8 {
            return Err(Error::Invalid(
                "improvement requires 1 to 8 criteria".into(),
            ));
        }
        privacy_safe_summary("objective", &request.objective)?;
        for criterion in &request.acceptance_criteria {
            require_bounded_public_text("criterion", criterion, 512)?;
            privacy_safe_summary("criterion", criterion)?;
        }
        require_texts("acceptance_criteria", &request.acceptance_criteria)?;
        let core_workspace = canonical_workspace(&request.core_workspace)?;
        if !Path::new(&core_workspace)
            .join("crates/aporic/Cargo.toml")
            .is_file()
        {
            return Err(Error::Invalid(
                "core_workspace must contain the Aporic core package".into(),
            ));
        }
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<ImprovementOutcome, _>(
            &tx,
            &request.idempotency_key,
            "improvement_submitted",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&tx, &request.source_session_id)?;
        let source_project_id: String = tx.query_row(
            "SELECT project_id FROM sessions WHERE session_id=?1",
            [&request.source_session_id],
            |row| row.get(0),
        )?;
        let source_workspace: String = tx.query_row(
            "SELECT workspace FROM projects WHERE project_id=?1",
            [&source_project_id],
            |row| row.get(0),
        )?;
        if source_workspace == core_workspace {
            return Err(Error::Invalid(
                "source and core workspaces must differ".into(),
            ));
        }
        let source_evidence: Option<(String, String)> = tx
            .query_row(
                "SELECT sessions.project_id, evidence_artifacts.grade FROM evidence_artifacts
             JOIN sessions ON sessions.session_id=evidence_artifacts.session_id
             WHERE evidence_artifacts.evidence_id=?1",
                [&request.source_evidence_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if source_evidence.as_ref().map(|(project, _)| project) != Some(&source_project_id) {
            return Err(Error::Conflict(
                "source evidence must belong to the source workspace".into(),
            ));
        }
        let core_project_id = find_or_create_project(&tx, &core_workspace, now)?;
        let normalized = request.objective.trim().to_lowercase();
        if let Some(existing_id) = tx
            .query_row(
                "SELECT request_id FROM improvement_requests WHERE source_project_id=?1
             AND core_project_id=?2 AND normalized_objective=?3",
                params![source_project_id, core_project_id, normalized],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(ImprovementOutcome {
                request: load_improvement(&tx, &existing_id)?,
                duplicate: false,
                possible_duplicate: true,
            });
        }
        let possible_duplicate = tx
            .query_row(
                "SELECT 1 FROM tasks WHERE project_id=?1 AND status IN ('queued', 'leased')
             AND lower(trim(objective))=?2 LIMIT 1",
                params![core_project_id, normalized],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some();
        let session_id = Uuid::now_v7().to_string();
        let task_id = Uuid::now_v7().to_string();
        let request_id = Uuid::now_v7().to_string();
        tx.execute(
            "INSERT INTO sessions(session_id, project_id, objective, status,
             opened_at_unix_ms, last_activity_at_unix_ms, closed_at_unix_ms, summary)
             VALUES (?1, ?2, ?3, 'completed', ?4, ?4, ?4, 'Improvement intake registration only')",
            params![
                session_id,
                core_project_id,
                "Cross-workspace improvement intake",
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO tasks(task_id, project_id, session_id, objective, acceptance_criteria_json,
             write_scope_json, depends_on_json, status, created_at_unix_ms, updated_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, '[]', '[]', 'queued', ?6, ?6)",
            params![task_id, core_project_id, session_id, request.objective.trim(),
                serde_json::to_string(&request.acceptance_criteria)?, now],
        )?;
        tx.execute(
            "INSERT INTO improvement_requests(request_id, source_project_id, source_evidence_id,
             core_project_id, task_id, normalized_objective, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                request_id,
                source_project_id,
                request.source_evidence_id,
                core_project_id,
                task_id,
                normalized,
                now
            ],
        )?;
        let outcome = ImprovementOutcome {
            request: ImprovementRequest {
                request_id,
                source_workspace,
                source_evidence_id: request.source_evidence_id.clone(),
                source_evidence_grade: parse_evidence_grade_value(&source_evidence.unwrap().1)?,
                core_workspace,
                task_id,
                objective: request.objective.trim().to_owned(),
                status: TaskStatus::Queued,
                assigned_worker: None,
                created_at_unix_ms: now,
                registration_only: true,
            },
            duplicate: false,
            possible_duplicate,
        };
        append_event(
            &tx,
            &request.idempotency_key,
            &core_project_id,
            "improvement_submitted",
            request,
            &outcome,
            now,
        )?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn list_improvements(
        &self,
        request: &ImprovementListRequest,
    ) -> Result<Vec<ImprovementRequest>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let Some(project_id) = project_id_for_workspace(&connection, &workspace)? else {
            return Ok(Vec::new());
        };
        let limit = i64::from(request.limit.unwrap_or(50).clamp(1, 100));
        let mut statement = connection.prepare(
            "SELECT request_id FROM improvement_requests WHERE source_project_id=?1 OR core_project_id=?1
             ORDER BY created_at_unix_ms DESC, request_id DESC LIMIT ?2",
        )?;
        let ids = statement
            .query_map(params![project_id, limit], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| load_improvement(&connection, id))
            .collect()
    }

    pub fn create_prototype_brief(
        &self,
        request: &PrototypeBriefCreateRequest,
    ) -> Result<PrototypeBriefOutcome> {
        for (name, value, bound) in [
            ("target_user", &request.target_user, 1024),
            (
                "core_experience_hypothesis",
                &request.core_experience_hypothesis,
                2048,
            ),
            ("fidelity", &request.fidelity, 256),
            ("reuse_boundary", &request.reuse_boundary, 1024),
            ("technology_stack", &request.technology_stack, 1024),
            ("repository_boundary", &request.repository_boundary, 1024),
            ("validation_method", &request.validation_method, 2048),
        ] {
            require_bounded_public_text(name, value, bound)?;
        }
        require_text("task_id", &request.task_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<PrototypeBriefOutcome, _>(
            &tx,
            &request.idempotency_key,
            "prototype_brief_created",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = project_id_for_workspace(&tx, &workspace)?
            .ok_or_else(|| Error::NotFound("workspace project".into()))?;
        let task = load_task(&tx, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        if task.project_id != project_id
            || matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled)
        {
            return Err(Error::Conflict(
                "brief requires an active task in the workspace".into(),
            ));
        }
        if tx
            .query_row(
                "SELECT 1 FROM prototype_briefs WHERE task_id=?1",
                [&request.task_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some()
        {
            return Err(Error::Conflict("task already has a prototype brief".into()));
        }
        let brief = PrototypeBrief {
            brief_id: Uuid::now_v7().to_string(),
            task_id: request.task_id.clone(),
            target_user: request.target_user.trim().to_owned(),
            core_experience_hypothesis: request.core_experience_hypothesis.trim().to_owned(),
            fidelity: request.fidelity.trim().to_owned(),
            reuse_boundary: request.reuse_boundary.trim().to_owned(),
            technology_stack: request.technology_stack.trim().to_owned(),
            repository_boundary: request.repository_boundary.trim().to_owned(),
            validation_method: request.validation_method.trim().to_owned(),
            created_at_unix_ms: now,
            advisory: true,
        };
        tx.execute(
            "INSERT INTO prototype_briefs(brief_id, project_id, task_id, target_user,
            core_experience_hypothesis, fidelity, reuse_boundary, technology_stack,
            repository_boundary, validation_method, created_at_unix_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                brief.brief_id,
                project_id,
                brief.task_id,
                brief.target_user,
                brief.core_experience_hypothesis,
                brief.fidelity,
                brief.reuse_boundary,
                brief.technology_stack,
                brief.repository_boundary,
                brief.validation_method,
                brief.created_at_unix_ms
            ],
        )?;
        let outcome = PrototypeBriefOutcome {
            brief,
            duplicate: false,
        };
        append_event(
            &tx,
            &request.idempotency_key,
            &project_id,
            "prototype_brief_created",
            request,
            &outcome,
            now,
        )?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn review_prototype(
        &self,
        request: &PrototypeReviewRequest,
    ) -> Result<PrototypeReviewOutcome> {
        require_text("task_id", &request.task_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<PrototypeReviewOutcome, _>(
            &tx,
            &request.idempotency_key,
            "prototype_review_created",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = project_id_for_workspace(&tx, &workspace)?
            .ok_or_else(|| Error::NotFound("workspace project".into()))?;
        let brief = load_prototype_brief(&tx, &request.task_id)?
            .ok_or_else(|| Error::NotFound("prototype brief for task".into()))?;
        let task =
            load_task(&tx, &request.task_id)?.ok_or_else(|| Error::NotFound("task".into()))?;
        if task.project_id != project_id {
            return Err(Error::Conflict("task belongs to another workspace".into()));
        }
        let smoke = evidence_state(&tx, &project_id, &request.smoke_evidence_ids, false)?;
        let rules = evidence_state(&tx, &project_id, &request.rule_evidence_ids, false)?;
        let user_value = evidence_state(&tx, &project_id, &request.user_play_evidence_ids, true)?;
        let mut next_actions = Vec::new();
        if smoke == PrototypeEvidenceState::Unknown {
            next_actions.push("Run a functional smoke check and attach evidence.".into());
        }
        if rules == PrototypeEvidenceState::Unknown {
            next_actions.push("Check the core rules and attach evidence.".into());
        }
        if user_value != PrototypeEvidenceState::Observed {
            next_actions.push("Observe target-user play and attach direct evidence before claiming user value or a validated vertical slice.".into());
        }
        let review = PrototypeReview {
            review_id: Uuid::now_v7().to_string(),
            brief_id: brief.brief_id,
            task_id: request.task_id.clone(),
            smoke_evidence_ids: request.smoke_evidence_ids.clone(),
            rule_evidence_ids: request.rule_evidence_ids.clone(),
            user_play_evidence_ids: request.user_play_evidence_ids.clone(),
            smoke,
            rules,
            user_value,
            next_actions,
            created_at_unix_ms: now,
            advisory: true,
        };
        tx.execute(
            "INSERT INTO prototype_reviews(review_id, brief_id, task_id,
            smoke_evidence_json, rule_evidence_json, user_play_evidence_json,
            smoke_state, rule_state, user_value_state, created_at_unix_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                review.review_id,
                review.brief_id,
                review.task_id,
                serde_json::to_string(&request.smoke_evidence_ids)?,
                serde_json::to_string(&request.rule_evidence_ids)?,
                serde_json::to_string(&request.user_play_evidence_ids)?,
                evidence_state_str(&review.smoke),
                evidence_state_str(&review.rules),
                evidence_state_str(&review.user_value),
                now
            ],
        )?;
        let outcome = PrototypeReviewOutcome {
            review,
            duplicate: false,
        };
        append_event(
            &tx,
            &request.idempotency_key,
            &project_id,
            "prototype_review_created",
            request,
            &outcome,
            now,
        )?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn get_prototype(&self, request: &PrototypeGetRequest) -> Result<Option<PrototypeStatus>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let Some(project_id) = project_id_for_workspace(&connection, &workspace)? else {
            return Ok(None);
        };
        let Some(task) = load_task(&connection, &request.task_id)? else {
            return Ok(None);
        };
        if task.project_id != project_id {
            return Ok(None);
        }
        let Some(brief) = load_prototype_brief(&connection, &request.task_id)? else {
            return Ok(None);
        };
        let latest_review = connection.query_row(
            "SELECT review_id, smoke_evidence_json, rule_evidence_json, user_play_evidence_json,
             smoke_state, rule_state, user_value_state, created_at_unix_ms
             FROM prototype_reviews WHERE task_id=?1 ORDER BY created_at_unix_ms DESC, rowid DESC LIMIT 1",
            [&request.task_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?,
                row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?,
                row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, i64>(7)?)),
        ).optional()?.map(|(review_id, smoke_ids, rule_ids, play_ids, smoke, rules, user_value, at)| {
            let smoke = parse_evidence_state(&smoke)?;
            let rules = parse_evidence_state(&rules)?;
            let user_value = parse_evidence_state(&user_value)?;
            let mut next_actions = Vec::new();
            if smoke == PrototypeEvidenceState::Unknown { next_actions.push("Run a functional smoke check and attach evidence.".into()); }
            if rules == PrototypeEvidenceState::Unknown { next_actions.push("Check the core rules and attach evidence.".into()); }
            if user_value != PrototypeEvidenceState::Observed { next_actions.push("Observe target-user play and attach direct evidence before claiming user value or a validated vertical slice.".into()); }
            Ok::<_, Error>(PrototypeReview { review_id, brief_id: brief.brief_id.clone(), task_id: request.task_id.clone(),
                smoke_evidence_ids: serde_json::from_str(&smoke_ids)?,
                rule_evidence_ids: serde_json::from_str(&rule_ids)?,
                user_play_evidence_ids: serde_json::from_str(&play_ids)?,
                smoke, rules, user_value, next_actions, created_at_unix_ms: at, advisory: true })
        }).transpose()?;
        Ok(Some(PrototypeStatus {
            brief,
            latest_review,
        }))
    }
}

fn load_improvement(connection: &Connection, id: &str) -> Result<ImprovementRequest> {
    let (
        request_id,
        source_workspace,
        source_evidence_id,
        source_evidence_grade,
        core_workspace,
        task_id,
        objective,
        status,
        assigned_worker,
        created_at_unix_ms,
    ) = connection.query_row(
        "SELECT i.request_id, source.workspace, i.source_evidence_id, e.grade, core.workspace,
         i.task_id, t.objective, t.status, t.lease_owner, i.created_at_unix_ms FROM improvement_requests i
         JOIN projects source ON source.project_id=i.source_project_id
         JOIN projects core ON core.project_id=i.core_project_id
         JOIN evidence_artifacts e ON e.evidence_id=i.source_evidence_id
         JOIN tasks t ON t.task_id=i.task_id WHERE i.request_id=?1",
        [id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, i64>(9)?,
            ))
        },
    )?;
    Ok(ImprovementRequest {
        request_id,
        source_workspace,
        source_evidence_id,
        source_evidence_grade: parse_evidence_grade_value(&source_evidence_grade)?,
        core_workspace,
        task_id,
        objective,
        status: parse_task_status(status)?,
        assigned_worker,
        created_at_unix_ms,
        registration_only: true,
    })
}

fn load_prototype_brief(connection: &Connection, task_id: &str) -> Result<Option<PrototypeBrief>> {
    Ok(connection
        .query_row(
            "SELECT brief_id, task_id, target_user, core_experience_hypothesis,
        fidelity, reuse_boundary, technology_stack, repository_boundary, validation_method,
        created_at_unix_ms FROM prototype_briefs WHERE task_id=?1",
            [task_id],
            |row| {
                Ok(PrototypeBrief {
                    brief_id: row.get(0)?,
                    task_id: row.get(1)?,
                    target_user: row.get(2)?,
                    core_experience_hypothesis: row.get(3)?,
                    fidelity: row.get(4)?,
                    reuse_boundary: row.get(5)?,
                    technology_stack: row.get(6)?,
                    repository_boundary: row.get(7)?,
                    validation_method: row.get(8)?,
                    created_at_unix_ms: row.get(9)?,
                    advisory: true,
                })
            },
        )
        .optional()?)
}

fn evidence_state(
    connection: &Connection,
    project_id: &str,
    ids: &[String],
    user_play: bool,
) -> Result<PrototypeEvidenceState> {
    if ids.len() > 8 {
        return Err(Error::Invalid("evidence category exceeds 8 IDs".into()));
    }
    require_texts("evidence_ids", ids)?;
    let mut state = PrototypeEvidenceState::Unknown;
    let workspace: Option<String> = if user_play {
        connection
            .query_row(
                "SELECT workspace FROM projects WHERE project_id=?1",
                [project_id],
                |row| row.get(0),
            )
            .optional()?
    } else {
        None
    };
    for id in ids {
        let evidence: Option<(String, String, String, String)> = connection.query_row(
            "SELECT e.grade, e.kind, e.locator, e.content_sha256 FROM evidence_artifacts e JOIN sessions s ON s.session_id=e.session_id
             WHERE e.evidence_id=?1 AND s.project_id=?2",
            params![id, project_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).optional()?;
        let (grade, kind, locator, digest) =
            evidence.ok_or_else(|| Error::NotFound(format!("evidence {id} in workspace")))?;
        if user_play
            && (kind != "workspace_file"
                || !locator.ends_with(".playtest.json")
                || !valid_playtest_report(&locator, &digest, workspace.as_deref().unwrap_or(""))?)
        {
            continue;
        }
        if grade == "direct" {
            state = PrototypeEvidenceState::Observed;
        } else if state == PrototypeEvidenceState::Unknown {
            state = PrototypeEvidenceState::Reported;
        }
    }
    Ok(state)
}

fn valid_playtest_report(locator: &str, expected_digest: &str, workspace: &str) -> Result<bool> {
    let path = match fs::canonicalize(locator) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if !path.is_file() || !path.starts_with(workspace) {
        return Ok(false);
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(64 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 {
        return Ok(false);
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != expected_digest {
        return Ok(false);
    }
    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return Ok(false),
    };
    Ok(value
        .get("target_user")
        .and_then(|v| v.as_str())
        .is_some_and(|v| !v.trim().is_empty())
        && value
            .get("participant_count")
            .and_then(|v| v.as_u64())
            .is_some_and(|v| v > 0)
        && value
            .get("observed_behavior")
            .and_then(|v| v.as_str())
            .is_some_and(|v| !v.trim().is_empty())
        && value
            .get("session_date")
            .and_then(|v| v.as_str())
            .is_some_and(|v| !v.trim().is_empty()))
}

fn evidence_state_str(state: &PrototypeEvidenceState) -> &'static str {
    match state {
        PrototypeEvidenceState::Unknown => "unknown",
        PrototypeEvidenceState::Reported => "reported",
        PrototypeEvidenceState::Observed => "observed",
    }
}

fn parse_evidence_state(raw: &str) -> Result<PrototypeEvidenceState> {
    match raw {
        "unknown" => Ok(PrototypeEvidenceState::Unknown),
        "reported" => Ok(PrototypeEvidenceState::Reported),
        "observed" => Ok(PrototypeEvidenceState::Observed),
        _ => Err(Error::Invalid("invalid prototype evidence state".into())),
    }
}

fn privacy_safe_summary(name: &str, value: &str) -> Result<()> {
    let lower = value.to_ascii_lowercase();
    if value.contains(['\n', '\r', '{', '}'])
        || [
            "password=",
            "token=",
            "bearer ",
            "-----begin",
            "sk-",
            "api_key",
            "secret=",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return Err(Error::Invalid(format!(
            "{name} must be concise metadata without raw conversation or credentials"
        )));
    }
    Ok(())
}

pub(super) fn export_improvement_data(
    connection: &Connection,
    project_id: &str,
) -> Result<(
    Vec<ImprovementRequest>,
    Vec<PrototypeBrief>,
    Vec<PrototypeReview>,
)> {
    let improvements = {
        let mut statement = connection.prepare("SELECT request_id FROM improvement_requests
            WHERE source_project_id=?1 OR core_project_id=?1 ORDER BY created_at_unix_ms, request_id")?;
        let ids = statement
            .query_map([project_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| load_improvement(connection, id))
            .collect::<Result<Vec<_>>>()?
    };
    let briefs = {
        let mut statement = connection.prepare(
            "SELECT task_id FROM prototype_briefs
            WHERE project_id=?1 ORDER BY created_at_unix_ms, brief_id",
        )?;
        let ids = statement
            .query_map([project_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| {
                load_prototype_brief(connection, id)?
                    .ok_or_else(|| Error::NotFound(format!("prototype brief {id}")))
            })
            .collect::<Result<Vec<_>>>()?
    };
    let reviews = {
        let mut statement = connection.prepare(
            "SELECT r.review_id, r.brief_id, r.task_id,
            r.smoke_evidence_json, r.rule_evidence_json, r.user_play_evidence_json,
            r.smoke_state, r.rule_state, r.user_value_state, r.created_at_unix_ms
            FROM prototype_reviews r JOIN prototype_briefs b ON b.brief_id=r.brief_id
            WHERE b.project_id=?1 ORDER BY r.created_at_unix_ms, r.review_id",
        )?;
        let rows = statement
            .query_map([project_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(9)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter().map(|(review_id, brief_id, task_id, smoke_ids, rule_ids, play_ids, smoke, rules, user_value, at)| {
            let smoke = parse_evidence_state(&smoke)?;
            let rules = parse_evidence_state(&rules)?;
            let user_value = parse_evidence_state(&user_value)?;
            let mut next_actions = Vec::new();
            if smoke == PrototypeEvidenceState::Unknown { next_actions.push("Run a functional smoke check and attach evidence.".into()); }
            if rules == PrototypeEvidenceState::Unknown { next_actions.push("Check the core rules and attach evidence.".into()); }
            if user_value != PrototypeEvidenceState::Observed { next_actions.push("Observe target-user play and attach direct evidence before claiming user value or a validated vertical slice.".into()); }
            Ok(PrototypeReview { review_id, brief_id, task_id,
                smoke_evidence_ids: serde_json::from_str(&smoke_ids)?,
                rule_evidence_ids: serde_json::from_str(&rule_ids)?,
                user_play_evidence_ids: serde_json::from_str(&play_ids)?,
                smoke, rules, user_value, next_actions, created_at_unix_ms: at, advisory: true })
        }).collect::<Result<Vec<_>>>()?
    };
    Ok((improvements, briefs, reviews))
}
