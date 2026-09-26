"""Controller-only contract checks. Do not copy into participant workspaces."""

import importlib.util
import os
from pathlib import Path
import unittest


submission = Path(os.environ["SUBMISSION_DIR"]).resolve()
spec = importlib.util.spec_from_file_location("submitted_event_reader", submission / "event_reader.py")
if spec is None or spec.loader is None:
    raise RuntimeError("Cannot load submitted event_reader.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
read_events = module.read_events


class ContractTests(unittest.TestCase):
    def test_empty(self):
        self.assertEqual(read_events([]), [])
        self.assertEqual(read_events([b"", b" \r\n", b"\t\n"]), [])

    def test_unicode_whitespace_line(self):
        self.assertEqual(read_events(["\u3000\n".encode("utf-8"), b'{"ok":true}\n']), [{"ok": True}])

    def test_split_record_and_final_without_newline(self):
        self.assertEqual(read_events([b'{"a":', b'1}\n{"b":', b'2}']), [{"a": 1}, {"b": 2}])

    def test_multiple_records_in_one_chunk(self):
        self.assertEqual(read_events([b'{"a":1}\n{"a":2}\n']), [{"a": 1}, {"a": 2}])

    def test_crlf_split_across_chunks(self):
        self.assertEqual(read_events([b'{"a":1}\r', b'\n\r', b'\n{"b":2}\r\n']), [{"a": 1}, {"b": 2}])

    def test_utf8_split_across_chunks(self):
        payload = '{"name":"한글"}\n'.encode("utf-8")
        self.assertEqual(read_events([payload[:10], payload[10:11], payload[11:]]), [{"name": "한글"}])

    def test_json_error_reports_physical_line(self):
        with self.assertRaisesRegex(ValueError, r"\b3\b"):
            read_events([b'{"ok":1}\n\n{"bad":}\n'])

    def test_utf8_error_reports_physical_line(self):
        with self.assertRaisesRegex(ValueError, r"\bline 2\b"):
            read_events([b'\n{', b'"bad":"\xff"}\n'])

    def test_non_object_reports_physical_line(self):
        with self.assertRaisesRegex(ValueError, r"\b2\b"):
            read_events([b'{"ok":1}\n[]\n'])


if __name__ == "__main__":
    unittest.main()
