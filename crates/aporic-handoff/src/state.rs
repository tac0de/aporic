use crate::protocol::{Day, Event, HandoffCapsule, WorkspaceCheckpoint, handoff_sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_ID_BYTES: usize = 256;
const MAX_SCOPE_BYTES: usize = 1_024;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_ITEMS: usize = 32;
const MAX_CAPSULE_BYTES: usize = 32 * 1_024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectedHandoff {
    pub capsule: HandoffCapsule,
    pub workspace: WorkspaceCheckpoint,
    pub handoff_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayState {
    pub day: Day,
    pub opened_workspace: WorkspaceCheckpoint,
    pub latest_handoff: Option<ProjectedHandoff>,
    pub closed: bool,
    pub successor_day_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub revision: u64,
    pub days: BTreeMap<String, DayState>,
}

impl State {
    pub(crate) fn apply(&mut self, sequence: u64, event: &Event) -> Result<(), &'static str> {
        if sequence != self.revision + 1 {
            return Err("NONCONTIGUOUS_SEQUENCE");
        }
        match event {
            Event::DayOpened { day, workspace } => self.open_day(day, workspace)?,
            Event::HandoffProjected {
                day_id,
                capsule,
                workspace,
                handoff_sha256,
            } => self.project_handoff(day_id, capsule, workspace, handoff_sha256)?,
            Event::DayClosed {
                day_id,
                handoff_sha256,
                workspace,
            } => self.close_day(day_id, handoff_sha256, workspace)?,
        }
        self.revision = sequence;
        Ok(())
    }

    fn open_day(&mut self, day: &Day, workspace: &WorkspaceCheckpoint) -> Result<(), &'static str> {
        validate_day(day)?;
        validate_workspace(workspace)?;
        if self.days.contains_key(&day.day_id) {
            return Err("DAY_ID_CONFLICT");
        }
        if self.days.values().any(|candidate| {
            candidate.day.session_ref == day.session_ref && candidate.day.scope == day.scope
        }) {
            return Err("SESSION_ALREADY_RECORDED");
        }
        if self.days.values().any(|candidate| {
            candidate.day.scope == day.scope
                && candidate.day.lineage_ref == day.lineage_ref
                && !candidate.closed
        }) {
            return Err("LINEAGE_ALREADY_OPEN");
        }

        if let Some(predecessor_ref) = &day.predecessor {
            validate_id(&predecessor_ref.day_id)?;
            validate_digest(&predecessor_ref.handoff_sha256)?;
            let predecessor = self
                .days
                .get_mut(&predecessor_ref.day_id)
                .ok_or("PREDECESSOR_NOT_FOUND")?;
            if !predecessor.closed {
                return Err("PREDECESSOR_NOT_CLOSED");
            }
            if predecessor.day.scope != day.scope {
                return Err("PREDECESSOR_SCOPE_MISMATCH");
            }
            if predecessor.day.lineage_ref != day.lineage_ref {
                return Err("PREDECESSOR_LINEAGE_MISMATCH");
            }
            if predecessor.successor_day_id.is_some() {
                return Err("PREDECESSOR_ALREADY_INHERITED");
            }
            let handoff = predecessor
                .latest_handoff
                .as_ref()
                .ok_or("PREDECESSOR_HANDOFF_MISSING")?;
            if handoff.handoff_sha256 != predecessor_ref.handoff_sha256 {
                return Err("PREDECESSOR_HANDOFF_MISMATCH");
            }
            if handoff.workspace != *workspace {
                return Err("WORKSPACE_HANDSHAKE_MISMATCH");
            }
            predecessor.successor_day_id = Some(day.day_id.clone());
        } else if self.days.values().any(|candidate| {
            candidate.day.scope == day.scope && candidate.day.lineage_ref == day.lineage_ref
        }) {
            return Err("PREDECESSOR_REQUIRED");
        }

        self.days.insert(
            day.day_id.clone(),
            DayState {
                day: day.clone(),
                opened_workspace: workspace.clone(),
                latest_handoff: None,
                closed: false,
                successor_day_id: None,
            },
        );
        Ok(())
    }

    fn project_handoff(
        &mut self,
        day_id: &str,
        capsule: &HandoffCapsule,
        workspace: &WorkspaceCheckpoint,
        digest: &str,
    ) -> Result<(), &'static str> {
        validate_id(day_id)?;
        validate_capsule(capsule)?;
        validate_workspace(workspace)?;
        validate_digest(digest)?;
        let day = self.days.get_mut(day_id).ok_or("DAY_NOT_FOUND")?;
        if day.closed {
            return Err("DAY_ALREADY_CLOSED");
        }
        if workspace.binding_sha256 != day.opened_workspace.binding_sha256 {
            return Err("WORKSPACE_BINDING_MISMATCH");
        }
        let expected =
            handoff_sha256(&day.day, capsule, workspace).map_err(|_| "HANDOFF_HASH_FAILED")?;
        if expected != digest {
            return Err("HANDOFF_HASH_MISMATCH");
        }
        day.latest_handoff = Some(ProjectedHandoff {
            capsule: capsule.clone(),
            workspace: workspace.clone(),
            handoff_sha256: digest.into(),
        });
        Ok(())
    }

    fn close_day(
        &mut self,
        day_id: &str,
        digest: &str,
        workspace: &WorkspaceCheckpoint,
    ) -> Result<(), &'static str> {
        validate_id(day_id)?;
        validate_digest(digest)?;
        validate_workspace(workspace)?;
        let day = self.days.get_mut(day_id).ok_or("DAY_NOT_FOUND")?;
        if day.closed {
            return Err("DAY_ALREADY_CLOSED");
        }
        let handoff = day.latest_handoff.as_ref().ok_or("HANDOFF_REQUIRED")?;
        if handoff.handoff_sha256 != digest {
            return Err("HANDOFF_HASH_MISMATCH");
        }
        if handoff.workspace != *workspace {
            return Err("HANDOFF_STALE");
        }
        day.closed = true;
        Ok(())
    }
}

