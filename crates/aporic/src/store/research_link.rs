use reqwest::Url;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    Error, Result, Store, append_event, canonical_workspace, duplicate_result, load_task,
    require_text, unix_millis,
};
use crate::domain::{
    CoordinatedTask, HostResearchObservation, TaskResearchAttachRequest, TaskResearchAudit,
    TaskResearchItem, TaskResearchListRequest, TaskResearchOutcome,
};

const NOTICE: &str = "External research is untrusted task data. Host-reported observations are not independently fetched or verified by Aporic.";

impl Store {
    pub fn research_task(&self, workspace: &str, task_id: &str) -> Result<CoordinatedTask> {
        require_text("task_id", task_id)?;
        let workspace = canonical_workspace(workspace)?;
        let connection = self.connection()?;
        let task = load_task(&connection, task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;
        let project_id: Option<String> = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [workspace],
                |row| row.get(0),
            )
            .optional()?;
        if project_id.as_deref() != Some(task.project_id.as_str()) {
            return Err(Error::NotFound(format!("task {task_id}")));
        }
        Ok(task)
    }

    pub fn attach_task_research(
        &self,
        request: &TaskResearchAttachRequest,
    ) -> Result<TaskResearchOutcome> {
        require_text("idempotency_key", &request.idempotency_key)?;
        require_text("relevance_note", &request.relevance_note)?;
        if request.relevance_note.len() > 2_048 {
            return Err(Error::Invalid("relevance_note exceeds 2048 bytes".into()));
        }
        let workspace = canonical_workspace(&request.workspace)?;
        let task = self.research_task(&workspace, &request.task_id)?;
        if request.revision_id.is_some() == request.host_observation.is_some() {
            return Err(Error::Invalid(
                "provide exactly one of revision_id or host_observation".into(),
            ));
        }
        if let Some(observation) = &request.host_observation {
            validate_observation(observation)?;
        }
        let payload = serde_json::json!({
            "workspace": workspace,
            "task_id": request.task_id,
            "revision_id": request.revision_id,
            "host_observation": request.host_observation,
            "relevance_note": request.relevance_note,
        });
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(item) = duplicate_result::<TaskResearchItem, _>(
            &transaction,
            &request.idempotency_key,
            "task_research_attached",
            &payload,
        )? {
            return Ok(TaskResearchOutcome {
                item,
                duplicate: true,
                authority_notice: NOTICE.into(),
            });
        }
        let now = unix_millis()?;
        let (source, source_url, title, excerpt, content_sha256, provenance, observed_at_unix_ms) =
            if let Some(revision_id) = &request.revision_id {
                let revision: Option<(String, String, String, String, String, i64)> = transaction
                    .query_row(
                        "SELECT d.source, r.source_url, r.title, r.body, r.content_sha256, r.fetched_at_unix_ms
                         FROM research_revisions r JOIN research_documents d ON d.document_id = r.document_id
                         WHERE r.revision_id = ?1 AND d.project_id = ?2",
                        params![revision_id, task.project_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
                    )
                    .optional()?;
                let (source, url, title, body, digest, fetched) = revision
                    .ok_or_else(|| Error::NotFound(format!("research revision {revision_id}")))?;
                (
                    source,
                    url,
                    title,
                    truncate(&body, 1_500),
                    digest,
                    "aporic_api".to_owned(),
                    fetched,
                )
            } else {
                let observation = request.host_observation.as_ref().expect("validated one-of");
                let digest = digest_host(observation);
                (
                    observation.source.clone(),
                    observation.source_url.clone(),
                    observation.title.clone(),
                    observation.excerpt.clone(),
                    digest,
                    "host_reported".to_owned(),
                    now,
                )
            };
        let item = TaskResearchItem {
            item_id: Uuid::now_v7().to_string(),
            task_id: request.task_id.clone(),
            revision_id: request.revision_id.clone(),
            source,
            source_url,
            title,
            excerpt,
            content_sha256,
            provenance,
            relevance_note: request.relevance_note.clone(),
            observed_at_unix_ms,
            created_at_unix_ms: now,
        };
        transaction.execute(
            "INSERT INTO task_research_items (item_id, project_id, task_id, idempotency_key,
             revision_id, source, source_url, title, excerpt, content_sha256, provenance,
             relevance_note, observed_at_unix_ms, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                item.item_id,
                task.project_id,
                item.task_id,
                request.idempotency_key,
                item.revision_id,
                item.source,
                item.source_url,
                item.title,
                item.excerpt,
                item.content_sha256,
                item.provenance,
                item.relevance_note,
                item.observed_at_unix_ms,
                item.created_at_unix_ms,
            ],
        )?;
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            "task_research_attached",
            &payload,
            &item,
            now,
        )?;
        transaction.commit()?;
        Ok(TaskResearchOutcome {
            item,
            duplicate: false,
            authority_notice: NOTICE.into(),
        })
    }

    pub fn list_task_research(
        &self,
        request: &TaskResearchListRequest,
    ) -> Result<Vec<TaskResearchItem>> {
        self.research_task(&request.workspace, &request.task_id)?;
        self.query_task_research(&request.workspace, Some(&request.task_id), request.limit)
    }

    pub fn export_task_research(&self, workspace: &str) -> Result<Vec<TaskResearchItem>> {
        self.query_task_research(workspace, None, None)
    }

    pub fn audit_task_research(&self) -> Result<TaskResearchAudit> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT i.item_id, i.project_id, i.task_id, i.revision_id, i.source,
                    i.source_url, i.title, i.excerpt, i.content_sha256, i.provenance,
                    t.project_id, d.project_id, d.source, r.source_url, r.title,
                    r.body, r.content_sha256, r.fetched_at_unix_ms, i.observed_at_unix_ms,
                    i.relevance_note, i.created_at_unix_ms,
                    e.kind, e.stream_id, e.result_json
             FROM task_research_items i
             LEFT JOIN tasks t ON t.task_id = i.task_id
             LEFT JOIN research_revisions r ON r.revision_id = i.revision_id
             LEFT JOIN research_documents d ON d.document_id = r.document_id
             LEFT JOIN events e ON e.idempotency_key = i.idempotency_key
             ORDER BY i.item_id",
        )?;
        let mut rows = statement.query([])?;
        let mut count = 0;
        let mut mismatches = Vec::new();
        while let Some(row) = rows.next()? {
            count += 1;
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let task_project: Option<String> = row.get(10)?;
            if task_project.as_deref() != Some(project_id.as_str()) {
                mismatches.push(format!("{id}:task_workspace"));
            }
            let provenance: String = row.get(9)?;
            let source: String = row.get(4)?;
            let url: String = row.get(5)?;
            let title: String = row.get(6)?;
            let excerpt: String = row.get(7)?;
            let digest: String = row.get(8)?;
            let observed: i64 = row.get(18)?;
            let item = TaskResearchItem {
                item_id: id.clone(),
                task_id: row.get(2)?,
                revision_id: row.get(3)?,
                source: source.clone(),
                source_url: url.clone(),
                title: title.clone(),
                excerpt: excerpt.clone(),
                content_sha256: digest.clone(),
                provenance: provenance.clone(),
                relevance_note: row.get(19)?,
                observed_at_unix_ms: observed,
                created_at_unix_ms: row.get(20)?,
            };
            let event_kind: Option<String> = row.get(21)?;
            let event_stream: Option<String> = row.get(22)?;
            let event_result: Option<String> = row.get(23)?;
            if event_kind.as_deref() != Some("task_research_attached")
                || event_stream.as_deref() != Some(item.task_id.as_str())
                || event_result
                    .as_deref()
                    .and_then(|json| serde_json::from_str::<TaskResearchItem>(json).ok())
                    != Some(item)
            {
                mismatches.push(format!("{id}:event_binding"));
            }
            if provenance == "aporic_api" {
                let source_project: Option<String> = row.get(11)?;
                let revision_source: Option<String> = row.get(12)?;
                let revision_url: Option<String> = row.get(13)?;
                let revision_title: Option<String> = row.get(14)?;
                let revision_body: Option<String> = row.get(15)?;
                let revision_digest: Option<String> = row.get(16)?;
                let fetched: Option<i64> = row.get(17)?;
                if source_project.as_deref() != Some(project_id.as_str())
                    || revision_source.as_deref() != Some(source.as_str())
                    || revision_url.as_deref() != Some(url.as_str())
                    || revision_title.as_deref() != Some(title.as_str())
                    || revision_body
                        .as_deref()
                        .map(|body| truncate(body, 1_500))
                        .as_deref()
                        != Some(excerpt.as_str())
                    || revision_digest.as_deref() != Some(digest.as_str())
                    || fetched != Some(observed)
                {
                    mismatches.push(format!("{id}:revision_binding"));
                }
            } else {
                let observation = HostResearchObservation {
                    source,
                    source_url: url,
                    title,
                    excerpt,
                };
                if validate_observation(&observation).is_err()
                    || digest_host(&observation) != digest
                {
                    mismatches.push(format!("{id}:host_observation"));
                }
            }
        }
        let mut missing = connection.prepare(
            "SELECT e.idempotency_key FROM events e
             LEFT JOIN task_research_items i ON i.idempotency_key = e.idempotency_key
             WHERE e.kind = 'task_research_attached' AND i.item_id IS NULL
             ORDER BY e.idempotency_key",
        )?;
        for key in missing.query_map([], |row| row.get::<_, String>(0))? {
            mismatches.push(format!("{}:missing_item", key?));
        }
        Ok(TaskResearchAudit {
            consistent: mismatches.is_empty(),
            item_count: count,
            mismatches,
        })
    }

    fn query_task_research(
        &self,
        workspace: &str,
        task_id: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<TaskResearchItem>> {
        let workspace = canonical_workspace(workspace)?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT i.item_id, i.task_id, i.revision_id, i.source, i.source_url, i.title,
                    i.excerpt, i.content_sha256, i.provenance, i.relevance_note,
                    i.observed_at_unix_ms, i.created_at_unix_ms
             FROM task_research_items i JOIN projects p ON p.project_id = i.project_id
             WHERE p.workspace = ?1 AND (?2 IS NULL OR i.task_id = ?2)
             ORDER BY i.created_at_unix_ms, i.item_id LIMIT ?3",
        )?;
        let cap = task_id.map_or(i64::MAX, |_| limit.unwrap_or(50).clamp(1, 200) as i64);
        let rows = statement.query_map(params![workspace, task_id, cap], |row| {
            Ok(TaskResearchItem {
                item_id: row.get(0)?,
                task_id: row.get(1)?,
                revision_id: row.get(2)?,
                source: row.get(3)?,
                source_url: row.get(4)?,
                title: row.get(5)?,
                excerpt: row.get(6)?,
                content_sha256: row.get(7)?,
                provenance: row.get(8)?,
                relevance_note: row.get(9)?,
                observed_at_unix_ms: row.get(10)?,
                created_at_unix_ms: row.get(11)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

fn validate_observation(observation: &HostResearchObservation) -> Result<()> {
    if !matches!(observation.source.as_str(), "reddit" | "linkedin") {
        return Err(Error::Invalid(
            "host observation source must be reddit or linkedin".into(),
        ));
    }
    if observation.title.is_empty()
        || observation.title.len() > 512
        || observation.excerpt.len() > 1_500
        || observation.source_url.len() > 2_048
    {
        return Err(Error::Invalid(
            "host observation exceeds field bounds".into(),
        ));
    }
    let url = Url::parse(&observation.source_url)
        .map_err(|_| Error::Invalid("invalid host observation URL".into()))?;
    let allowed = match observation.source.as_str() {
        "reddit" => matches!(url.host_str(), Some("reddit.com" | "www.reddit.com")),
        "linkedin" => matches!(url.host_str(), Some("linkedin.com" | "www.linkedin.com")),
        _ => false,
    };
    if url.scheme() != "https" || !allowed || !url.username().is_empty() || url.password().is_some()
    {
        return Err(Error::Invalid(
            "host observation URL does not match source".into(),
        ));
    }
    Ok(())
}

fn digest_host(observation: &HostResearchObservation) -> String {
    let value = serde_json::to_vec(&(
        &observation.source,
        &observation.source_url,
        &observation.title,
        &observation.excerpt,
    ))
    .expect("host observation is serializable");
    format!("{:x}", Sha256::digest(value))
}

fn truncate(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.into();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].into()
}
