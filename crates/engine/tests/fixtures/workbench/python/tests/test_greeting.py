import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from greeting import greeting


class GreetingTest(unittest.TestCase):
    def test_declared_greeting(self) -> None:
        self.assertEqual(greeting(), "hello")


if __name__ == "__main__":
    unittest.main()
