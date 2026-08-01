import importlib.util
import io
import json
import os
import stat
import sys
import tarfile
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "arda_beta_ops.py"
SPEC = importlib.util.spec_from_file_location("arda_beta_ops", SCRIPT)
assert SPEC and SPEC.loader
beta_ops = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = beta_ops
SPEC.loader.exec_module(beta_ops)


class PrivateBetaOpsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)
        self.home = self.base / "home"
        self.root = self.base / "source" / "Arda"
        self.home.mkdir(parents=True)
        self.root.mkdir(parents=True)
        (self.root / "AGENTS.md").write_text("instructions\n", encoding="utf-8")
        (self.root / "core/state").mkdir(parents=True)
        (self.root / "core/state/contract_registry.json").write_text("{}\n", encoding="utf-8")
        (self.root / "scripts").mkdir()
        (self.root / "scripts/arda_beta_ops.py").write_text("# fixture\n", encoding="utf-8")
        (self.root / "docs/operator").mkdir(parents=True)
        (self.root / "docs/operator/private-beta-install-recovery.md").write_text("fixture\n", encoding="utf-8")
        self.layout = beta_ops.Layout(
            home=self.home,
            root=self.root,
            config=self.home / ".config/arda",
            data=self.home / ".local/share/arda",
            launcher_data=self.home / ".local/share/arda.launcher",
            cache=self.home / ".cache/arda",
            runtime=self.home / ".local/run/arda",
            install_lib=self.home / ".local/lib/arda",
            install_bin=self.home / ".local/bin/arda-launcher",
            desktop_file=self.home / ".local/share/applications/arda-launcher.desktop",
            operation_state=self.home / ".local/state/arda",
        )

    def tearDown(self):
        self.temp.cleanup()

    def seed_state(self):
        self.layout.config.mkdir(parents=True, exist_ok=True)
        self.layout.data.mkdir(parents=True, exist_ok=True)
        (self.layout.config / "settings.toml").write_text("mode = 'local'\n", encoding="utf-8")
        (self.layout.config / "arda.env").write_text("OPENAI_API_KEY=super-secret\n", encoding="utf-8")
        (self.layout.data / "runs.jsonl").write_text('{"run":"one"}\n', encoding="utf-8")

    def test_backup_restore_round_trip_excludes_secret_named_files(self):
        self.seed_state()
        archive = self.base / "backup.tar.gz"

        receipt = beta_ops.backup(self.layout, archive)
        self.assertEqual(receipt["file_count"], 2)
        self.assertEqual(receipt["excluded_secret_count"], 1)
        self.assertEqual(stat.S_IMODE(archive.stat().st_mode), 0o600)

        with tarfile.open(archive, "r:gz") as handle:
            names = handle.getnames()
            self.assertIn("config/settings.toml", names)
            self.assertIn("data/runs.jsonl", names)
            self.assertNotIn("config/arda.env", names)

        self.layout.config.rename(self.home / "config-before-restore")
        self.layout.data.rename(self.home / "data-before-restore")
        restored = beta_ops.restore(self.layout, archive)
        self.assertEqual(restored["file_count"], 2)
        self.assertEqual((self.layout.config / "settings.toml").read_text(), "mode = 'local'\n")
        self.assertEqual((self.layout.data / "runs.jsonl").read_text(), '{"run":"one"}\n')
        self.assertFalse((self.layout.config / "arda.env").exists())

    def test_restore_rejects_path_traversal(self):
        archive = self.base / "malicious.tar.gz"
        manifest = {
            "contract": beta_ops.CONTRACT,
            "operation": "backup",
            "files": [],
        }
        with tarfile.open(archive, "w:gz") as handle:
            manifest_bytes = json.dumps(manifest).encode()
            manifest_info = tarfile.TarInfo("manifest.json")
            manifest_info.size = len(manifest_bytes)
            handle.addfile(manifest_info, io.BytesIO(manifest_bytes))
            payload = b"owned"
            traversal = tarfile.TarInfo("config/../../owned")
            traversal.size = len(payload)
            handle.addfile(traversal, io.BytesIO(payload))

        with self.assertRaisesRegex(beta_ops.BetaOpsError, "unsafe archive member"):
            beta_ops.restore(self.layout, archive)
        self.assertFalse((self.base / "owned").exists())

    def test_restore_rejects_file_omitted_from_backup_manifest(self):
        archive = self.base / "unmanifested-file.tar.gz"
        manifest = {
            "contract": beta_ops.CONTRACT,
            "operation": "backup",
            "files": [],
        }
        with tarfile.open(archive, "w:gz") as handle:
            manifest_bytes = json.dumps(manifest).encode()
            manifest_info = tarfile.TarInfo(beta_ops.ARCHIVE_MANIFEST)
            manifest_info.size = len(manifest_bytes)
            handle.addfile(manifest_info, io.BytesIO(manifest_bytes))
            payload = b"unverified data"
            extra = tarfile.TarInfo("config/injected.toml")
            extra.size = len(payload)
            handle.addfile(extra, io.BytesIO(payload))

        with self.assertRaisesRegex(beta_ops.BetaOpsError, "archive members do not match backup manifest"):
            beta_ops.restore(self.layout, archive)
        self.assertFalse((self.layout.config / "injected.toml").exists())

    def test_reset_backs_up_and_quarantines_only_state_roots(self):
        self.seed_state()
        self.layout.cache.mkdir(parents=True)
        self.layout.runtime.mkdir(parents=True)
        (self.layout.cache / "cache.bin").write_bytes(b"cache")
        source_marker = self.root / "KEEP_SOURCE"
        source_marker.write_text("untouched\n", encoding="utf-8")

        receipt = beta_ops.reset(self.layout, self.base / "reset-backup.tar.gz")

        self.assertFalse(self.layout.config.exists())
        self.assertFalse(self.layout.data.exists())
        self.assertFalse(self.layout.cache.exists())
        self.assertFalse(self.layout.runtime.exists())
        self.assertTrue(source_marker.is_file())
        self.assertFalse(receipt["source_repository_touched"])
        self.assertEqual(set(receipt["moved"]), {"config", "data", "cache", "runtime"})

    def test_diagnostics_redacts_values_and_operator_paths(self):
        self.layout.config.mkdir(parents=True)
        (self.layout.config / "settings.toml").write_text(
            f"API_TOKEN=visible-token\nendpoint=https://user:pass@example.test\nroot={self.root}\nhome={self.home}\n",
            encoding="utf-8",
        )
        (self.layout.config / "credentials.json").write_text('{"token":"must-not-ship"}\n', encoding="utf-8")
        output = self.base / "diagnostics.tar.gz"

        receipt = beta_ops.diagnostics(self.layout, output)
        self.assertEqual(receipt["status"], "ok")
        self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o600)
        with tarfile.open(output, "r:gz") as handle:
            names = handle.getnames()
            self.assertIn("diagnostics.json", names)
            self.assertIn("config-redacted/settings.toml", names)
            self.assertNotIn("config-redacted/credentials.json", names)
            payloads = []
            for name in names:
                if not handle.getmember(name).isfile():
                    continue
                extracted = handle.extractfile(name)
                self.assertIsNotNone(extracted)
                assert extracted is not None
                payloads.append(extracted.read())
            combined = b"\n".join(payloads).decode("utf-8")
        self.assertNotIn("visible-token", combined)
        self.assertNotIn("user:pass", combined)
        self.assertNotIn(str(self.root), combined)
        self.assertNotIn(str(self.home), combined)
        self.assertIn("<redacted>", combined)
        self.assertIn("$ARDA_ROOT", combined)

    def test_clean_profile_install_and_uninstall_preserve_state_and_source(self):
        artifact = self.base / "arda-launcher-fixture"
        artifact.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        artifact.chmod(0o755)
        self.layout.data.mkdir(parents=True)
        state_marker = self.layout.data / "state.json"
        state_marker.write_text("{}\n", encoding="utf-8")
        source_marker = self.root / "KEEP_SOURCE"
        source_marker.write_text("untouched\n", encoding="utf-8")

        installed = beta_ops.install_launcher(self.layout, artifact)
        self.assertTrue(Path(installed["launcher"]).is_file())
        self.assertTrue(self.layout.install_bin.is_symlink())
        self.assertTrue(self.layout.desktop_file.is_file())
        self.assertEqual(os.spawnv(os.P_WAIT, str(self.layout.install_bin), [str(self.layout.install_bin)]), 0)

        removed = beta_ops.uninstall_launcher(self.layout)
        self.assertFalse(self.layout.install_lib.exists())
        self.assertFalse(self.layout.install_bin.exists())
        self.assertFalse(self.layout.desktop_file.exists())
        self.assertTrue(state_marker.is_file())
        self.assertTrue(source_marker.is_file())
        self.assertFalse(removed["source_repository_touched"])

    def test_uninstall_from_fresh_layout_removes_existing_launcher_symlink(self):
        artifact = self.base / "arda-launcher-fixture"
        artifact.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        artifact.chmod(0o755)
        beta_ops.install_launcher(self.layout, artifact)

        fresh_layout = beta_ops.layout_from_args(Namespace(home=str(self.home), root=str(self.root)))
        self.assertEqual(fresh_layout.install_bin, self.home / ".local/bin/arda-launcher")
        self.assertTrue(fresh_layout.install_bin.is_symlink())

        receipt = beta_ops.uninstall_launcher(fresh_layout)

        self.assertFalse(fresh_layout.install_bin.exists())
        self.assertFalse(fresh_layout.install_bin.is_symlink())
        self.assertIn(str(fresh_layout.install_bin), receipt["removed"])

    def test_reset_quarantines_launcher_webkit_state(self):
        self.seed_state()
        self.layout.launcher_data.mkdir(parents=True)
        (self.layout.launcher_data / "hsts-storage.sqlite").write_bytes(b"webkit state")

        receipt = beta_ops.reset(self.layout, self.base / "reset-backup.tar.gz")

        self.assertFalse(self.layout.launcher_data.exists())
        self.assertIn("launcher_data", receipt["moved"])
        self.assertTrue(Path(receipt["moved"]["launcher_data"]).is_dir())

    def test_readiness_reports_missing_launcher_dynamic_libraries(self):
        launcher = self.layout.install_lib / "arda-launcher"
        launcher.parent.mkdir(parents=True)
        launcher.write_bytes(b"ELF fixture")
        launcher.chmod(0o700)

        completed = beta_ops.subprocess.CompletedProcess(
            ["ldd", str(launcher)],
            0,
            stdout="libgdk-3.so.0 => not found\nlibc.so.6 => /lib64/libc.so.6\n",
        )
        with mock.patch.object(beta_ops.shutil, "which", side_effect=lambda command: f"/usr/bin/{command}"), mock.patch.object(
            beta_ops.subprocess, "run", return_value=completed
        ):
            projection = beta_ops.readiness(self.layout)

        check = next(item for item in projection["checks"] if item["check_id"] == "launcher.dynamic_libraries")
        self.assertEqual(check["status"], "fail")
        self.assertIn("libgdk-3.so.0", check["evidence"])
        self.assertIn("GTK3", check["recovery"])

    def test_readiness_accepts_resolved_launcher_dynamic_libraries(self):
        launcher = self.layout.install_lib / "arda-launcher"
        launcher.parent.mkdir(parents=True)
        launcher.write_bytes(b"ELF fixture")
        launcher.chmod(0o700)

        completed = beta_ops.subprocess.CompletedProcess(
            ["ldd", str(launcher)],
            0,
            stdout="libc.so.6 => /lib64/libc.so.6\n",
        )
        with mock.patch.object(beta_ops.shutil, "which", side_effect=lambda command: f"/usr/bin/{command}"), mock.patch.object(
            beta_ops.subprocess, "run", return_value=completed
        ):
            projection = beta_ops.readiness(self.layout)

        check = next(item for item in projection["checks"] if item["check_id"] == "launcher.dynamic_libraries")
        self.assertEqual(check["status"], "pass")
        self.assertEqual(check["evidence"], "all native shared libraries resolved")

    def test_layout_rejects_state_inside_source_repository(self):
        unsafe = beta_ops.Layout(
            home=self.home,
            root=self.root,
            config=self.root / "config-state",
            data=self.layout.data,
            launcher_data=self.layout.launcher_data,
            cache=self.layout.cache,
            runtime=self.layout.runtime,
            install_lib=self.layout.install_lib,
            install_bin=self.layout.install_bin,
            desktop_file=self.layout.desktop_file,
            operation_state=self.layout.operation_state,
        )
        with self.assertRaisesRegex(beta_ops.BetaOpsError, "inside the source repository"):
            beta_ops.validate_layout(unsafe)

    def test_explicit_home_ignores_inherited_xdg_roots(self):
        previous = os.environ.get("XDG_RUNTIME_DIR")
        os.environ["XDG_RUNTIME_DIR"] = "/run/user/fixture"
        try:
            layout = beta_ops.layout_from_args(Namespace(home=str(self.home), root=str(self.root)))
        finally:
            if previous is None:
                os.environ.pop("XDG_RUNTIME_DIR", None)
            else:
                os.environ["XDG_RUNTIME_DIR"] = previous
        self.assertEqual(layout.runtime, self.home / ".local/run/arda")
        beta_ops.validate_layout(layout)

    def test_compatibility_accepts_only_declared_bluefin_lts_profile(self):
        supported = beta_ops.compatibility(
            os_release={
                "ID": "centos",
                "ID_LIKE": "rhel fedora",
                "VERSION_ID": "10",
                "PRETTY_NAME": "Bluefin LTS",
                "VARIANT_ID": "bluefin",
            },
            machine="x86_64",
        )
        unsupported = beta_ops.compatibility(
            os_release={"ID": "ubuntu", "VERSION_ID": "24.04", "PRETTY_NAME": "Ubuntu 24.04"},
            machine="x86_64",
        )

        self.assertEqual(supported["status"], "supported")
        self.assertEqual(supported["profile_id"], "bluefin-lts-10-x86_64")
        self.assertEqual(unsupported["status"], "unsupported")
        self.assertIn("before installation", unsupported["message"])

    def test_cli_install_rejects_unsupported_profile_before_partial_install(self):
        artifact = self.base / "candidate"
        artifact.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        artifact.chmod(0o755)
        unsupported = {
            "contract": beta_ops.COMPATIBILITY_CONTRACT,
            "status": "unsupported",
            "profile_id": None,
            "message": "unsupported profile; refusing before installation",
        }

        with mock.patch.object(beta_ops, "compatibility", return_value=unsupported):
            result = beta_ops.main(
                [
                    "install-launcher",
                    "--root",
                    str(self.root),
                    "--home",
                    str(self.home),
                    "--artifact",
                    str(artifact),
                ]
            )

        self.assertEqual(result, 2)
        self.assertFalse(self.layout.install_lib.exists())
        self.assertFalse(self.layout.install_bin.exists())
        self.assertFalse(self.layout.desktop_file.exists())

    def test_release_manifest_is_byte_identical_for_identical_declared_inputs(self):
        artifact = self.base / "arda-launcher-0.3.0-rc.0"
        artifact.write_bytes(b"deterministic candidate bytes\n")
        first = self.base / "release-manifest-first.json"
        second = self.base / "release-manifest-second.json"
        declared_inputs = {
            "source_commit": "0123456789abcdef",
            "source_date_epoch": 1785542400,
            "rustc": "rustc fixture",
            "cargo": "cargo fixture",
            "node": "v24.0.0",
            "pnpm": "10.0.0",
            "target": "x86_64-unknown-linux-gnu",
        }

        first_receipt = beta_ops.generate_release_manifest(
            self.layout,
            artifact,
            first,
            "0.3.0-rc.0",
            build_inputs=declared_inputs,
        )
        second_receipt = beta_ops.generate_release_manifest(
            self.layout,
            artifact,
            second,
            "0.3.0-rc.0",
            build_inputs=declared_inputs,
        )

        self.assertEqual(first.read_bytes(), second.read_bytes())
        self.assertEqual(first_receipt["manifest_sha256"], second_receipt["manifest_sha256"])
        manifest = json.loads(first.read_text(encoding="utf-8"))
        self.assertEqual(manifest["artifact"]["sha256"], beta_ops.hash_file(artifact))
        self.assertEqual(manifest["artifact"]["version"], "0.3.0-rc.0")
        self.assertEqual(manifest["schemas"]["project_contract"], "arda.project-contract.v1")
        self.assertEqual(manifest["schemas"]["run_event"], "arda.run-event.v1")
        self.assertEqual(manifest["schemas"]["execution_receipt"], "arda.execution-receipt.v1")
        self.assertTrue(manifest["rollback_compatibility"]["state_preserving"])

    def test_upgrade_and_rollback_restore_prior_binary_and_preserve_run_truth(self):
        beta = self.base / "arda-launcher-stage4"
        beta.write_bytes(b"#!/bin/sh\n# stage4 beta\nexit 0\n")
        beta.chmod(0o755)
        candidate = self.base / "arda-launcher-0.3.0-rc.0"
        candidate.write_bytes(b"#!/bin/sh\n# stage5 candidate\nexit 0\n")
        candidate.chmod(0o755)
        self.seed_state()
        beta_ops.install_launcher(self.layout, beta)
        beta_sha = beta_ops.hash_file(beta)
        run_truth_before = beta_ops.hash_file(self.layout.data / "runs.jsonl")
        backup_path = self.base / "pre-upgrade.tar.gz"

        upgraded = beta_ops.upgrade_launcher(self.layout, candidate, backup_path)
        self.assertEqual(upgraded["prior_artifact_sha256"], beta_sha)
        self.assertEqual(upgraded["candidate_artifact_sha256"], beta_ops.hash_file(candidate))
        self.assertTrue(backup_path.is_file())
        installed_manifest = json.loads(
            (self.layout.install_lib / "install-manifest.json").read_text(encoding="utf-8")
        )
        self.assertEqual(
            installed_manifest["upgrade"]["backup_sha256"],
            beta_ops.hash_file(backup_path),
        )
        (self.layout.data / "runs.jsonl").write_text(
            '{"run":"one"}\n{"run":"rc0-fixture","terminal":"succeeded","mutation_receipts":["only-once"]}\n',
            encoding="utf-8",
        )
        post_run_backup = self.base / "post-run.tar.gz"
        beta_ops.backup(self.layout, post_run_backup)

        rolled_back = beta_ops.rollback_launcher(
            self.layout,
            Path(upgraded["rollback_receipt"]),
            post_run_backup,
        )

        self.assertEqual(beta_ops.hash_file(self.layout.install_lib / "arda-launcher"), beta_sha)
        runs = (self.layout.data / "runs.jsonl").read_text(encoding="utf-8")
        self.assertEqual(runs.count('"run":"rc0-fixture"'), 1)
        self.assertEqual(runs.count('"only-once"'), 1)
        self.assertNotEqual(beta_ops.hash_file(self.layout.data / "runs.jsonl"), run_truth_before)
        self.assertEqual(
            (self.layout.config / "arda.env").read_text(encoding="utf-8"),
            "OPENAI_API_KEY=super-secret\n",
        )
        self.assertTrue(rolled_back["terminal_run_truth_preserved"])
        self.assertTrue(rolled_back["source_repository_touched"] is False)
        rolled_back_manifest = json.loads(
            (self.layout.install_lib / "install-manifest.json").read_text(encoding="utf-8")
        )
        self.assertEqual(
            rolled_back_manifest["rollback_result"]["state_archive_sha256"],
            beta_ops.hash_file(post_run_backup),
        )

    def test_diagnostics_identifies_installed_release_without_source_content(self):
        artifact = self.base / "candidate"
        artifact.write_bytes(b"#!/bin/sh\n# source-canary-must-not-ship\nexit 0\n")
        artifact.chmod(0o755)
        release_manifest = self.base / "release-manifest.json"
        beta_ops.generate_release_manifest(
            self.layout,
            artifact,
            release_manifest,
            "0.3.0-rc.0",
            build_inputs={
                "source_commit": "0123456789abcdef",
                "source_date_epoch": 1785542400,
                "rustc": "rustc fixture",
                "cargo": "cargo fixture",
                "node": "v24.0.0",
                "pnpm": "10.0.0",
                "target": "x86_64-unknown-linux-gnu",
            },
        )
        beta_ops.install_launcher(self.layout, artifact, release_manifest=release_manifest)
        output = self.base / "diagnostics-release.tar.gz"

        beta_ops.diagnostics(self.layout, output)

        with tarfile.open(output, "r:gz") as handle:
            diagnostics_handle = handle.extractfile("diagnostics.json")
            self.assertIsNotNone(diagnostics_handle)
            assert diagnostics_handle is not None
            diagnostics = json.load(diagnostics_handle)
            payloads = []
            for member in handle.getmembers():
                if not member.isfile():
                    continue
                extracted = handle.extractfile(member)
                self.assertIsNotNone(extracted)
                assert extracted is not None
                payloads.append(extracted.read())
            combined = b"".join(payloads)
        self.assertEqual(diagnostics["installed_release"]["version"], "0.3.0-rc.0")
        self.assertEqual(
            diagnostics["installed_release"]["artifact_sha256"], beta_ops.hash_file(artifact)
        )
        self.assertNotIn(b"source-canary-must-not-ship", combined)


if __name__ == "__main__":
    unittest.main()
