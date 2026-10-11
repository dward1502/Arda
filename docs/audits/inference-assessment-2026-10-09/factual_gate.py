"""Offline review gate. Review labels are trusted input, not automated truth detection.

Run: python3 factual_gate.py ARTIFACT_PREFIX REVIEWS_JSON
Prints a qualification report; exits 1 for incomplete/failed qualification.
Original runner `passed` remains a mechanical-only historical result.
"""
import hashlib
import json
import sys
from pathlib import Path

EXPECTED = {f'{kind}-{n}' for kind in ('research', 'recovery', 'coding') for n in (1, 2)}


def evaluate(artifacts, reviews):
    artifacts = artifacts if isinstance(artifacts, dict) else {}
    reviews = reviews if isinstance(reviews, dict) else {}
    rows = []
    for key in sorted(EXPECTED):
        raw = artifacts.get(key)
        review = reviews.get(key, {})
        review = review if isinstance(review, dict) else {}
        mechanical = False
        bound = False
        if isinstance(raw, bytes):
            try:
                data = json.loads(raw)
            except (ValueError, UnicodeError):
                data = None
            data = data if isinstance(data, dict) else {}
            cases = data.get('cases', [])
            kind, repetition = key.rsplit('-', 1)
            mechanical = (data.get('passed') is True and isinstance(cases, list)
                          and len(cases) == 1 and isinstance(cases[0], dict)
                          and cases[0].get('passed') is True
                          and cases[0].get('kind') == kind
                          and type(cases[0].get('repetition')) is int
                          and cases[0]['repetition'] == int(repetition))
            bound = (review.get('sha256') == hashlib.sha256(raw).hexdigest()
                     and all(isinstance(review.get(field), str) and review[field].strip()
                             for field in ('reviewer', 'reason')))
        factual = bool(bound and review.get('verdict') == 'pass')
        rows.append({'case': key, 'mechanical_pass': mechanical,
                     'review_bound': bool(bound), 'factual_pass': factual,
                     'reason': review.get('reason', 'Missing factual review')})
    return {'scope': 'bounded fixture qualification, not deployment acceptance',
            'qualified': (set(artifacts) == EXPECTED and set(reviews) == EXPECTED
                          and all(r['mechanical_pass'] and r['factual_pass'] for r in rows)),
            'mechanical_passes': sum(r['mechanical_pass'] for r in rows),
            'factual_passes': sum(r['factual_pass'] for r in rows),
            'expected_cases': len(EXPECTED), 'rows': rows}


if __name__ == '__main__':
    prefix, review_path = sys.argv[1:]
    artifacts = {key: Path(f'{prefix}{key}.json').read_bytes()
                 for key in EXPECTED if Path(f'{prefix}{key}.json').is_file()}
    result = evaluate(artifacts, json.loads(Path(review_path).read_text()))
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if result['qualified'] else 1)
