use super::*;
use crate::domain::{
    GovernmentBootstrapRequest, GovernmentIncumbent, GovernmentPerson, GovernmentPersonOutcome,
    GovernmentPersonRegisterRequest, GovernmentRoster, GovernmentRosterOutcome, GovernmentTerm,
    GovernmentTermAppointRequest, GovernmentTermEndRequest, GovernmentTermOutcome,
};

impl Store {
    pub fn bootstrap_government(
        &self,
        request: &GovernmentBootstrapRequest,
    ) -> Result<GovernmentRosterOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let project_id = project_for_session(&transaction, &request.session_id)?;
        let event_key = format!("government:{project_id}:{}", request.idempotency_key);
        if let Some(mut outcome) = duplicate_result::<GovernmentRosterOutcome, _>(
            &transaction,
            &event_key,
            "government_roster_bootstrapped",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        let existing: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM government_people WHERE project_id = ?1",
            [&project_id],
            |row| row.get(0),
        )?;
        if existing != 0 {
            return Err(Error::Conflict(
                "government roster already has people; bootstrap must run on an empty roster"
                    .to_owned(),
            ));
        }
        let active_legacy_minister: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM office_appointments AS office
             JOIN sessions ON sessions.session_id = office.session_id
             WHERE office.project_id = ?1 AND office.revoked_at_unix_ms IS NULL
             AND sessions.status = 'open' AND sessions.abandoned = 0",
            [&project_id],
            |row| row.get(0),
        )?;
        if active_legacy_minister != 0 {
            return Err(Error::Conflict(
                "revoke active session-scoped product minister appointments before bootstrapping the named cabinet".to_owned(),
            ));
        }
        for (full_name, position_id) in crate::government::INITIAL_CABINET {
            let person_id = Uuid::now_v7().to_string();
            insert_person(&transaction, &project_id, &person_id, full_name, now)?;
            insert_term(
                &transaction,
                &project_id,
                &person_id,
                position_id,
                None,
                None,
                now,
            )?;
        }
        transaction.execute(
            "INSERT INTO government_roster_state(project_id, initialized_at_unix_ms) VALUES (?1, ?2)",
            params![project_id, now],
        )?;
        let outcome = GovernmentRosterOutcome {
            roster: roster_for_project(&transaction, &project_id, 200)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &event_key,
            &project_id,
            "government_roster_bootstrapped",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn register_government_person(
        &self,
        request: &GovernmentPersonRegisterRequest,
    ) -> Result<GovernmentPersonOutcome> {
        require_text("session_id", &request.session_id)?;
        require_bounded_public_text("full_name", &request.full_name, 80)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let project_id = project_for_session(&transaction, &request.session_id)?;
        let event_key = format!("government:{project_id}:{}", request.idempotency_key);
        if let Some(mut outcome) = duplicate_result::<GovernmentPersonOutcome, _>(
            &transaction,
            &event_key,
            "government_person_registered",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        require_initialized(&transaction, &project_id)?;
        let person_id = Uuid::now_v7().to_string();
        insert_person(
            &transaction,
            &project_id,
            &person_id,
            &request.full_name,
            now,
        )?;
        let outcome = GovernmentPersonOutcome {
            person: load_person(&transaction, &person_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &event_key,
            &person_id,
            "government_person_registered",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn appoint_government_term(
        &self,
        request: &GovernmentTermAppointRequest,
    ) -> Result<GovernmentTermOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("person_id", &request.person_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if !crate::government::known_position(&request.position_id) {
            return Err(Error::Invalid("unknown government position".to_owned()));
        }
        if let Some(note) = &request.handoff_note {
            require_bounded_public_text("handoff_note", note, 2048)?;
        }
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let project_id = project_for_session(&transaction, &request.session_id)?;
        let event_key = format!("government:{project_id}:{}", request.idempotency_key);
        if let Some(mut outcome) = duplicate_result::<GovernmentTermOutcome, _>(
            &transaction,
            &event_key,
            "government_term_appointed",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        require_initialized(&transaction, &project_id)?;
        let person_project: String = transaction
            .query_row(
                "SELECT project_id FROM government_people WHERE person_id = ?1",
                [&request.person_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("government person {}", request.person_id)))?;
        if person_project != project_id {
            return Err(Error::Conflict(
                "person belongs to another workspace".to_owned(),
            ));
        }
        let occupied: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM government_terms WHERE project_id = ?1
             AND (position_id = ?2 OR person_id = ?3) AND ended_at_unix_ms IS NULL",
            params![project_id, request.position_id, request.person_id],
            |row| row.get(0),
        )?;
        if occupied != 0 {
            return Err(Error::Conflict(
                "position or person already has an active term".to_owned(),
            ));
        }
        let predecessor: Option<String> = transaction
            .query_row(
                "SELECT term_id FROM government_terms WHERE project_id = ?1 AND position_id = ?2
             ORDER BY sequence DESC LIMIT 1",
                params![project_id, request.position_id],
                |row| row.get(0),
            )
            .optional()?;
        if predecessor.is_some() && request.handoff_note.is_none() {
            return Err(Error::Invalid(
                "successor term requires a handoff_note".to_owned(),
            ));
        }
        let term_id = insert_term(
            &transaction,
            &project_id,
            &request.person_id,
            &request.position_id,
            predecessor.as_deref(),
            request.handoff_note.as_deref(),
            now,
        )?;
        let outcome = GovernmentTermOutcome {
            term: load_term(&transaction, &term_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &event_key,
            &term_id,
            "government_term_appointed",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn end_government_term(
        &self,
        request: &GovernmentTermEndRequest,
    ) -> Result<GovernmentTermOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("term_id", &request.term_id)?;
        require_bounded_public_text("reason", &request.reason, 1024)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let project_id = project_for_session(&transaction, &request.session_id)?;
        let event_key = format!("government:{project_id}:{}", request.idempotency_key);
        if let Some(mut outcome) = duplicate_result::<GovernmentTermOutcome, _>(
            &transaction,
            &event_key,
            "government_term_ended",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        let term_project: String = transaction
            .query_row(
                "SELECT project_id FROM government_terms WHERE term_id = ?1",
                [&request.term_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("government term {}", request.term_id)))?;
        if term_project != project_id {
            return Err(Error::Conflict(
                "term belongs to another workspace".to_owned(),
            ));
        }
        let term = load_term(&transaction, &request.term_id)?;
        if term.ended_at_unix_ms.is_some() {
            return Err(Error::Conflict("government term already ended".to_owned()));
        }
        if term.position_id == "product.experiment.minister" {
            let active_session_appointments: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM office_appointments AS office
                 JOIN sessions ON sessions.session_id = office.session_id
                 WHERE office.project_id = ?1 AND office.assignee_id = ?2
                   AND office.revoked_at_unix_ms IS NULL
                   AND sessions.status = 'open' AND sessions.abandoned = 0",
                params![project_id, term.person_id],
                |row| row.get(0),
            )?;
            if active_session_appointments != 0 {
                return Err(Error::Conflict(
                    "revoke active product minister session appointments before ending the named term".to_owned(),
                ));
            }
        }
        transaction.execute(
            "UPDATE government_terms SET ended_at_unix_ms = ?1, end_reason = ?2 WHERE term_id = ?3",
            params![now, request.reason, request.term_id],
        )?;
        let outcome = GovernmentTermOutcome {
            term: load_term(&transaction, &request.term_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &event_key,
            &request.term_id,
            "government_term_ended",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn government_roster(
        &self,
        request: &GovernmentWorkspaceRequest,
    ) -> Result<GovernmentRoster> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id: Option<String> = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [workspace],
                |row| row.get(0),
            )
            .optional()?;
        match project_id {
            Some(id) => {
                roster_for_project(&connection, &id, request.limit.unwrap_or(50).clamp(1, 200))
            }
            None => Ok(GovernmentRoster {
                current: vec![],
                people: vec![],
                terms: vec![],
                positions: crate::government::positions(),
                initialized: false,
                advisory: true,
                grants_authority: false,
            }),
        }
    }

    pub fn audit_government_roster(&self) -> Result<(u64, u64, u64)> {
        let connection = self.connection()?;
        let people = connection
            .prepare("SELECT person_id FROM government_people ORDER BY sequence")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let terms = connection
            .prepare("SELECT term_id FROM government_terms ORDER BY sequence")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let invalid = people
            .iter()
            .filter(|id| load_person(&connection, id).is_err())
            .count()
            + terms
                .iter()
                .filter(|id| load_term(&connection, id).is_err())
                .count();
        Ok((people.len() as u64, terms.len() as u64, invalid as u64))
    }
}

fn project_for_session(transaction: &Transaction<'_>, session_id: &str) -> Result<String> {
    transaction
        .query_row(
            "SELECT project_id FROM sessions WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("session {session_id}")))
}

fn require_initialized(connection: &Connection, project_id: &str) -> Result<()> {
    let initialized: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM government_roster_state WHERE project_id = ?1)",
        [project_id],
        |row| row.get(0),
    )?;
    if initialized {
        Ok(())
    } else {
        Err(Error::Conflict(
            "government roster has not been bootstrapped".to_owned(),
        ))
    }
}

pub(super) fn roster_initialized(connection: &Connection, project_id: &str) -> Result<bool> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM government_roster_state WHERE project_id = ?1)",
            [project_id],
            |row| row.get(0),
        )
        .map_err(Error::from)
}

pub(super) fn active_person_for_position(
    connection: &Connection,
    project_id: &str,
    position_id: &str,
) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT person_id FROM government_terms WHERE project_id = ?1
         AND position_id = ?2 AND ended_at_unix_ms IS NULL",
            params![project_id, position_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Error::from)
}

pub(super) fn active_product_lead(
    connection: &Connection,
    project_id: &str,
    person_id: &str,
) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT position_id FROM government_terms WHERE project_id = ?1
         AND person_id = ?2 AND ended_at_unix_ms IS NULL
         AND position_id IN ('product.design.lead', 'product.frontend.lead',
                             'product.backend.lead', 'product.game.lead')",
            params![project_id, person_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Error::from)
}

fn insert_person(
    connection: &Connection,
    project_id: &str,
    person_id: &str,
    full_name: &str,
    now: i64,
) -> Result<()> {
    let digest = digest_json(&serde_json::json!({
        "person_id": person_id, "project_id": project_id, "full_name": full_name,
        "created_at_unix_ms": now,
    }))?;
    connection.execute(
        "INSERT INTO government_people(person_id, project_id, full_name, person_sha256, created_at_unix_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![person_id, project_id, full_name, digest, now],
    )?;
    Ok(())
}

fn insert_term(
    connection: &Connection,
    project_id: &str,
    person_id: &str,
    position_id: &str,
    predecessor: Option<&str>,
    handoff_note: Option<&str>,
    now: i64,
) -> Result<String> {
    let term_id = Uuid::now_v7().to_string();
    let position = crate::government::position_definition(position_id)
        .ok_or_else(|| Error::Invalid("unknown government position".to_owned()))?;
    let position_json = serde_json::to_string(&position)?;
    let digest = digest_json(&serde_json::json!({
        "term_id": term_id, "project_id": project_id, "person_id": person_id,
        "position_id": position_id, "position": position, "predecessor_term_id": predecessor,
        "handoff_note": handoff_note, "started_at_unix_ms": now,
    }))?;
    connection.execute(
        "INSERT INTO government_terms(term_id, project_id, person_id, position_id, position_json,
          predecessor_term_id, handoff_note, started_at_unix_ms, term_sha256)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            term_id,
            project_id,
            person_id,
            position_id,
            position_json,
            predecessor,
            handoff_note,
            now,
            digest
        ],
    )?;
    Ok(term_id)
}

