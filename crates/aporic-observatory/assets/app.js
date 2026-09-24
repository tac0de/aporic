const $ = (id) => document.getElementById(id);
const stateLabels = {
  awaiting_effect: "집행 대기",
  awaiting_verification: "검증 대기",
  completed: "완료",
  failed: "실패",
  abandoned: "중단",
  open: "열림",
  closed: "닫힘",
  planned: "기동 예정",
  advisory: "권고",
  exited_ok: "정상 종료",
  signaled: "신호 종료",
  spawn_failed: "기동 실패",
};
const eventLabels = {
  authority_granted: "권한 부여",
  action_reserved: "작업 예약",
  effect_recorded: "효과 기록",
  verification_recorded: "검증 기록",
  reservation_abandoned: "예약 중단",
  day_opened: "하루 시작",
  handoff_projected: "인계 투영",
  day_closed: "하루 종료",
  model_planned: "모델 선택",
  model_finished: "모델 종료",
};

let reconnectDelay = 800;
let stopped = false;

function node(tag, className, text) {
  const element = document.createElement(tag);
  if (className) element.className = className;
  if (text !== undefined) element.textContent = text;
  return element;
}

function short(value, length = 9) {
  return value && value.length > length ? value.slice(0, length) : value || "—";
}

function setConnection(mode, label) {
  $("connectionState").className = `connection ${mode}`;
  $("connectionLabel").textContent = label;
}

function authHeaders() {
  const token = sessionStorage.getItem("aporic-observatory-token");
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function connect() {
  if (stopped) return;
  setConnection("", "봉화 연결 중");
  try {
    const response = await fetch("/api/events", { headers: authHeaders(), cache: "no-store" });
    if (response.status === 401) {
      $("authPanel").hidden = false;
      setConnection("error", "열람패 필요");
      return;
    }
    if (!response.ok || !response.body) throw new Error(`HTTP ${response.status}`);
    $("authPanel").hidden = true;
    setConnection("live", "봉화 연결됨");
    reconnectDelay = 800;
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    while (!stopped) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      let boundary;
      while ((boundary = buffer.indexOf("\n\n")) >= 0) {
        const block = buffer.slice(0, boundary);
        buffer = buffer.slice(boundary + 2);
        consumeEvent(block);
      }
    }
    throw new Error("stream closed");
  } catch (error) {
    setConnection("error", "연결 다시 시도 중");
    window.setTimeout(connect, reconnectDelay);
    reconnectDelay = Math.min(reconnectDelay * 1.7, 8000);
  }
}

function consumeEvent(block) {
  if (!block || block.startsWith(":")) return;
  const lines = block.split("\n");
  const event = lines.find((line) => line.startsWith("event: "))?.slice(7);
  const data = lines.filter((line) => line.startsWith("data: ")).map((line) => line.slice(6)).join("\n");
  if (event === "snapshot" && data) {
    render(JSON.parse(data));
  } else if (event === "source_error") {
    setConnection("error", "장부 확인 필요");
  }
}

function render(snapshot) {
  const { project, summary, sessions, agents, launches, streams } = snapshot;
  $("projectName").textContent = project.project_id;
  $("roleId").textContent = project.role_id;
  $("head").textContent = short(project.head);
  $("head").title = project.head;
  $("revision").textContent = `K${project.kernel_revision} · H${project.handoff_revision} · M${project.model_control_records}`;
  $("worktree").textContent = project.dirty ? "변경 있음" : "정갈함";
  $("worktree").className = project.dirty ? "dirty" : "clean";
  $("updatedAt").textContent = `${new Date().toLocaleTimeString("ko-KR")} 수신`;
  $("activeCount").textContent = summary.active;
  $("verifyCount").textContent = summary.awaiting_verification;
  $("completeCount").textContent = summary.completed;
  $("failedCount").textContent = summary.failed;
  renderDecision(sessions);
  renderAgents(agents);
  renderSessions(sessions);
  renderModels(launches);
  renderStream("authorityStream", streams.authority);
  renderStream("continuityStream", streams.continuity);
  renderStream("modelStream", streams.model_control);
}

