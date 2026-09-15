#!/usr/bin/env python3
"""Run the retired entrypoint in an isolated repo, never against live history."""
import pathlib
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).with_name('task-pivot.sh')


class RetiredTaskPivot(unittest.TestCase):
    def test_refuses_without_modifying_or_provisioning_history(self):
        for historical in (False, True):
            with self.subTest(historical=historical), tempfile.TemporaryDirectory() as d:
                root = pathlib.Path(d)
                script = root / 'scripts/task-pivot.sh'
                script.parent.mkdir()
                shutil.copy2(SCRIPT, script)
                queue = root / 'core/projects/tasks/queue.jsonl'
                before = b'preserve malformed historical bytes\n'
                if historical:
                    queue.parent.mkdir(parents=True)
                    queue.write_bytes(before)
                result = subprocess.run(
                    ['bash', str(script), 'must not be admitted', '--status', 'queued'],
                    text=True, capture_output=True, timeout=10,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('retired', result.stderr)
                if historical:
                    self.assertEqual(queue.read_bytes(), before)
                else:
                    self.assertFalse((root / 'core').exists())


if __name__ == '__main__':
    unittest.main()
