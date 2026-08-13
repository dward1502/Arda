import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).resolve().parents[1] / "scripts" / "arda_release_ops.py"
SPEC = importlib.util.spec_from_file_location("arda_release_ops", MODULE_PATH)
assert SPEC and SPEC.loader
release_ops = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_ops)


class ReleaseOpsTests(unittest.TestCase):
    def test_release_source_identity_covers_every_production_surface(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            covered = (
                "apps/arda-launcher/main.ts",
                "apps/arda-hud/main.tsx",
                "crates/engine/src/lib.rs",
                "sdk/javascript/src/index.js",
                "spec/project-contract/v1/schema.json",
                "scripts/arda_beta_ops.py",
                "src/main.rs",
                "config/providers.toml",
                "vendor/glib/src/lib.rs",
                ".github/workflows/release-sign.yml",
                "Cargo.toml",
                "services.toml",
            )
            for relative in covered:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(f"{relative}: first\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=root, check=True)

            baseline = release_ops.source_tree_sha256(root)
            for relative in covered:
                path = root / relative
                original = path.read_text(encoding="utf-8")
                path.write_text(f"{relative}: second\n", encoding="utf-8")
                self.assertNotEqual(
                    baseline,
                    release_ops.source_tree_sha256(root),
                    f"release identity omitted {relative}",
                )
                path.write_text(original, encoding="utf-8")

    def test_source_tree_identity_tracks_source_content_but_ignores_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            source = root / "apps" / "arda-launcher" / "main.ts"
            source.parent.mkdir(parents=True)
            source.write_text("first\n", encoding="utf-8")
            subprocess.run(["git", "add", "apps/arda-launcher/main.ts"], cwd=root, check=True)
            first = release_ops.source_tree_sha256(root)

            evidence = root / "docs" / "evidence" / "result.json"
            evidence.parent.mkdir(parents=True)
            evidence.write_text("{}\n", encoding="utf-8")
            self.assertEqual(first, release_ops.source_tree_sha256(root))

            source.write_text("second\n", encoding="utf-8")
            self.assertNotEqual(first, release_ops.source_tree_sha256(root))

    def test_bundle_manifest_and_checksums_are_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            deb = root / "arda-launcher.deb"
            appimage = root / "arda-launcher.AppImage"
            deb.write_bytes(b"deb-artifact")
            appimage.write_bytes(b"appimage-artifact")
            first_dir = root / "first"
            second_dir = root / "second"
            inputs = {
                "source_commit": "abc123",
                "source_date_epoch": 123,
                "toolchain": {"cargo": "cargo fixture"},
            }

            first = release_ops.generate_bundle_manifest(
                [appimage, deb],
                first_dir / "release-bundle-manifest.json",
                first_dir / "SHA256SUMS",
                "0.3.0-rc.0",
                inputs,
            )
            second = release_ops.generate_bundle_manifest(
                [deb, appimage],
                second_dir / "release-bundle-manifest.json",
                second_dir / "SHA256SUMS",
                "0.3.0-rc.0",
                inputs,
            )

            self.assertEqual(
                (first_dir / "release-bundle-manifest.json").read_bytes(),
                (second_dir / "release-bundle-manifest.json").read_bytes(),
            )
            self.assertEqual(
                (first_dir / "SHA256SUMS").read_bytes(),
                (second_dir / "SHA256SUMS").read_bytes(),
            )
            self.assertEqual(first["release_id"], second["release_id"])
            self.assertEqual([item["name"] for item in first["artifacts"]], sorted([deb.name, appimage.name]))
            for artifact in (deb, appimage):
                (first_dir / artifact.name).write_bytes(artifact.read_bytes())
            self.assertTrue(release_ops.verify_checksums(first_dir / "SHA256SUMS", first_dir))

    def test_bundle_manifest_rejects_dirty_release_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            artifact = root / "arda-launcher.AppImage"
            artifact.write_bytes(b"artifact")
            with self.assertRaisesRegex(ValueError, "clean tracked worktree"):
                release_ops.generate_bundle_manifest(
                    [artifact],
                    root / "release-bundle-manifest.json",
                    root / "SHA256SUMS",
                    "1.0.0",
                    {"source_commit": "abc123", "tracked_worktree_clean": False},
                )

    def test_sbom_traverses_only_launcher_reachable_cargo_nodes_and_reports_missing_licenses(self) -> None:
        cargo = {
            "packages": [
                {"id": "launcher", "name": "arda-launcher", "version": "0.3.0-rc.0", "license": None, "source": None},
                {"id": "serde", "name": "serde", "version": "1.0.0", "license": "MIT OR Apache-2.0", "source": "registry+https://example.invalid"},
                {"id": "unused", "name": "unused", "version": "9.9.9", "license": "MIT", "source": "registry+https://example.invalid"},
            ],
            "resolve": {
                "root": "launcher",
                "nodes": [
                    {"id": "launcher", "deps": [{"pkg": "serde"}]},
                    {"id": "serde", "deps": []},
                    {"id": "unused", "deps": []},
                ],
            },
        }
        pnpm = {
            "MIT": [
                {"name": "react", "versions": ["19.0.0"], "license": "MIT", "paths": ["/secret/path"]}
            ]
        }

        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "sbom.json"
            sbom = release_ops.generate_sbom(cargo, pnpm, output, root_package="arda-launcher")

            names = {(item["ecosystem"], item["name"]) for item in sbom["components"]}
            self.assertIn(("cargo", "serde"), names)
            self.assertIn(("pnpm", "react"), names)
            self.assertNotIn(("cargo", "unused"), names)
            self.assertNotIn("paths", json.dumps(sbom))
            self.assertEqual(sbom["status"], "blocked")
            self.assertEqual(sbom["missing_license"], ["cargo:arda-launcher@0.3.0-rc.0"])


if __name__ == "__main__":
    unittest.main()
