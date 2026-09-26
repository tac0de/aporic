"""Check that baseline and treatment expose the same canonical records."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "fixture"
canonical = [json.loads(line) for line in (ROOT / "canonical/history.jsonl").read_text().splitlines()]
treatment = [json.loads(line) for line in (ROOT / "treatment/import.jsonl").read_text().splitlines()]
assert canonical == treatment, "treatment import differs from canonical history"

notes = list((ROOT / "baseline/notes").glob("*.md"))
assert len(notes) == len(canonical), "baseline note count differs"
for record in canonical:
    matches = [
        path for path in notes
        if path.read_text().startswith(f"# {record['id']} — ")
    ]
    assert len(matches) == 1, f"expected one baseline note for {record['id']}"
    note = matches[0].read_text()
    assert record["text"] in note, f"text differs for {record['id']}"
    assert record["date"] in note, f"date missing for {record['id']}"
    assert record["topic"] in note, f"topic missing for {record['id']}"
    status = record["status"]
    if record["superseded_by"]:
        status += f" by {record['superseded_by']}"
    assert f"Status: {status}" in note, f"lifecycle differs for {record['id']}"

print(f"verified {len(canonical)} identical canonical records in both arms")
