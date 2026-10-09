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


import json
from types import SimpleNamespace
from unittest.mock import patch

class EndpointProvisioningTests(unittest.TestCase):
    def test_release_endpoint_snapshot_and_conflict_refusal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            release = root / "release"
            (release / "config/retained").mkdir(parents=True)
            (release / "config/retained/hermes-profile.json").write_bytes(
                (generator.REPO / "config/retained/hermes-profile.json").read_bytes())
            manifest = b'[[service]]\nname="manwe"\nstart={command="manwe",args=["--bind=127.0.0.1","--port=17171"]}\nhealth={url="http://127.0.0.1:17171",path="/healthz"}\n'
            (release / "services.toml").write_bytes(manifest)
            for name in ("code", "site"):
                (root / name).mkdir()
            legacy = root / "adapter.toml"
            legacy.write_text('schema_version="arda.hermes-adapter.v1"\nadapter_version="test"\nmax_timeout_ms=1000\ncancellation_grace_ms=100\nmax_prompt_bytes=1024\nmax_output_bytes=1024\nmax_turns=1\n[toolsets]\nread_only=[]\nverify=[]\nexecute_with_approval=[]\nhuman_approval=[]\ncompensate_with_approval=[]\n')
            args = SimpleNamespace(output_root=str(root / "output"), session_base=str(root / "sessions"),
                hermes_source=str(root / "code"), user_site=str(root / "site"),
                python=str(Path("/usr/bin/python3").resolve()), legacy_adapter=str(legacy), apply=True)
            with patch.object(generator, "REPO", release), patch.dict(os.environ, {"ARDA_ROOT": str(root / "decoy"), "ARDA_MANWE_URL": "http://127.0.0.1:7171"}):
                args.apply = False
                generator.generate(args)
                self.assertFalse((root / "output").exists())
                self.assertFalse((root / "sessions").exists())
                args.apply = True
                (release / "services.toml").write_text("invalid manifest")
                with self.assertRaises(ValueError):
                    generator.generate(args)
                self.assertFalse((root / "output").exists())
                (release / "services.toml").write_bytes(manifest)
                generator.generate(args)
                output = root / "output"
                profile = json.loads((output / "config.yaml").read_text())
                self.assertEqual(profile["custom_providers"][0]["base_url"], "http://127.0.0.1:17171/v1")
                self.assertEqual((output / "services.toml").read_bytes(), manifest)
                policy = json.loads((output / "runtime-policy.json").read_text())
                grant = next(g for g in policy["grants"] if g["destination"] == "/run/arda-runtime/services.toml")
                self.assertEqual((grant["source"], grant["role"], grant["kind"], grant["access"]),
                    (str(output / "services.toml"), "network_input", "file", "read_only"))
                generator.generate(args)
                saved = {p.name: p.read_bytes() for p in output.iterdir()}
                (release / "services.toml").write_bytes(manifest.replace(b"17171", b"27171"))
                with self.assertRaises(ValueError):
                    generator.generate(args)
                self.assertEqual({p.name: p.read_bytes() for p in output.iterdir()}, saved)

if __name__ == "__main__":
    unittest.main()
