import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "hades_markdown_link_check.py"


class CompletionLanguageCheckTests(unittest.TestCase):
    def run_check(self, markdown: str) -> tuple[subprocess.CompletedProcess[str], str]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            plans = root / "docs" / "plans"
            plans.mkdir(parents=True)
            (plans / "active-plan.md").write_text(markdown, encoding="utf-8")
            (plans / "evidence.md").write_text("# Evidence\n", encoding="utf-8")
            report = root / "report.md"
            result = subprocess.run(
                [
                    "python3",
                    str(SCRIPT),
                    "--root",
                    str(root),
                    "--out",
                    str(report),
                    "--check-completion-language",
                ],
                text=True,
                capture_output=True,
                check=False,
            )
            report_text = report.read_text(encoding="utf-8") if report.exists() else result.stderr
            return result, report_text

    def test_rejects_unqualified_completion_and_operational_claims(self) -> None:
        result, report = self.run_check(
            "# Active Plan\n\nThe capability is complete.\nThe bridge is operational.\n"
        )

        self.assertEqual(result.returncode, 1)
        self.assertIn("unqualified_completion_claim", report)
        self.assertIn("active-plan.md:3", report)
        self.assertIn("active-plan.md:4", report)

    def test_permits_historical_and_explicitly_qualified_claims(self) -> None:
        result, report = self.run_check(
            "# Active Plan\n\n"
            "> Historical quotation: the prototype was complete.\n"
            "Maturity: `implemented` — the contract is complete.\n"
        )

        self.assertEqual(result.returncode, 0, report)
        self.assertIn("Completion-language issues: 0", report)

    def test_requires_links_for_high_evidence_maturity_tags(self) -> None:
        result, report = self.run_check(
            "# Active Plan\n\nMaturity: `operator_accepted`.\n"
        )

        self.assertEqual(result.returncode, 1)
        self.assertIn("missing_evidence_link", report)
        self.assertIn("operator_accepted", report)

    def test_accepts_high_evidence_tags_with_local_evidence_links(self) -> None:
        result, report = self.run_check(
            "# Active Plan\n\n"
            "Maturity: `native_hud_verified` — [native receipt](evidence.md).\n"
            "Maturity: `phone_access_verified` — [live receipt](evidence.md).\n"
            "Maturity: `operator_accepted` — [operator record](evidence.md).\n"
        )

        self.assertEqual(result.returncode, 0, report)
        self.assertIn("Completion-language issues: 0", report)

    def test_accepts_rustdoc_intra_doc_links_as_non_file_targets(self) -> None:
        result, report = self.run_check(
            "# Active Plan\n\n"
            "[`Variant`](struct@Variant) and [error domains](error::ErrorDomain).\n"
        )

        self.assertEqual(result.returncode, 0, report)
        self.assertIn("Broken local links: 0", report)


if __name__ == "__main__":
    unittest.main()
