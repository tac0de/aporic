"""Read JSON objects from an append-only, byte-chunked event stream."""

import json
from typing import Iterable


def read_events(chunks: Iterable[bytes]) -> list[dict]:
    """Return NDJSON objects from byte chunks.

    This initial version was written for an older producer that sent one whole
    newline-terminated record per chunk.
    """
    events = []
    for chunk in chunks:
        for raw_line in chunk.split(b"\n"):
            if not raw_line.strip():
                continue
            value = json.loads(raw_line.decode("utf-8"))
            if not isinstance(value, dict):
                raise ValueError("expected JSON object")
            events.append(value)
    return events
