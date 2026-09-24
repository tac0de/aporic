const $ = (id) => document.getElementById(id);
const stateLabels = { awaiting_effect:"집행 중", awaiting_verification:"검증으로 이동", completed:"복명 완료", failed:"막힘", abandoned:"중단", open:"열림", closed:"닫힘", planned:"기동 예정", advisory:"권고", exited_ok:"정상 종료", signaled:"신호 종료", spawn_failed:"기동 실패" };
const eventLabels = { authority_granted:"권한 부여", action_reserved:"작업 예약", effect_recorded:"효과 기록", verification_recorded:"검증 기록", reservation_abandoned:"예약 중단", day_opened:"하루 시작", handoff_projected:"인계 투영", day_closed:"하루 종료", model_planned:"모델 선택", model_finished:"모델 종료" };
let reconnectDelay = 800;
let stopped = false;

function node(tag, className, text) { const element=document.createElement(tag); if(className) element.className=className; if(text!==undefined) element.textContent=text; return element; }
function short(value,length=9) { return value&&value.length>length?value.slice(0,length):value||"—"; }
function hash(value) { let result=2166136261; for(const char of value){ result^=char.charCodeAt(0); result=Math.imul(result,16777619); } return result>>>0; }
function setConnection(mode,label) { $("connectionState").className=`connection ${mode}`; $("connectionLabel").textContent=label; }
function authHeaders() { const token=sessionStorage.getItem("aporic-observatory-token"); return token?{Authorization:`Bearer ${token}`}:{ }; }

async function connect() {
  if(stopped) return;
  setConnection("","봉화 연결 중");
  try {
    const response=await fetch("/api/events",{headers:authHeaders(),cache:"no-store"});
    if(response.status===401){ $("authPanel").hidden=false; setConnection("error","열람패 필요"); return; }
    if(!response.ok||!response.body) throw new Error(`HTTP ${response.status}`);
    $("authPanel").hidden=true; setConnection("live","궁정 실시간"); reconnectDelay=800;
    const reader=response.body.getReader(); const decoder=new TextDecoder(); let buffer="";
    while(!stopped){ const {value,done}=await reader.read(); if(done) break; buffer+=decoder.decode(value,{stream:true}); let boundary; while((boundary=buffer.indexOf("\n\n"))>=0){ const block=buffer.slice(0,boundary); buffer=buffer.slice(boundary+2); consumeEvent(block); } }
    throw new Error("stream closed");
  } catch(error) { setConnection("error","봉화 재연결 중"); window.setTimeout(connect,reconnectDelay); reconnectDelay=Math.min(reconnectDelay*1.7,8000); }
}

function consumeEvent(block) {
  if(!block||block.startsWith(":")) return;
  const lines=block.split("\n"); const event=lines.find(line=>line.startsWith("event: "))?.slice(7); const data=lines.filter(line=>line.startsWith("data: ")).map(line=>line.slice(6)).join("\n");
  if(event==="snapshot"&&data) render(JSON.parse(data)); else if(event==="source_error") setConnection("error","장부 확인 필요");
}

function render(snapshot) {
  const {project,summary,sessions,agents,launches,streams}=snapshot;
  $("projectName").textContent=project.project_id; $("roleId").textContent=project.role_id; $("head").textContent=short(project.head); $("head").title=project.head;
  $("revision").textContent=`K${project.kernel_revision} · H${project.handoff_revision} · M${project.model_control_records}`; $("worktree").textContent=project.dirty?"변경 있음":"정갈함"; $("updatedAt").textContent=`${new Date().toLocaleTimeString("ko-KR")} 수신`;
  $("activeCount").textContent=summary.active; $("verifyCount").textContent=summary.awaiting_verification; $("completeCount").textContent=summary.completed; $("failedCount").textContent=summary.failed;
  renderBrief(sessions); renderCourt(agents,sessions); renderCards(agents,sessions,launches); renderStream("authorityStream",streams.authority); renderStream("continuityStream",streams.continuity); renderStream("modelStream",streams.model_control);
}

function renderBrief(sessions) {
  const current=sessions.find(session=>session.status==="open")||sessions[0];
  $("currentObjective").textContent=current?.objective||"열린 하루를 기다리고 있습니다.";
  $("nextAction").textContent=current?.next_action||current?.open_questions?.[0]||"기록된 다음 수 없음";
}

