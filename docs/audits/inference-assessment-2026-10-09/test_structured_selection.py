"""Offline regression tests; synthetic mutations are not live observations."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('selection', HERE / 'structured-selection-trial.py')
assert spec is not None and spec.loader is not None
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class SelectionTests(unittest.TestCase):
    def test_duplicate_members(self):
        with self.assertRaises(ValueError):
            module.validate_selection('{"evidence_ids":["U0"],"actions":["restart"],"actions":[]}', {'U0'})

    def test_retained_and_rejected_mutations(self):
        self.assertTrue(callable(getattr(module, 'qualify', None)), 'missing fail-closed qualification')
        saved = json.loads((HERE / 'structured-selection-explicit-chat.json').read_text())
        for original in saved['runs']:
            packet = json.loads(original['request']['messages'][1]['content'].split('\nChoose')[0])
            accepted = module.qualify(packet, original['response'], original['headers'], original['model'])
            self.assertTrue(accepted['accepted'])
            self.assertEqual(accepted['rendered_evidence'], original['rendered_evidence'])
            self.assertEqual(accepted['context']['scope'], packet['scope'])
            for kind in ['missing', 'model', 'provider', 'truncated', 'tools', 'choices', 'collision']:
                p, response, headers = copy.deepcopy(packet), copy.deepcopy(original['response']), dict(original['headers'])
                if kind == 'missing':
                    response['choices'][0]['message']['content'] = '{"evidence_ids":["M0"],"actions":[]}'
                elif kind in ['model', 'provider']:
                    headers['x-manwe-' + kind + '-id'] = 'wrong'
                elif kind == 'truncated':
                    response['choices'][0]['finish_reason'] = 'length'
                elif kind == 'tools':
                    response['choices'][0]['message']['tool_calls'] = [{'id':'unexpected'}]
                elif kind == 'choices':
                    response['choices'].append(copy.deepcopy(response['choices'][0]))
                else:
                    p['units'].append(copy.deepcopy(p['units'][0]))
                rejected = module.qualify(p, response, headers, original['model'])
                self.assertFalse(rejected['accepted'], kind)
                self.assertEqual(rejected['rendered_evidence'], [], kind)


if __name__ == '__main__':
    unittest.main()
