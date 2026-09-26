use rusqlite::{OptionalExtension, TransactionBehavior, params};
use uuid::Uuid;

use super::{
    Error, Result, Store, append_event, canonical_workspace, duplicate_result, load_task,
    require_text, unix_millis,
};
use crate::domain::{CoordinatedTask, TaskBriefReceipt, TaskBriefRequest};

impl Store {
    pub fn task_brief_task(&self, request: &TaskBriefRequest) -> Result<CoordinatedTask> {
        require_text("task_id", &request.task_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let task = load_task(&connection, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let project_id: Option<String> = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get(0),
            )
            .optional()?;
        if project_id.as_deref() != Some(task.project_id.as_str()) {
            return Err(Error::NotFound(format!("task {}", request.task_id)));
        }
        Ok(task)
    }

    pub fn record_task_brief(
        &self,
        request: &TaskBriefRequest,
        assembly: &crate::brief::Assembly,
        context_policy_sha256: &str,
    ) -> Result<(TaskBriefReceipt, bool)> {
        require_text("idempotency_key", &request.idempotency_key)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let payload = serde_json::json!({
            "workspace": workspace,
            "task_id": request.task_id,
            "max_context_bytes": request.max_context_bytes,
            "template_sha256": assembly.template_sha256,
            "context_policy_sha256": context_policy_sha256,
            "selected_item_ids": assembly.selected_item_ids,
            "brief_sha256": assembly.brief_sha256,
            "brief_bytes": assembly.text.len(),
        });
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = duplicate_result::<TaskBriefReceipt, _>(
            &transaction,
            &request.idempotency_key,
            "task_brief_recorded",
            &payload,
        )? {
            return Ok((receipt, true));
        }
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let project_id: Option<String> = transaction
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get(0),
            )
            .optional()?;
        if project_id.as_deref() != Some(task.project_id.as_str()) {
            return Err(Error::NotFound(format!("task {}", request.task_id)));
        }
        let receipt = TaskBriefReceipt {
            receipt_id: Uuid::now_v7().to_string(),
            task_id: request.task_id.clone(),
            template_id: assembly.template_id.clone(),
            template_version: assembly.template_version,
            template_sha256: assembly.template_sha256.clone(),
            context_policy_sha256: context_policy_sha256.to_owned(),
            selected_item_ids: assembly.selected_item_ids.clone(),
            brief_sha256: assembly.brief_sha256.clone(),
            brief_bytes: u32::try_from(assembly.text.len()).expect("bounded task brief"),
            created_at_unix_ms: now,
        };
        transaction.execute(
            "INSERT INTO task_brief_receipts
             (receipt_id, project_id, task_id, idempotency_key, template_id,
              template_version, template_sha256, context_policy_sha256,
              selected_item_ids_json, brief_sha256, brief_bytes, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                receipt.receipt_id,
                task.project_id,
                receipt.task_id,
                request.idempotency_key,
                receipt.template_id,
                receipt.template_version,
                receipt.template_sha256,
                receipt.context_policy_sha256,
                serde_json::to_string(&receipt.selected_item_ids)?,
                receipt.brief_sha256,
                receipt.brief_bytes,
                now,
            ],
        )?;
        append_event(
            &transaction,
            &request.idempotency_key,
            &receipt.task_id,
            "task_brief_recorded",
            &payload,
            &receipt,
            now,
        )?;
        transaction.commit()?;
        Ok((receipt, false))
    }

    pub fn list_task_brief_receipts(&self, raw_workspace: &str) -> Result<Vec<TaskBriefReceipt>> {
        let workspace = canonical_workspace(raw_workspace)?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT b.receipt_id, b.task_id, b.template_id, b.template_version,
                    b.template_sha256, b.context_policy_sha256, b.selected_item_ids_json,
                    b.brief_sha256, b.brief_bytes, b.created_at_unix_ms
             FROM task_brief_receipts b JOIN projects p ON p.project_id = b.project_id
             WHERE p.workspace = ?1 ORDER BY b.created_at_unix_ms, b.receipt_id",
        )?;
        let rows = statement.query_map([workspace], |row| {
            let ids: String = row.get(6)?;
            let selected_item_ids = serde_json::from_str(&ids).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(TaskBriefReceipt {
                receipt_id: row.get(0)?,
                task_id: row.get(1)?,
                template_id: row.get(2)?,
                template_version: row.get(3)?,
                template_sha256: row.get(4)?,
                context_policy_sha256: row.get(5)?,
                selected_item_ids,
                brief_sha256: row.get(7)?,
                brief_bytes: row.get(8)?,
                created_at_unix_ms: row.get(9)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}
