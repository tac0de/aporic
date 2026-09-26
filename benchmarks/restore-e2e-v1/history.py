"""Isolated Aporic MCP continuity adapter used only by the treatment arm."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time


WORKSPACE = Path(__file__).resolve().parent
DATABASE = WORKSPACE / "aporic.sqlite3"
TRANSCRIPT = WORKSPACE / ".bench-history-transcript.jsonl"
APORIC = "/Users/wonyoung_choi/projects/aporic/target/release/aporic"


def exchange(name: str, args: dict) -> dict:
    env = {**os.environ, "APORIC_DATABASE": str(DATABASE)}
    process = subprocess.Popen(
        [APORIC, "mcp", "serve", "--stdio"], stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1,
        env=env,
    )
    assert process.stdin and process.stdout and process.stderr
    sequence = 0

    def call(method: str, params: dict) -> dict:
        nonlocal sequence
        sequence += 1
        process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": sequence,
                                        "method": method, "params": params}) + "\n")
        process.stdin.flush()
        while True:
            line = process.stdout.readline()
            if not line:
                raise RuntimeError(process.stderr.read())
            response = json.loads(line)
            if response.get("id") == sequence:
                if "error" in response:
                    raise RuntimeError(response["error"])
                return response

    try:
        call("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
                            "clientInfo": {"name": "restore-e2e-benchmark", "version": "1"}})
        process.stdin.write(json.dumps({"jsonrpc": "2.0",
                                        "method": "notifications/initialized"}) + "\n")
        process.stdin.flush()
        response = call("tools/call", {"name": name, "arguments": args})
        value = json.loads(response["result"]["content"][0]["text"])
        encoded = json.dumps(value, ensure_ascii=False, sort_keys=True).encode()
        entry = {"tool": name, "reply": value, "reply_bytes": len(encoded),
                 "reply_sha256": hashlib.sha256(encoded).hexdigest(),
                 "observed_at_unix_ns": time.time_ns()}
        with TRANSCRIPT.open("a") as stream:
            stream.write(json.dumps(entry, ensure_ascii=False) + "\n")
        if value.get("ok") is not True:
            raise RuntimeError(value)
        return value["result"]
    finally:
        process.stdin.close()
        process.terminate()
        process.wait(timeout=3)


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] not in {"handoff", "resume", "recall"}:
        raise SystemExit("usage: python3 history.py handoff|resume|recall")
    action = sys.argv[1]
    workspace = str(WORKSPACE)
    if action == "handoff":
        note = sys.stdin.read(16_385)
        if not note.strip() or len(note.encode()) > 16_384:
            raise SystemExit("handoff requires 1–16384 bytes on stdin")
        opened = exchange("aporic_open", {"workspace": workspace,
                                          "objective": "Protect restore destinations",
                                          "idempotency_key": "restore-e2e-stage1-open"})
        session_id = opened["session_id"]
        exchange("aporic_record", {"session_id": session_id, "kind": "task_progress",
                                   "content": note, "evidence": None,
                                   "supersedes_record_id": None, "verifies_effect_id": None,
                                   "idempotency_key": "restore-e2e-stage1-note"})
        closed = exchange("aporic_close", {
            "session_id": session_id, "disposition": "handoff",
            "summary": "Restore destination investigation and plan recorded.",
            "next_action": "Inspect the saved plan and implement the restore destination fix.",
            "idempotency_key": "restore-e2e-stage1-close",
        })
        print(json.dumps({"ok": True, "session_id": session_id,
                          "disposition": closed["disposition"]}))
    elif action == "resume":
        print(json.dumps(exchange("aporic_resume", {"workspace": workspace}),
                         ensure_ascii=False, indent=2))
    else:
        print(json.dumps(exchange("aporic_recall", {
            "workspace": workspace, "objective": "restore destination existing symlink plan",
            "limit": 20}), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
