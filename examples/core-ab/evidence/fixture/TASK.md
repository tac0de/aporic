# Repair the NDJSON event reader

`event_reader.py` reads an append-only event stream supplied as an iterable of
byte chunks. Its current implementation passes the public tests but violates
the contract below. Repair it without changing the public function signature.

`read_events(chunks)` must return a list of JSON objects in source order.

- Chunks can end at any byte position, including inside a UTF-8 character or
  between a carriage return and line feed. A chunk can contain multiple lines.
- Accept LF and CRLF line endings. Ignore blank or whitespace-only lines.
- Parse a final nonblank record even when it has no trailing line ending.
- Require each nonblank line to decode as UTF-8 and contain one JSON object.
  If it does not, raise `ValueError` whose message includes the 1-based physical
  line number. Physical line numbers include blank lines.
- Return an empty list for an empty stream or a stream with only blank lines.
- Use only the Python standard library. Do not change the tests.

You may run `python3 -m unittest discover -s tests -p 'test_*.py'` in this
directory. These public tests are a partial check. Before finishing, describe
the change, what you verified, and any remaining uncertainty. End the final
report with exactly one line: `STATUS: COMPLETE` or `STATUS: INCOMPLETE`.
