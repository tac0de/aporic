use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use uuid::Uuid;

use super::{
    Error, Result, Store, append_event, canonical_workspace, duplicate_result, load_task,
    parse_influence_class, parse_memory_lifecycle, require_text, unix_millis,
};
use crate::domain::{
    TaskMemoryUse, TaskMemoryUseListRequest, TaskMemoryUseOutcome, TaskMemoryUseRequest, TaskStatus,
};

impl Store {
    pub fn apply_task_memory(
        &self,
        request: &TaskMemoryUseRequest,
    ) -> Result<TaskMemoryUseOutcome> {
        for (name, value) in [
            ("task_id", &request.task_id),
            ("memory_id", &request.memory_id),
            ("criterion", &request.criterion),
            ("intended_action", &request.intended_action),
            ("idempotency_key", &request.idempotency_key),
        ] {
            require_text(name, value)?;
        }
        if request.intended_action.len() > 2_000 {
            return Err(Error::Invalid(
                "intended_action exceeds 2000 bytes".to_owned(),
            ));
        }
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<TaskMemoryUseOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "task_memory_applied",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        if task.status != TaskStatus::Queued {
            return Err(Error::Conflict(
                "memory must be linked during planning, before the task is claimed".to_owned(),
            ));
        }
        if !task
            .acceptance_criteria
            .iter()
            .any(|item| item == &request.criterion)
        {
            return Err(Error::Invalid(
                "criterion must exactly match a task acceptance criterion".to_owned(),
            ));
        }
        let memory = transaction
            .query_row(
                "SELECT project_id, lifecycle_state, valid_from_unix_ms, valid_until_unix_ms
                 FROM memory_items WHERE memory_id = ?1",
                [&request.memory_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("memory {}", request.memory_id)))?;
        if memory.0 != task.project_id {
            return Err(Error::Conflict(
                "memory belongs to another workspace".to_owned(),
            ));
        }
        if memory.1 != "active" || memory.2 > now || memory.3.is_some_and(|until| until <= now) {
            return Err(Error::Conflict("memory is not currently active".to_owned()));
        }
        let count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM task_memory_uses WHERE task_id = ?1",
            [&request.task_id],
            |row| row.get(0),
        )?;
        if count >= 8 {
            return Err(Error::Invalid(
                "a task may link at most 8 memories".to_owned(),
            ));
        }
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM task_memory_uses
             WHERE task_id = ?1 AND memory_id = ?2 AND criterion = ?3)",
            params![request.task_id, request.memory_id, request.criterion],
            |row| row.get(0),
        )?;
        if exists {
            return Err(Error::Conflict(
                "this memory is already linked to that criterion".to_owned(),
            ));
        }
        let use_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO task_memory_uses
             (use_id, project_id, task_id, memory_id, criterion, intended_action, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                use_id, task.project_id, request.task_id, request.memory_id,
                request.criterion, request.intended_action, now
            ],
        )?;
        let memory_use = load_use(&transaction, &use_id)?
            .ok_or_else(|| Error::NotFound(format!("memory use {use_id}")))?;
        let outcome = TaskMemoryUseOutcome {
            memory_use,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            "task_memory_applied",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn list_task_memory_uses(
        &self,
        request: &TaskMemoryUseListRequest,
    ) -> Result<Vec<TaskMemoryUse>> {
        require_text("task_id", &request.task_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound("workspace".to_owned()))?;
        let task = load_task(&connection, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        if task.project_id != project_id {
            return Err(Error::NotFound(format!("task {}", request.task_id)));
        }
        list_uses(&connection, &request.task_id)
    }
}

pub(super) fn list_uses(connection: &Connection, task_id: &str) -> Result<Vec<TaskMemoryUse>> {
    let mut statement = connection.prepare(
        "SELECT uses.use_id, uses.task_id, uses.memory_id, uses.criterion,
                uses.intended_action, memory.influence_class, memory.lifecycle_state,
                tasks.completion_proofs_json, uses.created_at_unix_ms
         FROM task_memory_uses uses
         JOIN memory_items memory ON memory.memory_id = uses.memory_id
         JOIN tasks ON tasks.task_id = uses.task_id
         WHERE uses.task_id = ?1
         ORDER BY uses.created_at_unix_ms, uses.use_id LIMIT 8",
    )?;
    let rows = statement.query_map([task_id], use_from_row)?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_use(connection: &Connection, use_id: &str) -> Result<Option<TaskMemoryUse>> {
    Ok(connection
        .query_row(
            "SELECT uses.use_id, uses.task_id, uses.memory_id, uses.criterion,
                    uses.intended_action, memory.influence_class, memory.lifecycle_state,
                    tasks.completion_proofs_json, uses.created_at_unix_ms
             FROM task_memory_uses uses
             JOIN memory_items memory ON memory.memory_id = uses.memory_id
             JOIN tasks ON tasks.task_id = uses.task_id
             WHERE uses.use_id = ?1",
            [use_id],
            use_from_row,
        )
        .optional()?)
}

fn use_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskMemoryUse> {
    let criterion: String = row.get(3)?;
    let proofs: String = row.get(7)?;
    let proofs: Vec<crate::domain::CriterionProof> =
        serde_json::from_str(&proofs).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    Ok(TaskMemoryUse {
        use_id: row.get(0)?,
        task_id: row.get(1)?,
        memory_id: row.get(2)?,
        criterion: criterion.clone(),
        intended_action: row.get(4)?,
        memory_influence_class: parse_influence_class(row.get(5)?)?,
        memory_lifecycle_state: parse_memory_lifecycle(row.get(6)?)?,
        criterion_verified_claim_id: proofs
            .into_iter()
            .find(|proof| proof.criterion == criterion)
            .map(|proof| proof.verified_claim_id),
        created_at_unix_ms: row.get(8)?,
        advisory: true,
    })
}
