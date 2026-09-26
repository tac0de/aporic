import unittest

from event_reader import read_events


class PublicTests(unittest.TestCase):
    def test_two_complete_chunks(self):
        self.assertEqual(
            read_events([b'{"id":1}\n', b'{"id":2}\n']),
            [{"id": 1}, {"id": 2}],
        )

    def test_blank_lines(self):
        self.assertEqual(read_events([b'\n', b'  \n', b'{"id":3}\n']), [{"id": 3}])


if __name__ == "__main__":
    unittest.main()
