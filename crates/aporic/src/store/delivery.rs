//! Runner-only delivery bindings reuse immutable events without a projection.
use super::*;
use crate::delivery_receipts::DeliveryRunBinding;

impl Store {
    pub(crate) fn delivery_session_workspace(&self, session_id: &str) -> Result<String> {
        require_text("session_id", session_id)?;
        let connection = self.connection()?;
        connection.query_row(
            "SELECT p.workspace FROM sessions s JOIN projects p USING(project_id) WHERE s.session_id=?1",
            [session_id], |row| row.get(0),
        ).optional()?.ok_or_else(|| Error::NotFound(format!("session {session_id}")))
    }

    pub(crate) fn delivery_command(&self, spec_id: &str) -> Result<CommandSpec> {
        let connection = self.connection()?;
        load_command_spec(&connection, spec_id)?
            .ok_or_else(|| Error::NotFound(format!("spec {spec_id}")))
    }

    pub(crate) fn delivery_binding(&self, run_id: &str) -> Result<Option<DeliveryRunBinding>> {
        let connection = self.connection()?;
        let row: Option<(String, String)> = connection.query_row(
            "SELECT payload_json,result_json FROM events WHERE idempotency_key=?1 AND kind='delivery_run_bound'",
            [format!("delivery-run-bound:{run_id}")],
            |row| Ok((row.get(0)?,row.get(1)?)),
        ).optional()?;
        row.map(|(payload, result)| {
            let binding: DeliveryRunBinding = serde_json::from_str(&result)?;
            if payload != result || binding.run_id != run_id {
                return Err(Error::Conflict("delivery binding event mismatch".into()));
            }
            Ok(binding)
        })
        .transpose()
    }

    // Only the local Rust delivery runner calls this. No MCP receipt ingestion.
    pub(crate) fn bind_delivery_run(&self, binding: &DeliveryRunBinding) -> Result<()> {
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let run = load_execution_outcome(&tx, &binding.run_id)?
            .ok_or_else(|| Error::NotFound("delivery run".into()))?;
        let spec = load_command_spec(&tx, &run.run.spec_id)?
            .ok_or_else(|| Error::NotFound("delivery spec".into()))?;
        if run.run.status != ExecutionStatus::Succeeded
            || run.verified_claim_id.is_none()
            || spec.workspace != binding.workspace
            || spec.canonical_sha256 != binding.command_spec_sha256
        {
            return Err(Error::Conflict(
                "delivery binding requires same-workspace successful runner receipt".into(),
            ));
        }
        if !run.artifacts.iter().any(|a| {
            a.workspace_relative_path == binding.result_path && a.sha256 == binding.result_sha256
        }) {
            return Err(Error::Conflict(
                "delivery result is not a captured runner artifact".into(),
            ));
        }
        let key = format!("delivery-run-bound:{}", binding.run_id);
        if duplicate_result::<DeliveryRunBinding, _>(&tx, &key, "delivery_run_bound", binding)?
            .is_none()
        {
            append_event(
                &tx,
                &key,
                &run.run.session_id,
                "delivery_run_bound",
                binding,
                binding,
                unix_millis()?,
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}
