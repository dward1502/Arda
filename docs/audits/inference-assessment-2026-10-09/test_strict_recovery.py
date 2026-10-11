import copy
import json
from pathlib import Path
import unittest
from strict_recovery import validate

BASE = Path(__file__).parent


def evidence():
    return json.loads((BASE / 'p4-b11430-serial-t0-recovery-1.json').read_text())


def replacement_capture(captures):
    return next(i for i, cap in enumerate(captures) if any('records.txt' in c['function']['arguments'] for choice in (cap.get('response') or {}).get('choices', []) for c in choice.get('message', {}).get('tool_calls', [])))


class RecoveryValidation(unittest.TestCase):
    def test_actual_complete_chain(self):
        validate(evidence()['upstream_captures'])

    def test_rejects_truncated_before_replacement_result(self):
        caps = evidence()['upstream_captures']
        with self.assertRaises(AssertionError):
            validate(caps[:replacement_capture(caps) + 1])

    def test_rejects_unrelated_same_basename(self):
        caps = evidence()['upstream_captures']
        cap = caps[replacement_capture(caps)]
        cap['response']['choices'][0]['message']['tool_calls'][0]['function']['arguments'] = json.dumps({'path': '/unrelated/records.txt'})
        with self.assertRaises(AssertionError):
            validate(caps)

    def test_rejects_failed_replacement_result(self):
        caps = evidence()['upstream_captures']
        for cap in caps[replacement_capture(caps) + 1:]:
            msgs = (cap.get('request') or {}).get('messages', [])
            ids = {c['id'] for m in msgs for c in m.get('tool_calls', []) if 'records.txt' in c['function']['arguments']}
            for msg in msgs:
                if msg.get('role') == 'tool' and msg.get('tool_call_id') in ids:
                    msg['content'] = json.dumps({'error': 'read failed', 'content': ''})
        with self.assertRaises(AssertionError):
            validate(caps)

    def test_original_parallel_chain_rejected(self):
        d = json.loads((BASE / 'p4-b11430-fullctx-recovery-1.json').read_text())
        with self.assertRaises(AssertionError):
            validate(d['upstream_captures'])


if __name__ == '__main__':
    unittest.main()
