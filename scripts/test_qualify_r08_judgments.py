#!/usr/bin/env python3
"""Synthetic metric fixtures; these are NOT human acceptance labels."""
import hashlib
import json
import unittest
from qualify_r08_judgments import assess

class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.corpus = json.dumps([{"id": str(i), "task": "fixture", "context": []} for i in range(4)]).encode()
        self.digest = hashlib.sha256(self.corpus).hexdigest()
        self.pred = {"schema":"r08.judgments.v1", "corpus_sha256":self.digest, "execution_authority":False,
                     "verdicts":[{"query_id":str(i), "policy_id":"fixture", "policy_version":"1", "outcome":outcome}
                                 for i,outcome in enumerate(["pass", "fail", "conditional", "escalate"])]}
        self.labels = {"corpus_sha256":self.digest, "reviewer":"synthetic unit-test reviewer", "provenance":"human",
                       "labels":[{"id":str(i),"judgment":j,"reason":"synthetic metric fixture"} for i,j in enumerate(["reject","accept","reject","accept"])]}
    def test_unlabeled_does_not_invent_quality(self):
        out = assess(self.corpus, self.pred, None)
        self.assertEqual(out["status"], "needs_human_labels")
        self.assertIsNone(out["false_approval_rate"])
        self.assertIsNone(out["false_rejection_rate"])
    def test_rates_have_explicit_denominators_and_deferred_separate(self):
        out = assess(self.corpus, self.pred, self.labels)
        self.assertEqual(out["false_approvals"], 1)
        self.assertEqual(out["false_rejections"], 1)
        self.assertEqual(out["human_reject_count"], 2)
        self.assertEqual(out["human_accept_count"], 2)
        self.assertEqual(out["false_approval_rate"], .5)
        self.assertEqual(out["false_rejection_rate"], .5)
        self.assertEqual(out["deferred_count"], 2)
        self.assertEqual(out["decision_coverage"], .5)
    def test_all_deferred_is_not_good_quality(self):
        for verdict in self.pred["verdicts"]: verdict["outcome"] = "conditional"
        out = assess(self.corpus, self.pred, self.labels)
        self.assertEqual(out["decision_coverage"], 0)
        self.assertEqual(out["deferred_count"], 4)
        self.assertEqual(out["status"], "measured_not_accepted")
    def test_rejects_agent_labels(self):
        self.labels["provenance"] = "agent"
        with self.assertRaises(ValueError): assess(self.corpus, self.pred, self.labels)
    def test_rejects_changed_corpus(self):
        with self.assertRaises(ValueError): assess(self.corpus+b" ", self.pred, self.labels)
    def test_rejects_duplicate_or_missing_verdict(self):
        self.pred["verdicts"][1] = self.pred["verdicts"][0]
        with self.assertRaises(ValueError): assess(self.corpus, self.pred, self.labels)
    def test_rejects_duplicate_or_missing_labels(self):
        self.labels["labels"].pop()
        with self.assertRaises(ValueError): assess(self.corpus, self.pred, self.labels)
    def test_zero_denominator_is_not_zero_error(self):
        for label in self.labels["labels"]: label["judgment"] = "review"
        out = assess(self.corpus, self.pred, self.labels)
        self.assertIsNone(out["false_approval_rate"])
        self.assertIsNone(out["false_rejection_rate"])

if __name__ == "__main__": unittest.main()
