"""Offline tests; synthetic source and secret canaries only."""
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

SOURCE = Path(__file__).resolve().parents[2] / "src/bin/snapshot_runtime/readonly_tools.py"

class SourceGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "src").mkdir()
        (self.root / "src/main.rs").write_text("fn main() {}\n")
        (self.root / ".env").write_text("SYNTHETIC_SECRET_CANARY=yes")
        (self.root / "src/credentials.json").write_text("SYNTHETIC_SECRET_CANARY")
        (self.root / "src/link.rs").symlink_to(self.root / ".env")
        (self.root / "src/outside.rs").symlink_to("/etc/passwd")
        scope = {}
        exec(SOURCE.read_text(), scope)
        self.registry = SimpleNamespace(dispatch=lambda *a, **k: self.fail("unfiltered dispatch"))
        tools = SimpleNamespace(get_tool_definitions=lambda **k: [])
        scope["enforce_readonly_tools"](tools, self.registry, self.root)

    def invoke(self, name, **args):
        return json.loads(self.registry.dispatch(name, args))

    def test_source_read_is_bounded(self):
        self.assertIn("fn main()", self.invoke("read_file", path="src/main.rs")["content"])
        for path in (".env", "src/credentials.json", "src/link.rs", "src/outside.rs", "/etc/passwd", "../other.rs"):
            with self.subTest(path=path):
                self.assertIn("error", self.invoke("read_file", path=path))

    def test_recursive_search_excludes_secrets_and_symlinks(self):
        found = self.invoke("search_files", pattern="fn main", path=".")
        self.assertEqual(len(found["matches"]), 1)
        denied = self.invoke("search_files", pattern="SYNTHETIC_SECRET_CANARY", path=".")
        self.assertEqual(denied["matches"], [])
        names = self.invoke("search_files", pattern="*", path=".", target="files")
        self.assertEqual(names["matches"], ["src/main.rs"])

    def test_mutations_and_unknown_arguments_fail_closed(self):
        self.assertIn("error", self.invoke("terminal", command="id"))
        self.assertIn("error", self.invoke("read_file", path="src/main.rs", unexpected=True))
        self.assertIn("error", self.invoke("search_files", pattern=".", path="src/link.rs"))

    def test_auth_containers_and_hardlinks_are_not_source(self):
        (self.root / "auth.json").write_text("SYNTHETIC_AUTH_CANARY")
        os.link(self.root / "auth.json", self.root / "src/copied.rs")
        for path in ("auth.json", "src/copied.rs"):
            self.assertIn("error", self.invoke("read_file", path=path))
        self.assertEqual(self.invoke("search_files", pattern="SYNTHETIC_AUTH_CANARY")["matches"], [])

    def test_extensionless_build_file_and_unsupported_options(self):
        (self.root / "Makefile").write_text("test:\n\ttrue\n")
        self.assertIn("test:", self.invoke("read_file", path="Makefile")["content"])
        for option in ("context", "order", "output_mode"):
            self.assertIn("error", self.invoke("search_files", pattern="main", **{option: "unsupported"}))

if __name__ == "__main__":
    unittest.main()