function renderDecision(sessions) {
  const target = $("decisionBoard");
  target.replaceChildren();
  const current = sessions.find((session) => session.status === "open") || sessions[0];
  if (!current) return target.append(node("p", "empty", "열린 하루가 생기면 다음 판단이 이곳에 나타납니다."));
  const box = node("div", "judgement");
  box.append(node("p", "objective", current.objective || "현재 목표가 아직 투영되지 않았습니다."));
  box.append(node("p", "next", current.next_action ? `다음 수: ${current.next_action}` : "다음 수가 기록되지 않았습니다."));
  if (current.open_questions.length) {
    const list = node("ul", "questions");
    current.open_questions.slice(0, 4).forEach((question) => list.append(node("li", "", question)));
    box.append(list);
  }
  target.append(box);
}

function renderAgents(agents) {
  $("agentCount").textContent = `${agents.length}명`;
  const target = $("agentGrid");
  target.replaceChildren();
  if (!agents.length) return target.append(node("p", "empty", "예약된 작업이 없습니다."));
  agents.slice(0, 12).forEach((agent) => {
    const card = node("article", `agent-card ${agent.phase} ${agent.phase.startsWith("awaiting") ? "active" : ""}`);
    const top = node("div", "agent-top");
    const official = node("div", "official");
    official.append(node("div", "official-icon", "臣"));
    const identity = node("div");
    identity.append(node("p", "agent-name", agent.principal));
    identity.append(node("p", "agent-action", agent.action));
    official.append(identity);
    top.append(official, node("span", "state-chip", stateLabels[agent.phase] || agent.phase));
    const route = node("div", "agent-route");
    route.append(node("span", "", short(agent.reservation_id, 12)), node("i", "route-line"), node("span", "tier-chip", agent.routing_tier));
    card.append(top, route);
    target.append(card);
  });
}

function renderSessions(sessions) {
  $("sessionCount").textContent = `${sessions.length}일`;
  const target = $("sessionList");
  target.replaceChildren();
  if (!sessions.length) return target.append(node("p", "empty", "열린 세션이 없습니다."));
  sessions.slice(0, 6).forEach((session, index) => {
    const row = node("article", "session-row");
    row.append(node("div", "day-badge", String(sessions.length - index)));
    const info = node("div");
    info.append(node("p", "row-title", session.objective || session.day_id));
    info.append(node("p", "row-meta", session.session_ref));
    row.append(info, node("span", `row-status ${session.status}`, stateLabels[session.status] || session.status));
    target.append(row);
  });
}

function renderModels(launches) {
  const target = $("modelList");
  target.replaceChildren();
  if (!launches.length) return target.append(node("p", "empty", "모델 실행 기록이 없습니다."));
  launches.slice(0, 6).forEach((launch) => {
    const row = node("article", "model-row");
    row.append(node("div", "model-gauge", launch.tier.slice(0, 3)));
    const info = node("div");
    info.append(node("p", "row-title", launch.model));
    info.append(node("p", "row-meta", `${launch.reasoning_effort} · ${stateLabels[launch.status] || launch.status}`));
    row.append(info);
    target.append(row);
  });
}

function renderStream(id, items) {
  const target = $(id);
  target.replaceChildren();
  if (!items.length) return target.append(node("li", "empty", "기록 없음"));
  items.forEach((item) => {
    const row = node("li");
    row.append(node("span", "seq", `#${item.sequence}`));
    const body = node("div");
    body.append(node("span", "event-kind", eventLabels[item.kind] || item.kind));
    body.append(node("span", "event-detail", `${item.subject} · ${item.detail}`));
    row.append(body);
    target.append(row);
  });
}

$("authForm").addEventListener("submit", (event) => {
  event.preventDefault();
  sessionStorage.setItem("aporic-observatory-token", $("tokenInput").value);
  $("authPanel").hidden = true;
  connect();
});

window.addEventListener("beforeunload", () => { stopped = true; });
connect();