function courtFigure(agent,index,phaseIndex) {
  const seed=hash(agent.reservation_id); const phase=agent.phase; const columns={ awaiting_effect:[48,58], awaiting_verification:[78,45], completed:[51,24], failed:[22,72], abandoned:[16,65] }; const base=columns[phase]||[17,68];
  const offsetX=((phaseIndex%3)-1)*12; const offsetY=Math.floor(phaseIndex/3)*9; const button=node("button",`agent-token ${phase}`); button.type="button"; button.style.setProperty("--x",`${base[0]+offsetX+(seed%5)-2}%`); button.style.setProperty("--y",`${base[1]+offsetY}%`); button.setAttribute("aria-label",`${agent.principal}, ${stateLabels[phase]||phase}, ${agent.action}`);
  const avatar=node("span","avatar"); avatar.append(node("i","hat")); const copy=node("span","token-copy"); copy.append(node("b","",agent.principal),node("span","",agent.action),node("small","",`${stateLabels[phase]||phase} · ${agent.routing_tier}`)); button.append(avatar,copy); button.addEventListener("click",()=>showInspector(agent)); return button;
}

function renderCourt(agents) {
  const layer=$("peopleLayer"); layer.replaceChildren();
  if(!agents.length){ layer.append(node("p","court-empty","현재 투영된 에이전트 작업이 없습니다.")); return; }
  const phaseCounts={}; agents.slice(0,12).forEach((agent,index)=>{ const phaseIndex=phaseCounts[agent.phase]||0; phaseCounts[agent.phase]=phaseIndex+1; layer.append(courtFigure(agent,index,phaseIndex)); });
}

function showInspector(agent) {
  $("inspectorKicker").textContent=stateLabels[agent.phase]||agent.phase; $("inspectorName").textContent=agent.principal; const facts=$("inspectorFacts"); facts.replaceChildren();
  [["임무",agent.action],["모델 계층",agent.routing_tier],["작업",agent.task_ref],["세션",agent.session_ref],["예약",agent.reservation_id],["효과",agent.effect||"대기"],["검증",agent.verification||"대기"]].forEach(([label,value])=>{ const row=node("div"); row.append(node("dt","",label),node("dd","",value)); facts.append(row); });
  $("personInspector").hidden=false;
}

function detailCard(title,detail) { const card=node("div","detail-card"); card.append(node("b","",title),node("span","",detail)); return card; }
function renderCards(agents,sessions,launches) {
  fillCards("agentGrid",agents.slice(0,10).map(agent=>detailCard(`${agent.principal} · ${stateLabels[agent.phase]||agent.phase}`,`${agent.action} · ${agent.routing_tier}`)),"예약된 작업 없음");
  fillCards("sessionList",sessions.slice(0,8).map(day=>detailCard(day.objective||day.day_id,`${day.session_ref} · ${stateLabels[day.status]||day.status}`)),"열린 하루 없음");
  fillCards("modelList",launches.slice(0,8).map(launch=>detailCard(launch.model,`${launch.tier} · ${launch.reasoning_effort} · ${stateLabels[launch.status]||launch.status}`)),"모델 실행 기록 없음");
}
function fillCards(id,cards,empty) { const target=$(id); target.replaceChildren(); if(!cards.length) target.append(node("p","empty",empty)); else cards.forEach(card=>target.append(card)); }
function renderStream(id,items) { const target=$(id); target.replaceChildren(); if(!items.length){ target.append(node("li","empty","기록 없음")); return; } items.forEach(item=>{ const row=node("li"); row.append(node("span","seq",`#${item.sequence}`)); const body=node("div"); body.append(node("span","event-kind",eventLabels[item.kind]||item.kind),node("span","event-detail",`${item.subject} · ${item.detail}`)); row.append(body); target.append(row); }); }

$("closeInspector").addEventListener("click",()=>{ $("personInspector").hidden=true; });
$("toggleLedger").addEventListener("click",()=>{ const open=$("ledger").hidden; $("ledger").hidden=!open; $("toggleLedger").textContent=open?"장부 접기":"장부 펼치기"; $("toggleLedger").setAttribute("aria-expanded",String(open)); if(open) $("ledger").scrollIntoView({behavior:"smooth"}); });
$("authForm").addEventListener("submit",event=>{ event.preventDefault(); sessionStorage.setItem("aporic-observatory-token",$("tokenInput").value); $("authPanel").hidden=true; connect(); });
window.addEventListener("beforeunload",()=>{ stopped=true; });
connect();
