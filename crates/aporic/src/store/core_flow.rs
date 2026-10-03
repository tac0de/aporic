use super::*;
use crate::{core_flow::*, domain::SessionDelegationDecisionRequest};

fn child_key(key: &str, operation: &str) -> String {
    let digest = Sha256::digest(key.as_bytes());
    format!("core-flow:{digest:x}:{operation}")
}
fn bounded(field: &str, value: &str, max: usize) -> Result<()> {
    require_text(field, value)?;
    if value.len() > max {
        return Err(Error::Invalid(format!("{field} exceeds {max} bytes")));
    }
    Ok(())
}
impl Store {
    pub fn begin(&self, request: &BeginRequest, kernel_sha256: &str) -> Result<BeginOutcome> {
        bounded("workspace", &request.workspace, 4096)?;
        bounded("objective", &request.objective, 2048)?;
        bounded("idempotency_key", &request.idempotency_key, 256)?;
        bounded("worker.reason", &request.work_shape.worker.reason, 1024)?;
        bounded("reviewer.reason", &request.work_shape.reviewer.reason, 1024)?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut previous) = duplicate_result::<BeginOutcome, _>(
            &tx,
            &request.idempotency_key,
            "flow_begun",
            request,
        )? {
            previous.duplicate = true;
            return Ok(previous);
        }
        let opened = Self::open_session_in(
            &tx,
            &OpenRequest {
                workspace: request.workspace.clone(),
                objective: request.objective.clone(),
                idempotency_key: child_key(&request.idempotency_key, "open"),
            },
            kernel_sha256,
            true,
        )?;
        let delegation = Self::assess_session_delegation_in(
            &tx,
            &SessionDelegationDecisionRequest {
                session_id: opened.session_id.clone(),
                parallel_paths: request.work_shape.parallel_paths,
                material_change: request.work_shape.material_change,
                worker: request.work_shape.worker.clone(),
                reviewer: request.work_shape.reviewer.clone(),
                idempotency_key: child_key(&request.idempotency_key, "assess"),
            },
        )?
        .decision;
        let count: u64 = tx.query_row("SELECT count(*) FROM sessions WHERE project_id=?1 AND status='open' AND abandoned=0 AND session_id!=?2",params![opened.project_id,opened.session_id],|r|r.get(0))?;
        let recovery = tx.prepare("SELECT session_id FROM sessions WHERE project_id=?1 AND status='open' AND abandoned=0 AND session_id!=?2 ORDER BY opened_at_unix_ms DESC,session_id DESC LIMIT 5")?
            .query_map(params![opened.project_id,opened.session_id],|r| Ok(RecoverySession {session_id:r.get(0)?,status:"open_unfinished".into()}))?
            .collect::<std::result::Result<Vec<_>,_>>()?;
        let context = opened.context;
        let mut result = BeginOutcome {session_id:opened.session_id.clone(),project_id:opened.project_id,context,delegation,runtime:crate::runtime::current(),open_repair_count:opened.open_repair_count,
            repair_notice: "For repair details use aporic_accountability_list with --profile full; history is advisory, host permissions govern.".into(),omitted_recovery_sessions:count.saturating_sub(recovery.len() as u64),recovery,response_limit_bytes:MAX_RESPONSE_BYTES,response_omitted_items:0,duplicate:false};
        while response_bytes(&result)? > MAX_RESPONSE_BYTES {
            let Some(item) = result.context.selected_items.pop() else {
                return Err(Error::Invalid(
                    "begin metadata exceeds response bound".into(),
                ));
            };
            result.context.budget.used_content_bytes -= item.content.len() as u32;
            result.context.budget.omitted_items += 1;
            result.response_omitted_items += 1;
            result.context.budget.conservative_input_token_upper_bound =
                result.context.budget.used_content_bytes;
        }
        append_event(
            &tx,
            &request.idempotency_key,
            &opened.session_id,
            "flow_begun",
            request,
            &result,
            unix_millis()?,
        )?;
        tx.commit()?;
        Ok(result)
    }
    pub fn finish(&self, request: &FinishRequest) -> Result<FinishOutcome> {
        bounded("session_id", &request.session_id, 128)?;
        bounded("summary", &request.summary, 2048)?;
        bounded("idempotency_key", &request.idempotency_key, 256)?;
        if let Some(action) = &request.next_action {
            bounded("next_action", action, 2048)?;
        }
        if request.notes.len() > MAX_NOTES {
            return Err(Error::Invalid("finish permits at most 8 notes".into()));
        }
        let mut total = 0;
        for note in &request.notes {
            bounded("note.content", &note.content, 2048)?;
            total += note.content.len();
            if let Some(evidence) = &note.evidence {
                bounded("note.evidence", evidence, 2048)?;
                total += evidence.len();
            }
            for id in [&note.supersedes_record_id, &note.verifies_effect_id]
                .into_iter()
                .flatten()
            {
                bounded("note link", id, 128)?;
            }
        }
        if total > 8192 {
            return Err(Error::Invalid("finish notes exceed 8192 bytes".into()));
        }
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut previous) = duplicate_result::<FinishOutcome, _>(
            &tx,
            &request.idempotency_key,
            "flow_finished",
            request,
        )? {
            previous.duplicate = true;
            previous.close.duplicate = true;
            return Ok(previous);
        }
        let mut ids = Vec::new();
        for (index, note) in request.notes.iter().enumerate() {
            let outcome = Self::record_in(
                &tx,
                &RecordRequest {
                    session_id: request.session_id.clone(),
                    kind: note.kind.clone(),
                    content: note.content.clone(),
                    evidence: note.evidence.clone(),
                    supersedes_record_id: note.supersedes_record_id.clone(),
                    verifies_effect_id: note.verifies_effect_id.clone(),
                    idempotency_key: child_key(&request.idempotency_key, &format!("note-{index}")),
                },
            )?;
            ids.push(outcome.record.record_id);
        }
        let close = Self::close_session_in(
            &tx,
            &CloseRequest {
                session_id: request.session_id.clone(),
                disposition: request.disposition.clone(),
                summary: request.summary.clone(),
                next_action: request.next_action.clone(),
                idempotency_key: child_key(&request.idempotency_key, "close"),
            },
        )?;
        let result = FinishOutcome {
            close,
            record_ids: ids,
            duplicate: false,
        };
        if response_bytes(&result)? > MAX_RESPONSE_BYTES {
            return Err(Error::Invalid("finish response exceeds bound".into()));
        }
        append_event(
            &tx,
            &request.idempotency_key,
            &request.session_id,
            "flow_finished",
            request,
            &result,
            unix_millis()?,
        )?;
        tx.commit()?;
        Ok(result)
    }
}
