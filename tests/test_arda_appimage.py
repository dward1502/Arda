import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).resolve().parents[1] / "scripts" / "arda_appimage.py"
ROOT = MODULE_PATH.parents[1]
SPEC = importlib.util.spec_from_file_location("arda_appimage", MODULE_PATH)
assert SPEC and SPEC.loader
appimage = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(appimage)


class AppImagePackagingTests(unittest.TestCase):
    def test_tauri_build_disables_incompatible_linuxdeploy_strip(self) -> None:
        package = json.loads((ROOT / "apps/arda-launcher/package.json").read_text(encoding="utf-8"))
        self.assertEqual(package["scripts"]["tauri"], "NO_STRIP=true cargo-tauri")

    def test_fetch_verified_reuses_matching_cached_input(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            cached = Path(temp) / "tool"
            cached.write_bytes(b"pinned")
            expected = appimage.sha256_file(cached)
            self.assertEqual(
                appimage.fetch_verified("https://example.invalid/unreachable", cached, expected),
                cached,
            )

    def test_package_requires_exact_tool_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            appdir = root / "Arda.AppDir"
            appdir.mkdir()
            (appdir / "AppRun").write_text("#!/bin/sh\n", encoding="utf-8")
            (appdir / "arda.desktop").write_text("[Desktop Entry]\n", encoding="utf-8")
            tool = root / "appimagetool"
            tool.write_bytes(b"wrong")
            runtime = root / "runtime"
            runtime.write_bytes(b"runtime")
            with self.assertRaisesRegex(ValueError, "appimagetool SHA-256 mismatch"):
                appimage.package_appimage(
                    appdir,
                    root / "arda.AppImage",
                    tool,
                    runtime,
                    1,
                    tool_sha256="0" * 64,
                    runtime_sha256=appimage.sha256_file(runtime),
                )

    def test_package_passes_pinned_runtime_and_fixed_epoch(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            appdir = root / "Arda.AppDir"
            appdir.mkdir()
            (appdir / "AppRun").write_text("#!/bin/sh\n", encoding="utf-8")
            (appdir / "arda.desktop").write_text("[Desktop Entry]\n", encoding="utf-8")
            runtime = root / "runtime"
            runtime.write_bytes(b"runtime-bytes")
            tool = root / "appimagetool"
            tool.write_text(
                "#!/usr/bin/env python3\n"
                "import os, pathlib, sys\n"
                "assert os.environ['SOURCE_DATE_EPOCH'] == '1234567890'\n"
                "assert os.environ['ARCH'] == 'x86_64'\n"
                "assert sys.argv[1:4] == ['--no-appstream', '--runtime-file', sys.argv[3]]\n"
                "out = pathlib.Path(sys.argv[-1])\n"
                "out.write_bytes(b'\\x7fELF0000AI\\x02' + b'x' * 64)\n",
                encoding="utf-8",
            )
            os.chmod(tool, 0o755)
            output = root / "arda.AppImage"
            result = appimage.package_appimage(
                appdir,
                output,
                tool,
                runtime,
                1234567890,
                tool_sha256=appimage.sha256_file(tool),
                runtime_sha256=appimage.sha256_file(runtime),
            )
            self.assertEqual(result, output)
            self.assertTrue(os.access(output, os.X_OK))


if __name__ == "__main__":
    unittest.main()
