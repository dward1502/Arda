import copy
import hashlib
import json
import unittest
from pathlib import Path
import factual_gate as gate

BASE = Path(__file__).parent

class GateTests(unittest.TestCase):
    def setUp(self):
        self.artifacts = {}
        self.reviews = {}
        for kind in ('research', 'recovery', 'coding'):
            for n in (1, 2):
                key = f'{kind}-{n}'
                raw = (BASE / f'p4-saluki-dualgpu-{key}.json').read_bytes()
                self.artifacts[key] = raw
                self.reviews[key] = {'sha256': hashlib.sha256(raw).hexdigest(), 'verdict': 'pass', 'reviewer': 'synthetic-positive-control', 'reason': 'Test label only'}

    def test_invalid_metadata_rejects(self):
        for reviewer, reason in [('  ', '\t'), (True, ['x'])]:
            self.reviews['coding-1'].update(reviewer=reviewer, reason=reason)
            self.assertFalse(gate.evaluate(self.artifacts, self.reviews)['qualified'])

    def test_malformed_shapes_reject(self):
        for bad in (None, [], 'bad'):
            reviews = dict(self.reviews); reviews['coding-1'] = bad
            self.assertFalse(gate.evaluate(self.artifacts, reviews)['qualified'])
            artifacts = dict(self.artifacts); artifacts['coding-1'] = json.dumps(bad).encode()
            self.assertFalse(gate.evaluate(artifacts, self.reviews)['qualified'])
        self.assertFalse(gate.evaluate(self.artifacts, None)['qualified'])
        artifacts = dict(self.artifacts); artifacts['coding-1'] = b'not json'
        self.assertFalse(gate.evaluate(artifacts, self.reviews)['qualified'])

    def test_boolean_repetition_rejects(self):
        d = json.loads(self.artifacts['coding-1']); d['cases'][0]['repetition'] = True
        raw = json.dumps(d).encode(); self.artifacts['coding-1'] = raw
        self.reviews['coding-1']['sha256'] = hashlib.sha256(raw).hexdigest()
        self.assertFalse(gate.evaluate(self.artifacts, self.reviews)['qualified'])

    def test_positive_control(self):
        self.assertTrue(gate.evaluate(self.artifacts, self.reviews)['qualified'])

    def test_actual_coding_failures_reject(self):
        for key in ('coding-1', 'coding-2'):
            self.reviews[key]['verdict'] = 'fail'
        result = gate.evaluate(self.artifacts, self.reviews)
        self.assertFalse(result['qualified'])
        self.assertEqual(result['mechanical_passes'], 6)
        self.assertEqual(result['factual_passes'], 4)

    def test_missing_review_rejects(self):
        del self.reviews['coding-1']
        self.assertFalse(gate.evaluate(self.artifacts, self.reviews)['qualified'])

    def test_stale_review_rejects(self):
        self.artifacts['coding-1'] += b'\n'
        self.assertFalse(gate.evaluate(self.artifacts, self.reviews)['qualified'])

    def test_missing_case_rejects(self):
        del self.artifacts['research-2']
        self.assertFalse(gate.evaluate(self.artifacts, self.reviews)['qualified'])

    def test_mechanical_failure_rejects(self):
        d = json.loads(self.artifacts['coding-1']); d['passed'] = False
        raw = json.dumps(d).encode(); self.artifacts['coding-1'] = raw
        self.reviews['coding-1']['sha256'] = hashlib.sha256(raw).hexdigest()
        self.assertFalse(gate.evaluate(self.artifacts, self.reviews)['qualified'])

    def test_missing_reviewer_rejects(self):
        self.reviews['coding-1']['reviewer'] = ''
        self.assertFalse(gate.evaluate(self.artifacts, self.reviews)['qualified'])

if __name__ == '__main__':
    unittest.main()
