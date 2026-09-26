"""Post-run audit of preregistration hashes, source facts and tool output."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sqlite3


HERE = Path(__file__).resolve().parent
ROOT = Path("/private/tmp/aporic-continuity-v2")


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def audit() -> dict:
    metadata = json.loads((ROOT / "runs/metadata.json").read_text())
    sources = {"manifest": HERE / "cases.json", "protocol": HERE / "PROTOCOL.md",
               "runner": HERE / "run.py", "schema": HERE / "answer.schema.json"}
    for name, path in sources.items():
        assert sha(path) == metadata[f"{name}_sha256"], f"frozen {name} changed"
    setup = json.loads((ROOT / "setup.json").read_text())
    assert setup["manifest_sha256"] == metadata["manifest_sha256"]
    cases = json.loads(sources["manifest"].read_text())["cases"]
    anomalies = []
    for case in cases:
        cid = case["id"]
        db = ROOT / cid / "treatment/aporic.sqlite3"
        con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
        assert con.execute("pragma integrity_check").fetchone()[0] == "ok", cid
        actual_records = con.execute("select record_id, content, supersedes_record_id from records where kind='decision'").fetchall()
        ids = {}
        texts = {}
        for record_id, content, supersedes in actual_records:
            prefix, text = content.split(": ", 1)
            assert prefix.startswith("Decision "), (cid, content)
            name = prefix.removeprefix("Decision ")
            ids[record_id] = name
            texts[name] = (text, supersedes)
        assert set(texts) == {item["id"] for item in case["decisions"]}, cid
        for decision in case["decisions"]:
            text, supersedes = texts[decision["id"]]
            assert text == decision["text"], (cid, decision["id"])
            assert (ids[supersedes] if supersedes else None) == decision["supersedes"], (cid, decision["id"])
        tasks = [row[0] for row in con.execute("select objective from tasks")]
        assert set(tasks) == {f"[{item['id']}] {item['objective']}" for item in case["tasks"]}, cid
        handoffs = con.execute("select summary,next_action from sessions where status='handoff'").fetchall()
        if case["handoff"]:
            h = case["handoff"]
            assert handoffs == [(f"Historical handoff for [{h['id']}].", f"[{h['id']}] {h['action']}")], cid
        else:
            assert handoffs == [], cid
        con.close()
        for arm in ("baseline", "treatment"):
            path = ROOT / "runs" / f"{cid}-{arm}.jsonl"
            events = [json.loads(line) for line in path.read_text().splitlines()]
            for event in events:
                if event.get("type") != "item.completed":
                    continue
                item = event.get("item", {})
                if item.get("type") != "command_execution":
                    continue
                problem = None
                if item.get("exit_code") != 0 or item.get("status") != "completed":
                    problem = "command_failed"
                elif "aporic_read.py" in item.get("command", ""):
                    try:
                        response = json.loads(item.get("aggregated_output", ""))
                        if response.get("ok") is not True:
                            problem = "mcp_not_ok"
                    except (ValueError, AttributeError):
                        problem = "mcp_empty_or_malformed"
                if problem:
                    anomalies.append({"case_id": cid, "arm": arm,
                                      "command": item.get("command"), "problem": problem})
    return {"hashes_match": True, "source_facts_match": True,
            "case_count": len(cases), "tool_anomalies": anomalies,
            "evaluator_sha256": sha(HERE / "evaluate.py")}


if __name__ == "__main__":
    report = audit()
    (ROOT / "audit.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
