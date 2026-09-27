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
SOURCE_ARCHIVE_SHA = "88f148250c0335c091b03c8e917cb92cc24d5d76f310294fc248f1dda3e8cadb"
PACKAGE_LOCK_SHA = "442865184207874c368ba8cde3089ca37a943e3d517799044d8e32313a84f5e4"
CACHE_ARCHIVE_SHA = "471eeebc8903e3efa6e14f2d6569adbbc836c33b46ad7064e894117c11e1d1d0"
PRODUCT = Path("src/features/quiz/quizFlowService.ts")
REGRESSION = Path("tests/e0-stage-regression.test.ts")
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


def audit_submission(source: Path, submission: Path) -> dict:
    """Compare the entire submitted tree to the immutable source export."""
    allowed = {PRODUCT, REGRESSION}
    source_files = {
        path.relative_to(source): path
        for path in source.rglob("*") if path.is_file()
    }
    submitted_files: dict[Path, Path] = {}
    violations: list[str] = []
    for path in submission.rglob("*"):
        relative = path.relative_to(submission)
        if relative.parts[0] == "node_modules":
            continue
        if path.is_symlink():
            violations.append(f"symlink:{relative}")
        elif path.is_file():
            submitted_files[relative] = path

    for relative, original in source_files.items():
        proposed = submitted_files.get(relative)
        if proposed is None:
            violations.append(f"deleted:{relative}")
        elif relative not in allowed and sha256(proposed) != sha256(original):
            violations.append(f"changed:{relative}")
    for relative in submitted_files.keys() - source_files.keys():
        if relative not in allowed:
            violations.append(f"added:{relative}")

    product = submitted_files.get(PRODUCT)
    regression = submitted_files.get(REGRESSION)
    if product is None or product.stat().st_size > 1_048_576:
        violations.append(f"missing_or_oversized:{PRODUCT}")
    if regression is None or not 0 < regression.stat().st_size <= 65_536:
        violations.append(f"missing_empty_or_oversized:{REGRESSION}")
    return {
        "scope_pass": not violations,
        "violations": sorted(violations)[:30],
        "violation_count": len(violations),
        "regression_present": regression is not None,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--submission", type=Path, required=True)
    parser.add_argument("--npm-cache-archive", type=Path, required=True)
    args = parser.parse_args()
    submission = args.submission.resolve()
    if not submission.is_dir():
        raise SystemExit(f"Missing submission tree: {submission}")
    if sha256(args.npm_cache_archive) != CACHE_ARCHIVE_SHA:
        raise SystemExit("Frozen npm cache archive digest mismatch")

    archive = subprocess.run(
        ["git", "-C", str(SOURCE_REPO), "archive", SOURCE_COMMIT],
        capture_output=True,
        check=True,
    ).stdout
    if hashlib.sha256(archive).hexdigest() != SOURCE_ARCHIVE_SHA:
        raise SystemExit("Source archive digest mismatch")
    with tempfile.TemporaryDirectory(prefix="aporic-e0-kakao-grade-") as folder:
        root = Path(folder)
        cache = root / "frozen-npm-cache"
        cache.mkdir()
        with tarfile.open(args.npm_cache_archive, "r:gz") as stream:
            stream.extractall(cache, filter="data")
        (root / "source").mkdir()
        with tarfile.open(fileobj=io.BytesIO(archive)) as stream:
            stream.extractall(root / "source", filter="data")
        root = root / "source"
        if sha256(root / "package-lock.json") != PACKAGE_LOCK_SHA:
            raise SystemExit("Source package lock digest mismatch")
        audit = audit_submission(root, submission)
        if not audit["scope_pass"]:
            print(json.dumps({"accepted_mechanical": False, **audit}))
            return
        source_file = submission / PRODUCT
        original_product = (root / PRODUCT).read_bytes()
        original_sha = sha256(root / PRODUCT)
        submitted_sha = sha256(source_file)
        (root / PRODUCT).write_bytes(source_file.read_bytes())
        (root / "tests/e0-hidden-stage.test.ts").write_bytes(HIDDEN.read_bytes())
        submitted_regression = submission / REGRESSION
        if submitted_regression.is_file() and not submitted_regression.is_symlink():
            (root / REGRESSION).write_bytes(submitted_regression.read_bytes())

        install = run(
            [
                "npm", "ci", "--offline", "--ignore-scripts", "--no-audit",
                "--no-fund", "--cache", str(cache),
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
        regression = (
            run(["node", "--import", "tsx", "--test", str(REGRESSION)], root, 120)
            if audit["regression_present"] else None
        )
        (root / PRODUCT).write_bytes(original_product)
        regression_on_source = (
            run(["node", "--import", "tsx", "--test", str(REGRESSION)], root, 120)
            if regression is not None and regression["exit_code"] == 0 else None
        )
        print(json.dumps({
            "source_commit": SOURCE_COMMIT,
            "source_archive_sha256": hashlib.sha256(archive).hexdigest(),
            "npm_cache_archive_sha256": sha256(args.npm_cache_archive),
            "package_lock_sha256": sha256(root / "package-lock.json"),
            "original_product_sha256": original_sha,
            "submitted_product_sha256": submitted_sha,
            "hidden_test_sha256": sha256(HIDDEN),
            **audit,
            "hidden_pass": hidden["exit_code"] == 0,
            "public_pass": public["exit_code"] == 0,
            "typecheck_pass": typecheck["exit_code"] == 0,
            "regression_pass": regression is not None and regression["exit_code"] == 0,
            "regression_detects_baseline": (
                regression_on_source is not None and regression_on_source["exit_code"] != 0
                and not regression_on_source["timed_out"]
            ),
            "accepted_mechanical": (
                audit["scope_pass"]
                and all(result["exit_code"] == 0 for result in (hidden, public, typecheck))
                and regression is not None and regression["exit_code"] == 0
                and regression_on_source is not None
                and regression_on_source["exit_code"] != 0
                and not regression_on_source["timed_out"]
            ),
            "checks": [hidden, public, typecheck, regression, regression_on_source],
        }))


if __name__ == "__main__":
    main()
