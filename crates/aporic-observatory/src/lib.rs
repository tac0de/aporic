//! Read-only live observatory over Aporic's external ledgers.
//!
//! Court language belongs only to the bundled presentation. The JSON API uses
//! literal project, session, action, routing, effect, and verification terms.

use aporic_handoff::State as HandoffState;
use aporic_kernel::{EffectOutcome, State as KernelState, VerificationResult};
use aporic_model_control::{ApplicationPlan, AuditEvent, LaunchOutcome};
use aporic_projects::{ProjectBinding, observe_project};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const INDEX_HTML: &str = include_str!("../assets/index.html");
const APP_JS: &str = include_str!("../assets/app.js");
const STYLE_CSS: &str = include_str!("../assets/style.css");
const MAX_REQUEST_BYTES: usize = 8 * 1_024;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Kernel(aporic_kernel::Error),
    Handoff(aporic_handoff::Error),
    ModelControl(aporic_model_control::Error),
    Project(aporic_projects::Error),
    InvalidConfiguration(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::Kernel(error) => write!(formatter, "kernel error: {error}"),
            Self::Handoff(error) => write!(formatter, "handoff error: {error}"),
            Self::ModelControl(error) => write!(formatter, "model-control error: {error}"),
            Self::Project(error) => write!(formatter, "project error: {error}"),
            Self::InvalidConfiguration(reason) => {
                write!(formatter, "invalid observatory configuration: {reason}")
            }
        }
    }
}

impl std::error::Error for Error {}

macro_rules! from_error {
    ($source:ty, $variant:ident) => {
        impl From<$source> for Error {
            fn from(error: $source) -> Self {
                Self::$variant(error)
            }
        }
    };
}

from_error!(std::io::Error, Io);
from_error!(serde_json::Error, Json);
from_error!(aporic_kernel::Error, Kernel);
from_error!(aporic_handoff::Error, Handoff);
from_error!(aporic_model_control::Error, ModelControl);
from_error!(aporic_projects::Error, Project);

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Source {
    pub binding: ProjectBinding,
    pub role_id: String,
    pub scope: String,
    pub kernel_store: PathBuf,
    pub handoff_store: PathBuf,
    pub model_control_store: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind: SocketAddr,
    pub bearer_token: Option<String>,
}

impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
        if !self.bind.ip().is_loopback()
            && self
                .bearer_token
                .as_deref()
                .is_none_or(|token| token.len() < 24)
        {
            return Err(Error::InvalidConfiguration(
                "non-loopback binding requires a bearer token of at least 24 bytes",
            ));
        }
        if self.bearer_token.as_deref().is_some_and(|token| {
            token.len() > 512 || token.bytes().any(|byte| byte.is_ascii_control())
        }) {
            return Err(Error::InvalidConfiguration("bearer token is invalid"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    pub project: ProjectView,
    pub summary: Summary,
    pub sessions: Vec<SessionView>,
    pub agents: Vec<AgentView>,
    pub launches: Vec<LaunchView>,
    pub streams: Streams,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectView {
    pub project_id: String,
    pub role_id: String,
    pub scope: String,
    pub head: String,
    pub head_changed: bool,
    pub dirty: bool,
    pub kernel_revision: u64,
    pub handoff_revision: u64,
    pub model_control_records: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Summary {
    pub active: usize,
    pub awaiting_effect: usize,
    pub awaiting_verification: usize,
    pub completed: usize,
    pub failed: usize,
    pub open_sessions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionView {
    pub day_id: String,
    pub task_ref: String,
    pub session_ref: String,
    pub status: String,
    pub objective: Option<String>,
    pub next_action: Option<String>,
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentView {
    pub reservation_id: String,
    pub principal: String,
    pub task_ref: String,
    pub session_ref: String,
    pub action: String,
    pub routing_tier: String,
    pub routing_reasons: Vec<String>,
    pub phase: String,
    pub effect: Option<String>,
    pub verification: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaunchView {
    pub launch_id: String,
    pub tier: String,
    pub model: String,
    pub reasoning_effort: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Streams {
    pub authority: Vec<StreamItem>,
    pub continuity: Vec<StreamItem>,
    pub model_control: Vec<StreamItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StreamItem {
    pub sequence: u64,
    pub kind: String,
    pub subject: String,
    pub detail: String,
}

impl Source {
    pub fn snapshot(&self) -> Result<Snapshot> {
        let observation = observe_project(&self.binding)?;
        let kernel = aporic_kernel::load(&self.kernel_store)?;
        let handoff = aporic_handoff::load(&self.handoff_store)?;
        let model_control = aporic_model_control::load_audit(&self.model_control_store)?;
        let agents = agents(kernel.state());
        let sessions = sessions(handoff.state());
        let launches = launches(model_control.records());
        let summary = summarize(&agents, &sessions);
        Ok(Snapshot {
            project: ProjectView {
                project_id: observation.project_id,
                role_id: self.role_id.clone(),
                scope: self.scope.clone(),
                head: observation.head,
                head_changed: observation.head_changed,
                dirty: observation.dirty,
                kernel_revision: kernel.state().revision,
                handoff_revision: handoff.state().revision,
                model_control_records: model_control.records().len(),
            },
            summary,
            sessions,
            agents,
            launches,
            streams: Streams {
                authority: kernel_stream(kernel.records()),
                continuity: handoff_stream(handoff.records()),
                model_control: model_stream(model_control.records()),
            },
        })
    }
}

fn agents(state: &KernelState) -> Vec<AgentView> {
    state
        .reservations
        .values()
        .rev()
        .map(|state| {
            let effect = state
                .effect
                .as_ref()
                .map(|record| enum_name(record.outcome));
            let verification = state
                .verification
                .as_ref()
                .map(|record| enum_name(record.result));
            let phase = if state.abandonment_reason.is_some() {
                "abandoned"
            } else if matches!(
                state.verification.as_ref().map(|record| record.result),
                Some(VerificationResult::Passed)
            ) {
                "completed"
            } else if matches!(
                state.verification.as_ref().map(|record| record.result),
                Some(VerificationResult::Failed)
            ) || matches!(
                state.effect.as_ref().map(|record| record.outcome),
                Some(EffectOutcome::Failed)
            ) {
                "failed"
            } else if state.effect.is_some() {
                "awaiting_verification"
            } else {
                "awaiting_effect"
            };
            AgentView {
                reservation_id: state.reservation.reservation_id.clone(),
                principal: state.reservation.principal.clone(),
                task_ref: state.reservation.task_ref.clone(),
                session_ref: state.reservation.session_ref.clone(),
                action: state.reservation.action.clone(),
                routing_tier: state.reservation.routing_tier.clone(),
                routing_reasons: state.reservation.routing_reasons.clone(),
                phase: phase.into(),
                effect,
                verification,
            }
        })
        .collect()
}

fn sessions(state: &HandoffState) -> Vec<SessionView> {
    state
        .days
        .values()
        .rev()
        .map(|state| SessionView {
            day_id: state.day.day_id.clone(),
            task_ref: state.day.task_ref.clone(),
            session_ref: state.day.session_ref.clone(),
            status: if state.closed { "closed" } else { "open" }.into(),
            objective: state
                .latest_handoff
                .as_ref()
                .map(|handoff| handoff.capsule.objective.clone()),
            next_action: state
                .latest_handoff
                .as_ref()
                .map(|handoff| handoff.capsule.next_action.clone()),
            open_questions: state
                .latest_handoff
                .as_ref()
                .map(|handoff| handoff.capsule.open_questions.clone())
                .unwrap_or_default(),
        })
        .collect()
}

fn launches(records: &[aporic_model_control::StoredAuditEvent]) -> Vec<LaunchView> {
    let mut launches = BTreeMap::new();
    for record in records {
        match &record.event {
            AuditEvent::Planned { launch_id, plan } => {
                launches.insert(
                    launch_id.clone(),
                    LaunchView {
                        launch_id: launch_id.clone(),
                        tier: enum_name(plan.tier),
                        model: plan.model.clone(),
                        reasoning_effort: plan.reasoning_effort.clone(),
                        status: match plan.application {
                            ApplicationPlan::Advisory => "advisory",
                            _ => "planned",
                        }
                        .into(),
                    },
                );
            }
            AuditEvent::Finished { launch_id, outcome } => {
                if let Some(launch) = launches.get_mut(launch_id) {
                    launch.status = match outcome {
                        LaunchOutcome::Exited { code: 0 } => "exited_ok".into(),
                        LaunchOutcome::Exited { code } => format!("exited_{code}"),
                        LaunchOutcome::Signaled => "signaled".into(),
                        LaunchOutcome::SpawnFailed { .. } => "spawn_failed".into(),
                    };
                }
            }
        }
    }
    launches.into_values().rev().collect()
}

fn summarize(agents: &[AgentView], sessions: &[SessionView]) -> Summary {
    let mut summary = Summary {
        active: agents
            .iter()
            .filter(|agent| {
                matches!(
                    agent.phase.as_str(),
                    "awaiting_effect" | "awaiting_verification"
                )
            })
            .count(),
        open_sessions: sessions
            .iter()
            .filter(|session| session.status == "open")
            .count(),
        ..Summary::default()
    };
    for agent in agents {
        match agent.phase.as_str() {
            "awaiting_effect" => summary.awaiting_effect += 1,
            "awaiting_verification" => summary.awaiting_verification += 1,
            "completed" => summary.completed += 1,
            "failed" => summary.failed += 1,
            _ => {}
        }
    }
    summary
}

fn kernel_stream(records: &[aporic_kernel::StoredEvent]) -> Vec<StreamItem> {
    records
        .iter()
        .rev()
        .take(16)
        .map(|record| match &record.request.event {
            aporic_kernel::Event::AuthorityGranted { grant } => StreamItem {
                sequence: record.sequence,
                kind: "authority_granted".into(),
                subject: grant.principal.clone(),
                detail: grant.action.clone(),
            },
            aporic_kernel::Event::ActionReserved { reservation } => StreamItem {
                sequence: record.sequence,
                kind: "action_reserved".into(),
                subject: reservation.principal.clone(),
                detail: format!("{} · {}", reservation.action, reservation.routing_tier),
            },
            aporic_kernel::Event::EffectRecorded {
                reservation_id,
                outcome,
                ..
            } => StreamItem {
                sequence: record.sequence,
                kind: "effect_recorded".into(),
                subject: reservation_id.clone(),
                detail: enum_name(*outcome),
            },
            aporic_kernel::Event::VerificationRecorded {
                reservation_id,
                result,
                ..
            } => StreamItem {
                sequence: record.sequence,
                kind: "verification_recorded".into(),
                subject: reservation_id.clone(),
                detail: enum_name(*result),
            },
            aporic_kernel::Event::ReservationAbandoned {
                reservation_id,
                reason,
            } => StreamItem {
                sequence: record.sequence,
                kind: "reservation_abandoned".into(),
                subject: reservation_id.clone(),
                detail: reason.clone(),
            },
        })
        .collect()
}

fn handoff_stream(records: &[aporic_handoff::StoredEvent]) -> Vec<StreamItem> {
    records
        .iter()
        .rev()
        .take(12)
        .map(|record| match &record.request.event {
            aporic_handoff::Event::DayOpened { day, .. } => StreamItem {
                sequence: record.sequence,
                kind: "day_opened".into(),
                subject: day.day_id.clone(),
                detail: day.session_ref.clone(),
            },
            aporic_handoff::Event::HandoffProjected {
                day_id, capsule, ..
            } => StreamItem {
                sequence: record.sequence,
                kind: "handoff_projected".into(),
                subject: day_id.clone(),
                detail: capsule.next_action.clone(),
            },
            aporic_handoff::Event::DayClosed { day_id, .. } => StreamItem {
                sequence: record.sequence,
                kind: "day_closed".into(),
                subject: day_id.clone(),
                detail: "closed".into(),
            },
        })
        .collect()
}

fn model_stream(records: &[aporic_model_control::StoredAuditEvent]) -> Vec<StreamItem> {
    records
        .iter()
        .rev()
        .take(12)
        .map(|record| match &record.event {
            AuditEvent::Planned { launch_id, plan } => StreamItem {
                sequence: record.sequence,
                kind: "model_planned".into(),
                subject: launch_id.clone(),
                detail: format!(
                    "{} · {} · {}",
                    plan.model,
                    plan.reasoning_effort,
                    enum_name(plan.tier)
                ),
            },
            AuditEvent::Finished { launch_id, outcome } => StreamItem {
                sequence: record.sequence,
                kind: "model_finished".into(),
                subject: launch_id.clone(),
                detail: match outcome {
                    LaunchOutcome::Exited { code } => format!("exit {code}"),
                    LaunchOutcome::Signaled => "signaled".into(),
                    LaunchOutcome::SpawnFailed { .. } => "spawn failed".into(),
                },
            },
        })
        .collect()
}

fn enum_name<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".into())
}

pub fn serve(source: Source, config: ServerConfig) -> Result<()> {
    config.validate()?;
    let listener = TcpListener::bind(config.bind)?;
    let source = Arc::new(source);
    let token = Arc::new(config.bearer_token);
    for connection in listener.incoming() {
        let mut stream = connection?;
        let source = Arc::clone(&source);
        let token = Arc::clone(&token);
        std::thread::spawn(move || {
            let _ = handle(&mut stream, &source, token.as_deref());
        });
    }
    Ok(())
}

fn handle(stream: &mut TcpStream, source: &Source, token: Option<&str>) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let request = read_request(stream)?;
    let Some((path, authorization)) = parse_request(&request) else {
        return response(
            stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            "bad request",
        );
    };
    let authorized = token.is_none_or(|expected| {
        authorization
            .and_then(|value| value.strip_prefix("Bearer "))
            .is_some_and(|provided| constant_time_eq(provided.as_bytes(), expected.as_bytes()))
    });
    match path {
        "/" => response(stream, "200 OK", "text/html; charset=utf-8", INDEX_HTML),
        "/app.js" => response(stream, "200 OK", "text/javascript; charset=utf-8", APP_JS),
        "/style.css" => response(stream, "200 OK", "text/css; charset=utf-8", STYLE_CSS),
        "/api/snapshot" if authorized => {
            let encoded = serde_json::to_string(&source.snapshot()?)?;
            response(
                stream,
                "200 OK",
                "application/json; charset=utf-8",
                &encoded,
            )
        }
        "/api/events" if authorized => event_stream(stream, source),
        "/api/snapshot" | "/api/events" => response(
            stream,
            "401 Unauthorized",
            "application/json; charset=utf-8",
            "{\"error\":\"authentication_required\"}",
        ),
        _ => response(
            stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found",
        ),
    }
}

fn event_stream(stream: &mut TcpStream, source: &Source) -> Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'\r\n\r\n",
    )?;
    let mut previous = String::new();
    let mut heartbeat = 0_u8;
    loop {
        match source.snapshot() {
            Ok(snapshot) => {
                let encoded = serde_json::to_string(&snapshot)?;
                if encoded != previous {
                    stream.write_all(b"event: snapshot\ndata: ")?;
                    stream.write_all(encoded.as_bytes())?;
                    stream.write_all(b"\n\n")?;
                    stream.flush()?;
                    previous = encoded;
                    heartbeat = 0;
                } else if heartbeat >= 12 {
                    stream.write_all(b": keepalive\n\n")?;
                    stream.flush()?;
                    heartbeat = 0;
                } else {
                    heartbeat += 1;
                }
            }
            Err(error) => {
                let encoded = serde_json::to_string(&error.to_string())?;
                stream.write_all(format!("event: source_error\ndata: {encoded}\n\n").as_bytes())?;
                stream.flush()?;
            }
        }
        std::thread::sleep(Duration::from_millis(750));
    }
}

fn read_request(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    while bytes.len() <= MAX_REQUEST_BYTES {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::InvalidConfiguration(
            "HTTP request exceeds byte limit",
        ));
    }
    Ok(bytes)
}

fn parse_request(bytes: &[u8]) -> Option<(&str, Option<&str>)> {
    let request = std::str::from_utf8(bytes).ok()?;
    let mut lines = request.split("\r\n");
    let mut request_line = lines.next()?.split_whitespace();
    if request_line.next()? != "GET" {
        return None;
    }
    let path = request_line.next()?.split('?').next()?;
    if request_line.next()? != "HTTP/1.1" {
        return None;
    }
    let authorization = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("authorization")
            .then(|| value.trim())
    });
    Some((path, authorization))
}

fn response(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()?;
    Ok(())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let length = left.len().max(right.len());
    for index in 0..length {
        let a = left.get(index).copied().unwrap_or_default();
        let b = right.get(index).copied().unwrap_or_default();
        difference |= usize::from(a ^ b);
    }
    difference == 0
}

pub fn is_loopback(address: IpAddr) -> bool {
    address.is_loopback()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_binding_requires_a_long_token() {
        assert!(
            ServerConfig {
                bind: "0.0.0.0:4242".parse().unwrap(),
                bearer_token: None,
            }
            .validate()
            .is_err()
        );
        assert!(
            ServerConfig {
                bind: "127.0.0.1:4242".parse().unwrap(),
                bearer_token: None,
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn parses_only_bounded_get_requests_and_checks_tokens() {
        let request = b"GET /api/snapshot HTTP/1.1\r\nAuthorization: Bearer royal-secret\r\n\r\n";
        assert_eq!(
            parse_request(request),
            Some(("/api/snapshot", Some("Bearer royal-secret")))
        );
        assert!(constant_time_eq(b"royal-secret", b"royal-secret"));
        assert!(!constant_time_eq(b"royal-secret", b"other"));
    }
}