pub(super) fn load_person(connection: &Connection, person_id: &str) -> Result<GovernmentPerson> {
    let (person, project_id, digest): (GovernmentPerson, String, String) = connection
        .query_row(
            "SELECT person_id, project_id, full_name, person_sha256, created_at_unix_ms
         FROM government_people WHERE person_id = ?1",
            [person_id],
            |row| {
                Ok((
                    GovernmentPerson {
                        person_id: row.get(0)?,
                        full_name: row.get(2)?,
                        created_at_unix_ms: row.get(4)?,
                    },
                    row.get(1)?,
                    row.get(3)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("government person {person_id}")))?;
    let expected = digest_json(&serde_json::json!({
        "person_id": person.person_id, "project_id": project_id,
        "full_name": person.full_name, "created_at_unix_ms": person.created_at_unix_ms,
    }))?;
    if digest != expected {
        return Err(Error::Conflict(
            "government person digest mismatch".to_owned(),
        ));
    }
    Ok(person)
}

pub(super) fn load_term(connection: &Connection, term_id: &str) -> Result<GovernmentTerm> {
    let (term, project_id): (GovernmentTerm, String) = connection
        .query_row(
            "SELECT term_id, project_id, person_id, position_id, position_json, predecessor_term_id,
                handoff_note, started_at_unix_ms, ended_at_unix_ms, end_reason, term_sha256
         FROM government_terms WHERE term_id = ?1",
            [term_id],
            |row| {
                let position_json: String = row.get(4)?;
                let position = serde_json::from_str(&position_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4, rusqlite::types::Type::Text, Box::new(error),
                    )
                })?;
                Ok((
                    GovernmentTerm {
                        term_id: row.get(0)?,
                        person_id: row.get(2)?,
                        position_id: row.get(3)?,
                        position,
                        predecessor_term_id: row.get(5)?,
                        handoff_note: row.get(6)?,
                        started_at_unix_ms: row.get(7)?,
                        ended_at_unix_ms: row.get(8)?,
                        end_reason: row.get(9)?,
                        term_sha256: row.get(10)?,
                        advisory: true,
                        grants_authority: false,
                    },
                    row.get(1)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("government term {term_id}")))?;
    if term.position.position_id != term.position_id || term.position.version == 0 {
        return Err(Error::Conflict(
            "government term position snapshot mismatch".to_owned(),
        ));
    }
    let person_project: String = connection.query_row(
        "SELECT project_id FROM government_people WHERE person_id = ?1",
        [&term.person_id],
        |row| row.get(0),
    )?;
    if person_project != project_id {
        return Err(Error::Conflict(
            "government term person workspace mismatch".to_owned(),
        ));
    }
    let _ = load_person(connection, &term.person_id)?;
    let previous: Option<String> = connection
        .query_row(
            "SELECT term_id FROM government_terms WHERE project_id = ?1 AND position_id = ?2
         AND sequence < (SELECT sequence FROM government_terms WHERE term_id = ?3)
         ORDER BY sequence DESC LIMIT 1",
            params![project_id, term.position_id, term.term_id],
            |row| row.get(0),
        )
        .optional()?;
    if previous != term.predecessor_term_id {
        return Err(Error::Conflict(
            "government term predecessor does not match position history".to_owned(),
        ));
    }
    if let Some(predecessor_id) = &term.predecessor_term_id {
        let (prior_project, prior_position, prior_started, prior_ended): (String, String, i64, Option<i64>) = connection.query_row(
            "SELECT project_id, position_id, started_at_unix_ms, ended_at_unix_ms FROM government_terms WHERE term_id = ?1",
            [predecessor_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        if prior_project != project_id
            || prior_position != term.position_id
            || prior_started > term.started_at_unix_ms
            || prior_ended.is_none_or(|ended| ended > term.started_at_unix_ms)
        {
            return Err(Error::Conflict(
                "government predecessor chain mismatch".to_owned(),
            ));
        }
    }
    let expected = digest_json(&serde_json::json!({
        "term_id": term.term_id, "project_id": project_id, "person_id": term.person_id,
        "position_id": term.position_id, "position": term.position,
        "predecessor_term_id": term.predecessor_term_id,
        "handoff_note": term.handoff_note, "started_at_unix_ms": term.started_at_unix_ms,
    }))?;
    if term.term_sha256 != expected {
        return Err(Error::Conflict(
            "government term digest mismatch".to_owned(),
        ));
    }
    if term.ended_at_unix_ms.is_some() != term.end_reason.is_some() {
        return Err(Error::Conflict(
            "government term lifecycle mismatch".to_owned(),
        ));
    }
    if term
        .ended_at_unix_ms
        .is_some_and(|ended| ended < term.started_at_unix_ms)
    {
        return Err(Error::Conflict(
            "government term ends before it starts".to_owned(),
        ));
    }
    Ok(term)
}

fn roster_for_project(
    connection: &Connection,
    project_id: &str,
    limit: u32,
) -> Result<GovernmentRoster> {
    let initialized: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM government_roster_state WHERE project_id = ?1)",
        [project_id],
        |row| row.get(0),
    )?;
    let current = connection
        .prepare(
            "SELECT term_id FROM government_terms WHERE project_id = ?1
             AND ended_at_unix_ms IS NULL ORDER BY position_id ASC",
        )?
        .query_map([project_id], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .iter()
        .map(|id| {
            let term = load_term(connection, id)?;
            let person = load_person(connection, &term.person_id)?;
            Ok(GovernmentIncumbent { person, term })
        })
        .collect::<Result<Vec<_>>>()?;
    let people = connection.prepare(
        "SELECT person_id FROM government_people WHERE project_id = ?1 ORDER BY sequence DESC LIMIT ?2",
    )?.query_map(params![project_id, limit], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .iter().map(|id| load_person(connection, id)).collect::<Result<Vec<_>>>()?;
    let terms = connection.prepare(
        "SELECT term_id FROM government_terms WHERE project_id = ?1 ORDER BY sequence DESC LIMIT ?2",
    )?.query_map(params![project_id, limit], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .iter().map(|id| load_term(connection, id)).collect::<Result<Vec<_>>>()?;
    Ok(GovernmentRoster {
        current,
        people,
        terms,
        positions: crate::government::positions(),
        initialized,
        advisory: true,
        grants_authority: false,
    })
}
