"""Post-run audit and descriptive scoring of the single preregistered pair."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE / "results"


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    setup = json.loads((ROOT / "setup.json").read_text())
    metadata = json.loads((ROOT / "runs/metadata.json").read_text())
    frozen = {"protocol_sha256": "PROTOCOL.md", "prepare_sha256": "prepare.py",
              "runner_sha256": "run.py", "grader_sha256": "grade.py",
              "helper_sha256": "history.py", "hidden_test_sha256": "hidden_recovery.rs"}
    for field, name in frozen.items():
        assert setup[field] == sha(HERE / name), f"frozen {name} changed"
    for field, value in metadata.items():
        if field in setup:
            assert setup[field] == value, field
    assert metadata["run_sha256"] == setup["runner_sha256"]
    assert setup["source_commit"] == "aaf736aff7894594f12eec05ceac243b36deb92b"
    result = {"source_commit": setup["source_commit"], "setup_seconds": setup["prepare_seconds"],
              "codex_version": setup["codex_version"], "arms": {}, "audit": {}}
    for arm in ("baseline", "treatment"):
        sessions = [json.loads((ROOT / "runs" / f"{arm}-stage{i}.json").read_text())
                    for i in (1, 2)]
        grade = json.loads((ROOT / "runs" / f"grade-{arm}.json").read_text())
        assert all(session["arm"] == arm and session["stage"] == i
                   for i, session in enumerate(sessions, 1))
        assert sessions[0]["source_before"] == sessions[0]["source_after"]
        assert all(not session["sibling_reference_events"] for session in sessions)
        assert all(session["exit_code"] == 0 and not session["timed_out"] and session["usage"]
                   for session in sessions)
        assert grade["arm"] == arm and grade["hidden_test_sha256"] == setup["hidden_test_sha256"]
        source = ROOT / "submissions" / arm / "crates/aporic/src/recovery.rs"
        tests = ROOT / "submissions" / arm / "crates/aporic/tests/recovery.rs"
        assert sha(source) == grade["submitted_source_sha256"]
        assert sha(source) == sessions[1]["source_after"]["crates/aporic/src/recovery.rs"]
        assert sha(tests) == sessions[1]["source_after"]["crates/aporic/tests/recovery.rs"]
        result["arms"][arm] = {
            "passed": grade["passed"], "authored_test": grade["authored_test"],
            "scope_violations": grade["scope_violations"],
            "stage_seconds": [session["elapsed_seconds"] for session in sessions],
            "total_seconds": round(sum(session["elapsed_seconds"] for session in sessions), 6),
            "stage_input_tokens": [session["usage"]["input_tokens"] for session in sessions],
            "total_input_tokens": sum(session["usage"]["input_tokens"] for session in sessions),
            "total_cached_input_tokens": sum(session["usage"]["cached_input_tokens"] for session in sessions),
            "total_output_tokens": sum(session["usage"]["output_tokens"] for session in sessions),
            "total_reasoning_output_tokens": sum(session["usage"]["reasoning_output_tokens"] for session in sessions),
            "total_tool_calls": sum(session["tool_calls"] for session in sessions),
            "source_sha256": sha(source), "test_sha256": sha(tests),
            "operator_grade_seconds": grade["elapsed_seconds"],
        }
    assert result["arms"]["baseline"]["source_sha256"] == result["arms"]["treatment"]["source_sha256"]
    b = result["arms"]["baseline"]
    t = result["arms"]["treatment"]
    result["descriptive_difference"] = {
        "treatment_minus_baseline_seconds": round(t["total_seconds"] - b["total_seconds"], 6),
        "treatment_minus_baseline_input_tokens": t["total_input_tokens"] - b["total_input_tokens"],
        "treatment_time_percent": round((t["total_seconds"] / b["total_seconds"] - 1) * 100, 2),
        "treatment_input_percent": round((t["total_input_tokens"] / b["total_input_tokens"] - 1) * 100, 2),
    }
    transcript = [json.loads(line) for line in (ROOT / "treatment-history-transcript.jsonl").read_text().splitlines()]
    assert [entry["tool"] for entry in transcript] == [
        "aporic_open", "aporic_record", "aporic_close", "aporic_resume", "aporic_recall"]
    for entry in transcript:
        encoded = json.dumps(entry["reply"], ensure_ascii=False, sort_keys=True).encode()
        assert entry["reply_bytes"] == len(encoded)
        assert entry["reply_sha256"] == hashlib.sha256(encoded).hexdigest()
        assert entry["reply"]["ok"] is True
    treatment_stage2 = json.loads((ROOT / "runs/treatment-stage2.json").read_text())
    for command, tool in (("history.py resume", "aporic_resume"),
                          ("history.py recall", "aporic_recall")):
        events = [item for item in treatment_stage2["command_events"]
                  if command in item["command"]]
        assert len(events) == 1 and events[0]["exit_code"] == 0
        reply = next(item["reply"]["result"] for item in transcript if item["tool"] == tool)
        assert json.loads(events[0]["aggregated_output"]) == reply
    result["audit"] = {"frozen_hashes_match": True, "stage_one_source_unchanged": True,
                       "observed_sibling_references": 0, "treatment_mcp_reply_capture_matches": True,
                       "baseline_handoff_bytes": (ROOT / "baseline-HANDOFF.md").stat().st_size,
                       "treatment_record_bytes": len(transcript[1]["reply"]["result"]["record"]["content"].encode()),
                       "treatment_database_bytes": (ROOT / "treatment-aporic.sqlite3").stat().st_size}
    (ROOT / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
