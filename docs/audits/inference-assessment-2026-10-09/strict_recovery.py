"""Bounded recovery evidence validation; not a general factuality judge."""
import json
from pathlib import Path


def validate(captures, case_root=None):
    primary = [c for c in captures if len((c.get('request') or {}).get('tools', [])) > 1]
    assert primary, 'No primary case requests'
    emitted = []
    final_seen = False
    root = Path(case_root) if case_root is not None else None
    for cap in primary:
        assert cap.get('status') == 200 and not cap.get('error'), 'Primary delivery incomplete'
        # Hermes normalizes upstream call IDs. Bind IDs within each actual request
        # history; never equate upstream IDs with normalized downstream IDs.
        links = {}
        observed = []
        for msg in (cap.get('request') or {}).get('messages', []):
            if msg.get('role') == 'assistant':
                for call in msg.get('tool_calls', []):
                    if call['function']['name'] == 'read_file':
                        p = Path(json.loads(call['function']['arguments'])['path'])
                        if not p.is_absolute():
                            assert root is not None, 'Relative path without fixture root'
                            p = root / p
                        links[call['id']] = p
            elif msg.get('role') == 'tool':
                p = links.get(msg.get('tool_call_id'))
                if p is not None:
                    value = json.loads(msg.get('content', '{}'))
                    observed.append((p, value))
        def prior(name, missing=False):
            return any(p == root / name and (v.get('not_found') is True if missing else not v.get('error') and not v.get('not_found') and bool(v.get('content'))) for p, v in observed)
        for choice in (cap.get('response') or {}).get('choices', []):
            message = choice.get('message', {})
            for call in message.get('tool_calls', []):
                assert call['function']['name'] == 'read_file', 'Unexpected recovery mutation/tool'
                p = Path(json.loads(call['function']['arguments'])['path'])
                if root is None:
                    assert p.is_absolute() and p.name == 'missing.txt', 'Cannot establish fixture root'
                    root = p.parent
                if not p.is_absolute():
                    p = root / p
                assert p in [root / n for n in ('missing.txt', 'index.txt', 'records.txt')], 'Wrong exact fixture path'
                if p.name == 'index.txt':
                    assert prior('missing.txt', missing=True), 'Index requested before observing missing-file failure'
                if p.name == 'records.txt':
                    assert prior('index.txt'), 'Replacement requested before successful index result'
                    assert any(p2 == root / 'index.txt' and 'records.txt' in v.get('content', '') for p2, v in observed), 'Index does not name replacement'
                emitted.append(p.name)
            if choice.get('finish_reason') == 'stop' and (message.get('content') or message.get('reasoning_content')) and not message.get('tool_calls'):
                assert prior('missing.txt', missing=True) and prior('index.txt') and prior('records.txt'), 'Final answer precedes successful complete recovery'
                assert all(n in emitted for n in ('missing.txt', 'index.txt', 'records.txt')), 'No observed native chain'
                final_seen = True
    assert final_seen, 'No final response consuming completed recovery'


if __name__ == '__main__':
    import sys
    for filename in sys.argv[1:]:
        d = json.loads(Path(filename).read_text())
        c = d['cases'][0]
        validate(d['upstream_captures'], Path(d['state_root']) / f"{c['kind']}-{c['repetition']}")
        print(filename, 'strict procedural recovery PASS; factual grading separate')
