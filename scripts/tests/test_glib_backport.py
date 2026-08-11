#!/usr/bin/env python3
"""Deterministic contract checks for the Stage 5 glib 0.18.5 backport."""

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
VENDOR = ROOT / "vendor" / "glib-0.18.5"
ROOT_MANIFEST = ROOT / "Cargo.toml"
HUD_MANIFEST = ROOT / "apps" / "arda-hud" / "src-tauri" / "Cargo.toml"


class GlibBackportContractTests(unittest.TestCase):
    def test_variant_str_iter_uses_mutable_out_argument(self) -> None:
        source = (VENDOR / "src" / "variant_iter.rs").read_text()
        self.assertIn(
            "let mut p: *mut libc::c_char = std::ptr::null_mut();",
            source,
        )
        self.assertIn("                &mut p,", source)
        self.assertNotIn(
            "let p: *mut libc::c_char = std::ptr::null_mut();",
            source,
        )
        self.assertNotIn("                &p,", source)

    def test_both_tauri_graphs_patch_the_same_vendor(self) -> None:
        root_manifest = ROOT_MANIFEST.read_text()
        hud_manifest = HUD_MANIFEST.read_text()
        self.assertIn('glib = { path = "vendor/glib-0.18.5" }', root_manifest)
        self.assertIn(
            'glib = { path = "../../../vendor/glib-0.18.5" }',
            hud_manifest,
        )

    def test_patch_provenance_is_present(self) -> None:
        note = (VENDOR / "ARDA_PATCH.md").read_text()
        self.assertIn("RUSTSEC-2024-0429", note)
        self.assertIn("233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5", note)
        self.assertIn("e2f5aefcc60492b7f51a2ddcf1b649ef73f54bf4", note)


if __name__ == "__main__":
    unittest.main()
