import tempfile
import unittest
from pathlib import Path

from audit_jsonl_storage import audit


class IntegrityAuditTests(unittest.TestCase):
    def scan(self, content):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "events.jsonl"
            path.write_bytes(content)
            result = audit(path)
            self.assertEqual(path.read_bytes(), content)
            return result

    def test_complete_corruption_is_counted_without_record_content(self):
        result = self.scan(b'{"ok":1}\nprivate invalid record\n\xff\n\n')
        self.assertEqual(result["valid_records"], 1)
        self.assertEqual(result["malformed_records"], 2)
        self.assertEqual(result["blank_lines"], 1)
        self.assertNotIn("private", str(result))
        self.assertFalse(result["scan_inconclusive"])

    def test_partial_active_tail_is_not_classified_as_corruption(self):
        result = self.scan(b'{}\n{"unfinished":')
        self.assertEqual(result["malformed_records"], 0)
        self.assertFalse(result["unterminated_tail"]["valid_json"])
        self.assertEqual(result["unterminated_tail"]["offset"], 3)

    def test_nonfinite_numbers_are_not_valid_json(self):
        result = self.scan(b'{"value":NaN}\n{"value":Infinity}\n')
        self.assertEqual(result["malformed_records"], 2)

    def test_valid_unterminated_record_and_empty_file(self):
        result = self.scan(b'{}')
        self.assertTrue(result["unterminated_tail"]["valid_json"])
        self.assertEqual(result["bytes_read"], 2)
        self.assertIsNone(self.scan(b'')["unterminated_tail"])


if __name__ == "__main__":
    unittest.main()
