"""Read JSON objects from an append-only, byte-chunked event stream."""

import json
from typing import Iterable


def read_events(chunks: Iterable[bytes]) -> list[dict]:
    """Return JSON objects from an append-only, byte-chunked event stream."""
    events = []
    pending = bytearray()
    line_number = 1

    def parse_line(raw_line: bytes, number: int) -> None:
        # A CR is part of the line ending only when immediately before LF.
        if raw_line.endswith(b"\r"):
            raw_line = raw_line[:-1]
        try:
            line = raw_line.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise ValueError(f"line {number}: invalid UTF-8") from exc
        if not line.strip():
            return
        try:
            value = json.loads(line)
        except (json.JSONDecodeError, UnicodeError) as exc:
            raise ValueError(f"line {number}: invalid JSON") from exc
        if not isinstance(value, dict):
            raise ValueError(f"line {number}: expected JSON object")
        events.append(value)

    for chunk in chunks:
        pending.extend(chunk)
        while True:
            newline = pending.find(b"\n")
            if newline < 0:
                break
            parse_line(bytes(pending[:newline]), line_number)
            del pending[:newline + 1]
            line_number += 1

    if pending:
        parse_line(bytes(pending), line_number)
    return events
