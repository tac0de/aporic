"""Blind mechanical grader for the pre-fix Kakao stage fallback case."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile


SOURCE_REPO = Path("/Users/wonyoung_choi/projects/kakao-study-groupbot")
SOURCE_COMMIT = "671cb2df615a1df4151e5f090417e46773bf1043"
PRODUCT = Path("src/features/quiz/quizFlowService.ts")
HIDDEN = Path(__file__).with_name("hidden_kakao_stage.test.ts")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(argv: list[str], cwd: Path, timeout: int) -> dict:
    try:
        done = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, timeout=timeout)
        return {
            "command": argv,
            "exit_code": done.returncode,
            "stdout_tail": done.stdout[-6000:],
            "stderr_tail": done.stderr[-3000:],
            "timed_out": False,
        }
    except subprocess.TimeoutExpired as error:
        return {
            "command": argv,
            "exit_code": None,
            "stdout_tail": str(error.stdout or "")[-6000:],
            "stderr_tail": str(error.stderr or "")[-3000:],
            "timed_out": True,
        }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--submission", type=Path, required=True)
    parser.add_argument("--npm-cache", type=Path, required=True)
    args = parser.parse_args()
    source_file = args.submission / PRODUCT
    if source_file.is_symlink() or not source_file.is_file():
        raise SystemExit(f"Missing or symlinked submitted product file: {PRODUCT}")
    if source_file.stat().st_size > 1_048_576:
        raise SystemExit(f"Oversized submitted product file: {PRODUCT}")

    archive = subprocess.run(
        ["git", "-C", str(SOURCE_REPO), "archive", SOURCE_COMMIT],
        capture_output=True,
        check=True,
    ).stdout
    with tempfile.TemporaryDirectory(prefix="aporic-e0-kakao-grade-") as folder:
        root = Path(folder)
        with tarfile.open(fileobj=io.BytesIO(archive)) as stream:
            stream.extractall(root)
        original_sha = sha256(root / PRODUCT)
        submitted_sha = sha256(source_file)
        (root / PRODUCT).write_bytes(source_file.read_bytes())
        (root / "tests/e0-hidden-stage.test.ts").write_bytes(HIDDEN.read_bytes())

        install = run(
            [
                "npm", "ci", "--offline", "--ignore-scripts", "--no-audit",
                "--no-fund", "--cache", str(args.npm_cache),
            ],
            root,
            180,
        )
        if install["exit_code"] != 0:
            print(json.dumps({
                "instrument_error": "frozen_dependency_install_failed",
                "source_commit": SOURCE_COMMIT,
                "install": install,
            }))
            return
        hidden = run(
            ["node", "--import", "tsx", "--test", "tests/e0-hidden-stage.test.ts"],
            root,
            120,
        )
        public = run(
            ["node", "--import", "tsx", "--test", "tests/quiz-answer-routing.test.ts"],
            root,
            120,
        )
        typecheck = run(["npm", "run", "typecheck:all"], root, 180)
        print(json.dumps({
            "source_commit": SOURCE_COMMIT,
            "original_product_sha256": original_sha,
            "submitted_product_sha256": submitted_sha,
            "hidden_test_sha256": sha256(HIDDEN),
            "hidden_pass": hidden["exit_code"] == 0,
            "public_pass": public["exit_code"] == 0,
            "typecheck_pass": typecheck["exit_code"] == 0,
            "accepted_mechanical": all(
                result["exit_code"] == 0 for result in (hidden, public, typecheck)
            ),
            "checks": [hidden, public, typecheck],
        }))


if __name__ == "__main__":
    main()
