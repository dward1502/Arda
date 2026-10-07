"""Regression cases for selecting the accepted prompt from retained retries."""

import copy
import unittest

from audit_m5_continuity import prompt_matches, require


class PromptEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.memory = {"memory_id": "memory", "content": "complete relevant report"}
        self.assembly = {"capsule": {"memories": [self.memory]}}
        self.use = {"receipt_id": "accepted-use", "capsule_digest": "accepted-digest"}
        self.receipt = {"run_id": "run", "node_id": "execute", "context_capsule_digest": "accepted-digest"}
        self.prompt = {
            "run_id": "run", "node": {"id": "execute"},
            "context_use_receipt": self.use,
            "organism_context_projection": {
                "source_capsule_digest": "accepted-digest", "memories": [self.memory],
            },
        }

    def matches(self, prompt):
        return prompt_matches(prompt, self.receipt, self.assembly, self.use)

    def test_complete_projection_and_legacy_capsule(self):
        self.assertTrue(self.matches(self.prompt))
        legacy = copy.deepcopy(self.prompt)
        capsule = legacy.pop("organism_context_projection")
        capsule["capsule_digest"] = capsule.pop("source_capsule_digest")
        legacy["organism_context_capsule"] = capsule
        self.assertTrue(self.matches(legacy))

    def test_failed_attempt_in_same_transcript_is_not_accepted_evidence(self):
        stale = copy.deepcopy(self.prompt)
        stale["context_use_receipt"]["receipt_id"] = "discarded-use"
        self.assertFalse(self.matches(stale))

    def test_cross_run_or_stage_is_rejected(self):
        for key in ["run", "stage"]:
            with self.subTest(key=key):
                prompt = copy.deepcopy(self.prompt)
                if key == "run":
                    prompt["run_id"] = "other-run"
                else:
                    prompt["node"]["id"] = "verify"
                self.assertFalse(self.matches(prompt))

    def test_metadata_only_truncated_or_changed_memory_is_rejected(self):
        for memories in [[], [{"memory_id": "memory"}], [{"memory_id": "memory", "content": "complete"}]]:
            with self.subTest(memories=memories):
                prompt = copy.deepcopy(self.prompt)
                prompt["organism_context_projection"]["memories"] = memories
                self.assertFalse(self.matches(prompt))

    def test_wrong_capsule_commitment_is_rejected(self):
        prompt = copy.deepcopy(self.prompt)
        prompt["organism_context_projection"]["source_capsule_digest"] = "other-digest"
        self.assertFalse(self.matches(prompt))

    def test_failed_check_raises_even_under_python_optimization(self):
        with self.assertRaisesRegex(ValueError, "changed"):
            require(False, "changed")


if __name__ == "__main__":
    unittest.main()
