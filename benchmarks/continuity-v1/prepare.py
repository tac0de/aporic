"""Prepare isolated, parity-checked workspaces for the frozen continuity batch."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import random
import subprocess
import sys
import time


HERE = Path(__file__).resolve().parent
MANIFEST = HERE / "cases.json"
OUTPUT = Path("/private/tmp/aporic-continuity-v1")
APORIC = Path("/Users/wonyoung_choi/projects/aporic/target/release/aporic")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def rpc_process(database: Path):
    env = {**os.environ, "APORIC_DATABASE": str(database)}
    process = subprocess.Popen(
        [str(APORIC), "mcp", "serve", "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        env=env,
    )
    assert process.stdin and process.stdout and process.stderr
    sequence = 0

    def call(method: str, params: dict):
        nonlocal sequence
        sequence += 1
        process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": sequence,
                                        "method": method, "params": params}) + "\n")
        process.stdin.flush()
        while True:
            line = process.stdout.readline()
            if not line:
                raise RuntimeError(process.stderr.read())
            reply = json.loads(line)
            if reply.get("id") == sequence:
                return reply

    call("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
                        "clientInfo": {"name": "continuity-bench-seed", "version": "1"}})
    process.stdin.write(json.dumps({"jsonrpc": "2.0",
                                    "method": "notifications/initialized"}) + "\n")
    process.stdin.flush()

    def tool(name: str, args: dict):
        reply = call("tools/call", {"name": name, "arguments": args})
        value = json.loads(reply["result"]["content"][0]["text"])
        if not value.get("ok"):
            raise RuntimeError(f"{name}: {value}")
        return value["result"]

    return process, tool


def baseline_note(case: dict) -> str:
    lines = ["# Dated project handoff", "", "This is historical project state; check state.json before acting.", ""]
    for decision in case["decisions"]:
        superseded_by = next((item["id"] for item in case["decisions"]
                              if item["supersedes"] == decision["id"]), None)
        label = f"superseded by {superseded_by}" if superseded_by else "current"
        lines.append(f"- Decision {decision['id']} ({label}): {decision['text']}")
    if case["decisions"]:
        lines.append("")
    for task in case["tasks"]:
        lines.append(f"- Queued task {task['id']}: {task['objective']}")
    if case["tasks"]:
        lines.append("")
    if case["handoff"]:
        handoff = case["handoff"]
        lines.append(f"- Last session handed off [{handoff['id']}]: {handoff['action']}")
    elif not case["tasks"]:
        lines.append("- Last session completed; it left no unfinished task or handoff.")
    lines.append("")
    return "\n".join(lines)


HELPER = '''"""Read-only view of this case's isolated Aporic MCP state."""
import json, os, subprocess, sys
from pathlib import Path
name = sys.argv[1] if len(sys.argv) > 1 else ""
if name not in {"resume", "recall", "tasks"}:
    raise SystemExit("usage: python3 aporic_read.py resume|recall|tasks")
