"""Two fresh Codex sessions per arm, including authoring the history artifact."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import time


HERE = Path(__file__).resolve().parent
ROOT = Path("/private/tmp/aporic-restore-e2e-v1")
APORIC_BINARY = Path("/Users/wonyoung_choi/projects/aporic/target/release/aporic")
SOURCE = Path("crates/aporic/src/recovery.rs")
TEST = Path("crates/aporic/tests/recovery.rs")

COMMON = """This is an isolated copy of an actual Rust repository task.
Read TASK.md and the relevant source/tests. Do not inspect sibling workspaces,
the benchmark operator files, or the network. Do not commit, push, or deploy.
Keep product changes within recovery.rs and tests/recovery.rs.
"""
STAGE_ONE = """Stage 1 ends before editing product source. Investigate the
problem and preserve a concise handoff with the observed bug, proposed fix,
acceptance checks, and unresolved risks. A fresh session will implement it.
Do not edit recovery.rs or tests/recovery.rs yet.
"""
STAGE_TWO = """This is a fresh session. Recover your saved plan, inspect live
code, implement the correction, add a regression test, and run the relevant
mechanical checks if feasible. Do not claim success from intent alone.
"""
HISTORY = {
    ("baseline", 1): "Write the handoff to HANDOFF.md in this workspace.",
    ("baseline", 2): "Read HANDOFF.md for the prior session's plan.",
    ("treatment", 1): "Store the handoff with `python3 history.py handoff`, passing the note on stdin. Do not leave a duplicate handoff file.",
    ("treatment", 2): "Run `python3 history.py resume` and `python3 history.py recall` to recover the prior session's plan. Do not inspect the benchmark transcript file.",
}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(arm: str, stage: int, output: Path) -> dict:
    workspace = ROOT / arm
    prompt = COMMON + "\n" + (STAGE_ONE if stage == 1 else STAGE_TWO) + "\n" + HISTORY[(arm, stage)]
    command = ["codex", "exec", "--ephemeral", "--ignore-user-config",
               "--skip-git-repo-check", "--sandbox", "workspace-write", "--json",
               "-C", str(workspace), "-"]
    before = {str(path): sha(workspace / path) for path in (SOURCE, TEST)}
    started = time.monotonic_ns()
    timed_out = False
    try:
        process = subprocess.run(command, input=prompt, capture_output=True,
                                 text=True, timeout=180 if stage == 1 else 600)
        stdout, stderr, code = process.stdout, process.stderr, process.returncode
    except subprocess.TimeoutExpired as exc:
        timed_out = True
        stdout = exc.stdout.decode(errors="replace") if isinstance(exc.stdout, bytes) else (exc.stdout or "")
        stderr = exc.stderr.decode(errors="replace") if isinstance(exc.stderr, bytes) else (exc.stderr or "")
        code = None
    elapsed = (time.monotonic_ns() - started) / 1e9
    stem = output / f"{arm}-stage{stage}"
    stem.with_suffix(".jsonl").write_text(stdout)
    stem.with_suffix(".stderr.txt").write_text(stderr)
    events = []
    for line in stdout.splitlines():
        try:
            events.append(json.loads(line))
        except ValueError:
            pass
    usage = next((event.get("usage") for event in reversed(events)
                  if event.get("type") == "turn.completed"), None)
    messages = [event["item"].get("text") for event in events
                if event.get("type") == "item.completed"
                and event.get("item", {}).get("type") == "agent_message"]
    result = {"arm": arm, "stage": stage, "elapsed_seconds": round(elapsed, 6),
              "exit_code": code, "timed_out": timed_out, "usage": usage,
              "prompt_sha256": hashlib.sha256(prompt.encode()).hexdigest(),
              "command": command, "last_message": messages[-1] if messages else None,
              "tool_calls": sum(event.get("type") == "item.completed" and
                                event.get("item", {}).get("type") == "command_execution"
                                for event in events),
              "source_before": before,
              "source_after": {str(path): sha(workspace / path) for path in (SOURCE, TEST)},
              "command_events": [event["item"] for event in events
                                 if event.get("type") == "item.completed"
                                 and event.get("item", {}).get("type") == "command_execution"]}
    sibling = "treatment" if arm == "baseline" else "baseline"
    result["sibling_reference_events"] = [
        item for item in result["command_events"]
        if (str(ROOT / sibling) in item.get("command", "")
            or f"../{sibling}" in item.get("command", "")
            or str(ROOT / sibling) in item.get("aggregated_output", ""))
    ]
    stem.with_suffix(".json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    return result


def main() -> None:
    setup = json.loads((ROOT / "setup.json").read_text())
    assert setup["protocol_sha256"] == sha(HERE / "PROTOCOL.md")
    assert setup["hidden_test_sha256"] == sha(HERE / "hidden_recovery.rs")
    assert setup["helper_sha256"] == sha(HERE / "history.py")
    assert setup["prepare_sha256"] == sha(HERE / "prepare.py")
    assert setup["runner_sha256"] == sha(Path(__file__))
    assert setup["grader_sha256"] == sha(HERE / "grade.py")
    assert setup["aporic_binary_sha256"] == sha(APORIC_BINARY)
    for arm in ("baseline", "treatment"):
        workspace = ROOT / arm
        for path, key in ((SOURCE, "recovery_sha256"), (TEST, "tests_sha256")):
            assert sha(workspace / path) == setup["arms"][arm][key]
    output = ROOT / "runs"
    if output.exists():
        raise SystemExit("Refusing to overwrite existing runs")
    output.mkdir()
    metadata = {"protocol_sha256": sha(HERE / "PROTOCOL.md"),
                "prepare_sha256": sha(HERE / "prepare.py"),
                "run_sha256": sha(Path(__file__)),
                "hidden_test_sha256": sha(HERE / "hidden_recovery.rs"),
                "helper_sha256": sha(HERE / "history.py"),
                "codex_version": setup["codex_version"]}
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    for arm in ("baseline", "treatment"):
        for stage in (1, 2):
            result = execute(arm, stage, output)
            print(json.dumps({key: result[key] for key in
                              ("arm", "stage", "elapsed_seconds", "exit_code", "timed_out", "usage", "last_message")},
                             ensure_ascii=False), flush=True)
            if stage == 1:
                artifact = (ROOT / arm / "HANDOFF.md") if arm == "baseline" else (ROOT / arm / "aporic.sqlite3")
                print(json.dumps({"arm": arm, "history_artifact_exists": artifact.exists(),
                                  "history_artifact_bytes": artifact.stat().st_size if artifact.exists() else None}),
                      flush=True)


if __name__ == "__main__":
    main()