pub(crate) fn validate_request_text(value: &str) -> Result<(), &'static str> {
    validate_id(value)
}

fn validate_day(day: &Day) -> Result<(), &'static str> {
    validate_id(&day.day_id)?;
    validate_id(&day.lineage_ref)?;
    validate_id(&day.task_ref)?;
    validate_id(&day.session_ref)?;
    validate_scope(&day.scope)
}

fn validate_capsule(capsule: &HandoffCapsule) -> Result<(), &'static str> {
    validate_text(&capsule.objective)?;
    validate_items(&capsule.constraints)?;
    validate_items(&capsule.accepted_decisions)?;
    validate_items(&capsule.completed_checks)?;
    validate_items(&capsule.open_questions)?;
    validate_text(&capsule.next_action)?;
    let size = serde_json::to_vec(capsule)
        .map_err(|_| "INVALID_CAPSULE")?
        .len();
    if size > MAX_CAPSULE_BYTES {
        return Err("CAPSULE_TOO_LARGE");
    }
    Ok(())
}

fn validate_items(items: &[String]) -> Result<(), &'static str> {
    if items.len() > MAX_ITEMS {
        return Err("TOO_MANY_CAPSULE_ITEMS");
    }
    items.iter().try_for_each(|item| validate_text(item))
}

fn validate_text(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES {
        return Err("INVALID_CAPSULE_TEXT");
    }
    Ok(())
}

fn validate_workspace(workspace: &WorkspaceCheckpoint) -> Result<(), &'static str> {
    validate_digest(&workspace.binding_sha256)?;
    if !matches!(workspace.head.len(), 40 | 64)
        || !workspace.head.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("INVALID_WORKSPACE_HEAD");
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), &'static str> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("INVALID_DIGEST");
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > MAX_ID_BYTES {
        return Err("INVALID_ID");
    }
    Ok(())
}

fn validate_scope(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > MAX_SCOPE_BYTES {
        return Err("INVALID_SCOPE");
    }
    Ok(())
}
