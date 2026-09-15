#!/usr/bin/env python3
"""Isolated publication and consumer-limit regressions; no services touched."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("generator", Path(__file__).with_name("configure-retained-runtime.py"))
assert spec is not None and spec.loader is not None
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)


class PublicationTests(unittest.TestCase):
    def test_existing_file_is_revalidated_on_publication(self):
        with tempfile.TemporaryDirectory() as root:
            output = Path(root) / "output"
            with generator.pinned_directory(output, create=True) as fd:
                generator.publish(fd, "config", b"expected")
                generator.publish(fd, "config", b"expected")
                (output / "config").write_bytes(b"changed")
                with self.assertRaises(ValueError):
                    generator.publish(fd, "config", b"expected")
                self.assertEqual((output / "config").read_bytes(), b"changed")

    def test_parent_replacement_cannot_redirect_publication(self):
        with tempfile.TemporaryDirectory() as root:
            output = Path(root) / "output"
            other = Path(root) / "other"
            other.mkdir(mode=0o700)
            with generator.pinned_directory(output, create=True) as fd:
                output.rename(Path(root) / "original")
                output.symlink_to(other, target_is_directory=True)
                generator.publish(fd, "config", b"expected")
                self.assertFalse((other / "config").exists())
                self.assertEqual((Path(root) / "original/config").read_bytes(), b"expected")
            with self.assertRaises(OSError):
                with generator.pinned_directory(output, create=False):
                    pass

    def test_fifo_symlink_and_hardlink_rejected_without_blocking(self):
        with tempfile.TemporaryDirectory() as root:
            output = Path(root)
            with generator.pinned_directory(output, create=False) as fd:
                os.mkfifo(output / "fifo", 0o600)
                (output / "link").symlink_to("fifo")
                (output / "regular").write_bytes(b"expected")
                (output / "regular").chmod(0o600)
                os.link(output / "regular", output / "hard")
                for name in ("fifo", "link", "hard"):
                    with self.assertRaises((OSError, ValueError)):
                        generator.publish(fd, name, b"expected")

    def test_transport_budget_limits(self):
        legacy = dict(schema_version="arda.hermes-adapter.v1", adapter_version="test",
                      max_timeout_ms=300000, cancellation_grace_ms=1000,
                      max_prompt_bytes=1024, max_output_bytes=1048576, max_turns=1,
                      toolsets={role: [] for role in ("read_only", "verify", "execute_with_approval", "human_approval", "compensate_with_approval")})
        generator.adapter_config(legacy)
        for key in ("max_timeout_ms", "max_output_bytes"):
            invalid = dict(legacy)
            invalid[key] = int(str(legacy[key])) + 1
            with self.assertRaises(ValueError):
                generator.adapter_config(invalid)


if __name__ == "__main__":
    unittest.main()
