"""Read JSON objects from an append-only, byte-chunked event stream."""

import json
from typing import Iterable


def read_events(chunks: Iterable[bytes]) -> list[dict]:
    """Return NDJSON objects from byte chunks.

    This initial version was written for an older producer that sent one whole
    newline-terminated record per chunk.
    """
    events = []
    pending = bytearray()
    line_number = 0

    def parse_line(raw_line: bytes, number: int) -> None:
        # LF is the record separator; remove its optional preceding CR.
        if raw_line.endswith(b"\r"):
            raw_line = raw_line[:-1]
        try:
            line = raw_line.decode("utf-8")
            if not line.strip():
                return
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError("expected JSON object")
        except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
            raise ValueError(f"invalid event on line {number}: {exc}") from exc
        events.append(value)

    for chunk in chunks:
        pending.extend(chunk)
        while True:
            newline = pending.find(b"\n")
            if newline < 0:
                break
            line_number += 1
            parse_line(bytes(pending[:newline]), line_number)
            del pending[: newline + 1]

    if pending:
        line_number += 1
        parse_line(bytes(pending), line_number)
    return events
