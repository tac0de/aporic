"""Blind mechanical grader: build original source plus submitted recovery implementation."""

from __future__ import annotations

from io import BytesIO
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import time


HERE = Path(__file__).resolve().parent
PROJECT = HERE.parents[1]
ROOT = Path(os.environ.get("APORIC_RESTORE_GRADE_ROOT", "/private/tmp/aporic-restore-e2e-v1"))
SOURCE_COMMIT = "aaf736aff7894594f12eec05ceac243b36deb92b"
SOURCE_FILE = Path("crates/aporic/src/recovery.rs")
TEST_FILE = Path("crates/aporic/tests/recovery.rs")


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def grade(arm: str) -> dict:
    assert arm in {"baseline", "treatment"}
    setup = json.loads((ROOT / "setup.json").read_text())
    assert setup["source_commit"] == SOURCE_COMMIT
    assert setup["hidden_test_sha256"] == sha(HERE / "hidden_recovery.rs")
    assert setup["protocol_sha256"] == sha(HERE / "PROTOCOL.md")
    assert setup["grader_sha256"] == sha(Path(__file__))
    submitted = ROOT / arm
    grading = ROOT / f"grade-{arm}"
    if grading.exists():
        raise RuntimeError(f"Refusing to overwrite {grading}")
    grading.mkdir()
    archive = subprocess.check_output(["git", "archive", "--format=tar", SOURCE_COMMIT], cwd=PROJECT)
    with tarfile.open(fileobj=BytesIO(archive), mode="r:") as tar:
        tar.extractall(grading, filter="data")
    tracked = subprocess.check_output(["git", "ls-tree", "-r", "--name-only", SOURCE_COMMIT],
                                      cwd=PROJECT, text=True).splitlines()
    scope_violations = []
    for name in tracked:
        if (name == "AGENTS.md" or name.startswith("benchmarks/") or
                name.startswith("examples/core-ab/") or name in {str(SOURCE_FILE), str(TEST_FILE)}):
            continue
        original = grading / name
        candidate = submitted / name
        if not candidate.is_file() or sha(candidate) != sha(original):
            scope_violations.append(name)
    for directory_name in ("crates/aporic/src", "crates/aporic/tests"):
        original_names = {str(path.relative_to(grading)) for path in (grading / directory_name).rglob("*") if path.is_file()}
        submitted_names = {str(path.relative_to(submitted)) for path in (submitted / directory_name).rglob("*") if path.is_file()}
        scope_violations.extend(sorted(submitted_names - original_names))
    original_test = (grading / TEST_FILE).read_text()
    submitted_test = (submitted / TEST_FILE).read_text()
    authored_test = (submitted_test != original_test and "symlink" in submitted_test
                     and "restore_to" in submitted_test)
    shutil.copy2(submitted / SOURCE_FILE, grading / SOURCE_FILE)
    (grading / TEST_FILE).write_text(submitted_test + "\n" + (HERE / "hidden_recovery.rs").read_text())
    started = time.monotonic()
    env = {**os.environ, "CARGO_TARGET_DIR": str(ROOT / "grader-target")}
    try:
        run = subprocess.run(["cargo", "test", "--offline", "--locked", "-p", "aporic",
                              "--test", "recovery"], cwd=grading, env=env,
                             capture_output=True, text=True, timeout=600)
        code, stdout, stderr, timed_out = run.returncode, run.stdout, run.stderr, False
    except subprocess.TimeoutExpired as exc:
        code = None
        stdout = exc.stdout.decode(errors="replace") if isinstance(exc.stdout, bytes) else (exc.stdout or "")
        stderr = exc.stderr.decode(errors="replace") if isinstance(exc.stderr, bytes) else (exc.stderr or "")
        timed_out = True
    result = {"arm": arm, "passed": code == 0 and authored_test and not scope_violations,
              "authored_test": authored_test, "scope_violations": scope_violations,
              "exit_code": code,
              "timed_out": timed_out, "elapsed_seconds": round(time.monotonic() - started, 6),
              "submitted_source_sha256": sha(submitted / SOURCE_FILE),
              "hidden_test_sha256": sha(HERE / "hidden_recovery.rs"),
              "command": "cargo test --offline --locked -p aporic --test recovery"}
    output = ROOT / "runs"
    (output / f"grade-{arm}.stdout.txt").write_text(stdout)
    (output / f"grade-{arm}.stderr.txt").write_text(stderr)
    (output / f"grade-{arm}.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


if __name__ == "__main__":
    results = [grade(arm) for arm in sys.argv[1:]]
    print(json.dumps(results, indent=2))
