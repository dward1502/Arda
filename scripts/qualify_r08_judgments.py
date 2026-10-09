#!/usr/bin/env python3
"""Offline R08 metrics. Human provenance is an attestation, not authentication.

Pass = unconditional advisory approval; fail = rejection. Conditional/escalate
are reported separately, never silently converted into approval or rejection.
"""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path


def unique(rows, key):
    out = {}
    for row in rows:
        value = row[key]
        if not isinstance(value, str) or not value.strip() or value in out:
            raise ValueError(f"invalid or duplicate {key}")
        out[value] = row
    if not out:
        raise ValueError("empty corpus/labels/verdicts")
    return out


def assess(corpus, predictions, labels):
    digest = hashlib.sha256(corpus).hexdigest()
    cases = unique(json.loads(corpus), "id")
    if predictions.get("schema") != "r08.judgments.v1" or predictions.get("execution_authority") is not False:
        raise ValueError("expected offline advisory predictions")
    if predictions.get("corpus_sha256") != digest:
        raise ValueError("predictions do not match frozen corpus")
    verdicts = unique(predictions["verdicts"], "query_id")
    if verdicts.keys() != cases.keys():
        raise ValueError("verdict coverage differs from corpus")
    for verdict in verdicts.values():
        if verdict["outcome"] not in {"pass", "fail", "conditional", "escalate"}:
            raise ValueError("unknown verdict outcome")
        if not verdict.get("policy_id") or not verdict.get("policy_version"):
            raise ValueError("missing policy evidence")
    report = {"schema":"r08.quality.v1", "corpus_sha256":digest,
              "prediction_sha256":hashlib.sha256(json.dumps(predictions, sort_keys=True, allow_nan=False).encode()).hexdigest(),
              "case_count":len(cases), "execution_authority":False,
              "status":"needs_human_labels", "false_approval_rate":None, "false_rejection_rate":None,
              "human_label_count":0}
    if labels is None:
        return report
    if labels.get("corpus_sha256") != digest or labels.get("provenance") != "human":
        raise ValueError("require human-attested labels bound to this corpus")
    if not isinstance(labels.get("reviewer"), str) or not labels["reviewer"].strip():
        raise ValueError("require named reviewer")
    judgments = unique(labels["labels"], "id")
    if judgments.keys() != cases.keys():
        raise ValueError("label coverage differs from corpus")
    matrix = Counter()
    for key, label in judgments.items():
        if label["judgment"] not in {"accept", "reject", "review"}:
            raise ValueError("unknown human judgment")
        if not isinstance(label.get("reason"), str) or not label["reason"].strip():
            raise ValueError("require human rationale")
        matrix[(label["judgment"], verdicts[key]["outcome"])] += 1
    accepted = sum(n for (human, _), n in matrix.items() if human == "accept")
    rejected = sum(n for (human, _), n in matrix.items() if human == "reject")
    fa, fr = matrix[("reject", "pass")], matrix[("accept", "fail")]
    report.update(status="measured_not_accepted", human_label_count=len(judgments),
                  reviewer=labels["reviewer"], provenance="human_attested_not_authenticated",
                  labels_sha256=hashlib.sha256(json.dumps(labels, sort_keys=True).encode()).hexdigest(),
                  false_approvals=fa, false_rejections=fr, human_accept_count=accepted, human_reject_count=rejected,
                  false_approval_rate=fa/rejected if rejected else None,
                  false_rejection_rate=fr/accepted if accepted else None,
                  decisive_count=sum(n for (_, machine), n in matrix.items() if machine in {"pass", "fail"}),
                  decision_coverage=sum(n for (_, machine), n in matrix.items() if machine in {"pass", "fail"}) / len(cases),
                  deferred_count=sum(n for (_, machine), n in matrix.items() if machine in {"conditional", "escalate"}),
                  confusion_matrix={f"{human}/{machine}":n for (human, machine), n in sorted(matrix.items())})
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("corpus", type=Path)
    parser.add_argument("predictions", type=Path)
    parser.add_argument("--labels", type=Path)
    args = parser.parse_args()
    try:
        report = assess(args.corpus.read_bytes(), json.loads(args.predictions.read_text()),
                        json.loads(args.labels.read_text()) if args.labels else None)
    except (ValueError, KeyError, TypeError, OSError) as error:
        parser.error(str(error))
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