workspace = str(Path(__file__).resolve().parent)
env = {**os.environ, "APORIC_DATABASE": str(Path(workspace) / "aporic.sqlite3")}
p = subprocess.Popen(["/Users/wonyoung_choi/projects/aporic/target/release/aporic", "mcp", "serve", "--stdio"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1, env=env)
sequence = 0
def call(method, params):
    global sequence
    sequence += 1
    p.stdin.write(json.dumps({"jsonrpc":"2.0","id":sequence,"method":method,"params":params})+"\\n")
    p.stdin.flush()
    while True:
        line = p.stdout.readline()
        if not line: raise RuntimeError(p.stderr.read())
        result = json.loads(line)
        if result.get("id") == sequence: return result
call("initialize", {"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"continuity-bench","version":"1"}})
p.stdin.write(json.dumps({"jsonrpc":"2.0","method":"notifications/initialized"})+"\\n");p.stdin.flush()
tool = {"resume":"aporic_resume","recall":"aporic_recall","tasks":"aporic_task_list"}[name]
args = {"workspace":workspace}
if name == "recall": args.update({"objective":"current task and applicable decision","limit":20})
response = call("tools/call", {"name":tool,"arguments":args})
print(json.dumps(json.loads(response["result"]["content"][0]["text"]),ensure_ascii=False,indent=2))
p.stdin.close();p.terminate();p.wait(timeout=3)
'''


def seed(case: dict, workspace: Path) -> dict:
    database = workspace / "aporic.sqlite3"
    process, tool = rpc_process(database)
    try:
        opened = tool("aporic_open", {"workspace": str(workspace),
                                     "objective": f"Historical work for {case['id']}",
                                     "idempotency_key": f"{case['id']}-seed-open"})
        session_id = opened["session_id"]
        record_ids = {}
        for decision in case["decisions"]:
            content = f"Decision {decision['id']}: {decision['text']}"
            prior = record_ids.get(decision["supersedes"])
            result = tool("aporic_record", {
                "session_id": session_id, "kind": "decision", "content": content,
                "evidence": None, "supersedes_record_id": prior,
                "verifies_effect_id": None,
                "idempotency_key": f"{case['id']}-{decision['id']}",
            })
            record_ids[decision["id"]] = result["record"]["record_id"]
        task_ids = []
        for task in case["tasks"]:
            result = tool("aporic_task_create", {
                "session_id": session_id,
                "objective": f"[{task['id']}] {task['objective']}",
                "acceptance_criteria": ["Complete the named task after checking live state"],
                "write_scope": [], "depends_on": [],
                "idempotency_key": f"{case['id']}-{task['id']}",
            })
            task_ids.append(result["task"]["task_id"])
        handoff = case["handoff"]
        if handoff:
            summary = f"Historical handoff for [{handoff['id']}]."
            next_action = f"[{handoff['id']}] {handoff['action']}"
            disposition = "handoff"
        else:
            summary = "Prior session closed; inspect queued tasks and live state."
            next_action = None
            disposition = "completed"
        tool("aporic_close", {"session_id": session_id,
                              "disposition": disposition, "summary": summary,
                              "next_action": next_action,
                              "idempotency_key": f"{case['id']}-seed-close"})
        resume = tool("aporic_resume", {"workspace": str(workspace)})
        return {"database": str(database), "session_id": session_id,
                "record_ids": record_ids, "task_ids": task_ids,
                "resume_status": resume["status"],
                "resume_selected": resume.get("selected")}
    finally:
        process.stdin.close()
        process.terminate()
        process.wait(timeout=3)


def main() -> None:
    manifest_bytes = MANIFEST.read_bytes()
    manifest = json.loads(manifest_bytes)
    cases = manifest["cases"]
    assert len(cases) == 8
    assert len({case["id"] for case in cases}) == 8
    assert {stratum: sum(case["stratum"] == stratum for case in cases)
            for stratum in {case["stratum"] for case in cases}} == {
        "clear_target": 2, "superseded_decision": 2,
        "ambiguous_or_absent": 2, "stale_live_state": 2,
    }
    if OUTPUT.exists():
        raise SystemExit(f"Refusing to overwrite existing {OUTPUT}")
    OUTPUT.mkdir(parents=True)
    order = ["baseline", "treatment"] * 4
    random.Random(manifest["seed"]).shuffle(order)
    setup = {"manifest_sha256": digest(manifest_bytes),
             "case_order": {case["id"]: arm for case, arm in zip(cases, order)},
             "cases": {}}
    for case in cases:
        started = time.monotonic()
        case_dir = OUTPUT / case["id"]
        baseline = case_dir / "baseline"
        treatment = case_dir / "treatment"
        baseline.mkdir(parents=True)
        treatment.mkdir()
        state = json.dumps(case["state"], indent=2) + "\n"
        for workspace in (baseline, treatment):
            (workspace / "state.json").write_text(state)
        note = baseline_note(case)
        (baseline / "HANDOFF.md").write_text(note)
        (treatment / "aporic_read.py").write_text(HELPER)
        seeded = seed(case, treatment)
        expected_resume = ("ambiguous" if case["stratum"] == "ambiguous_or_absent"
                           and case["tasks"] else
                           "none" if not case["tasks"] and not case["handoff"] else
                           "ready")
        assert seeded["resume_status"] == expected_resume, (case["id"], seeded)
        setup["cases"][case["id"]] = {
            "baseline_note_sha256": digest(note.encode()),
            "state_sha256": digest(state.encode()),
            "treatment_seed": seeded,
            "setup_seconds": round(time.monotonic() - started, 6),
        }
    (OUTPUT / "setup.json").write_text(json.dumps(setup, indent=2) + "\n")
    print(json.dumps({"output": str(OUTPUT), "manifest_sha256": setup["manifest_sha256"],
                      "orders": setup["case_order"]}, indent=2))


if __name__ == "__main__":
    main()
