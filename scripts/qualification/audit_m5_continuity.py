#!/usr/bin/env python3
"""Read-only M5 evidence audit; never treats prompt presence as semantic use.

Writes a private report to --output. Compare it after restart with --baseline.
This does not submit work, authenticate a conversation, or supply operator acceptance.
"""

import argparse
import hashlib
import json
from pathlib import Path
import sqlite3


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def read_json(path):
    return json.loads(path.read_text())


def ledger(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def readonly(path):
    connection = sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True)
    connection.row_factory = sqlite3.Row
    return connection


def prompt_matches(prompt, receipt, assembly, use):
    """Match the accepted stage, not a stale/failed prompt in the same transcript."""
    if (prompt.get("run_id") != receipt["run_id"]
            or prompt.get("node", {}).get("id") != receipt["node_id"]
            or prompt.get("context_use_receipt") != use):
        return False
    capsule = prompt.get("organism_context_projection") or prompt.get("organism_context_capsule")
    if not capsule:
        return False
    digest = capsule.get("source_capsule_digest", capsule.get("capsule_digest"))
    return (digest == receipt["context_capsule_digest"] == use["capsule_digest"]
            and capsule.get("memories") == assembly["capsule"]["memories"])


def audit(root, sessions_root, objective_ids):
    uses = ledger(root / "data/vaire/context_use_receipts.jsonl")
    outcomes = ledger(root / "data/vaire/context_outcome_receipts.jsonl")
    memories = [read_json(p) for p in (root / "core/state/memory").rglob("*.json")]
    result = {"objectives": {}, "files": {}, "semantic_consumption": "requires review of relevant findings in the accepted output"}
    with readonly(root / "data/arda/objectives.sqlite3") as db:
        # One consistent read view of the authority database.
        db.execute("BEGIN")
        for oid in objective_ids:
            row = db.execute("SELECT * FROM objectives WHERE id=?", (oid,)).fetchone()
            require(row is not None and row["state"] == "completed", f"{oid}: not completed")
            objective = {"authority": dict(row), "leaves": {}, "corrections": []}
            result["objectives"][oid] = objective
            for row in db.execute("SELECT * FROM leaves WHERE objective_id=? ORDER BY id", (oid,)):
                leaf = dict(row)
                run = leaf["execution_run_id"]
                require(run and Path(run).name == run, "invalid run path")
                bound = db.execute("SELECT assembly_json FROM resident_context_bindings WHERE run_id=?", (run,)).fetchone()
                require(bound is not None, f"{run}: missing context binding")
                assembly = json.loads(bound[0])
                context = assembly["capsule"]["context"]
                require(context["lineage"]["objective_id"] == oid and context["lineage"]["run_id"] == run, "wrong context lineage")
                selected = assembly["use_receipt"]["memory_refs"]
                run_root = root / "data/runs" / run
                contract_digest = read_json(run_root / "execution-receipts/execute.json")["project_contract_digest"]
                require(selected == [m["memory_id"] for m in assembly["capsule"]["memories"]], "memory selection mismatch")
                for memory in assembly["capsule"]["memories"]:
                    matches = [m for m in memories if m["id"] == memory["memory_id"]]
                    require(len(matches) == 1, "missing or duplicate governed memory")
                    current = matches[0]
                    require(current["state"] == "active", "selected memory no longer active")
                    require(current["objective_id"] == oid and current["project_id"] == context["lineage"]["project_id"], "memory crossed objective/project scope")
                    require(current["project_contract_digest"] == contract_digest, "memory crossed contract scope")
                    require(current["content"] == memory["content"] and sha(memory["content"].encode()) == memory["content_digest"], "selected memory changed")
                for path in sorted(run_root.rglob("*")):
                    if path.is_file() and path.name != "journal.lock":
                        result["files"][str(path.relative_to(root))] = sha(path.read_bytes())
                transcript = sessions_root / ("run-" + hashlib.sha256(run.encode()).hexdigest()) / "state.db"
                with readonly(transcript) as session_db:
                    sessions = [dict(r) for r in session_db.execute("SELECT id,started_at,ended_at,message_count,tool_call_count FROM sessions ORDER BY id")]
                    prompts = []
                    for r in session_db.execute("SELECT session_id,content FROM messages WHERE role='user' ORDER BY id"):
                        marker = "Canonical node context follows:\n"
                        if r["content"] and marker in r["content"]:
                            value, _ = json.JSONDecoder().raw_decode(r["content"].split(marker, 1)[1])
                            prompts.append((r["session_id"], value))
                stages = ["execute", "verify", "review"] + (["close"] if leaf["stage"] == "complete" else [])
                receipts = []
                prompt_evidence = {}
                for stage in stages:
                    receipt = read_json(run_root / "execution-receipts" / (stage + ".json"))
                    require(receipt["run_id"] == run and receipt["node_id"] == stage, "wrong receipt lineage")
                    require(receipt["status"] == ("failed" if stage == "review" and leaf["stage"] == "failed" else "succeeded"), "unexpected stage result")
                    if receipts:
                        require(receipt["parent_receipts"] == [receipts[-1]["receipt_digest"]], "broken stage chain")
                    receipts.append(receipt)
                    if stage == "close":
                        continue
                    matching_uses = [u for u in uses if "arda://vaire/context-use/" + u["receipt_id"] == receipt["context_use_receipt_ref"]]
                    require(len(matching_uses) == 1, "missing or duplicate use receipt")
                    use = matching_uses[0]
                    require(use["run_id"] == run and use["objective_id"] == oid and use["memory_refs"] == selected, "wrong use lineage")
                    matches = [sid for sid, p in prompts if prompt_matches(p, receipt, assembly, use)]
                    require(matches, f"{run}/{stage}: no matching retained worker prompt")
                    prompt_evidence[stage] = {"sessions": matches, "memory_refs": selected, "use_receipt": use["receipt_id"], "summary_digest": sha(receipt["summary"].encode())}
                run_outcomes = [o for o in outcomes if o.get("run_id") == run]
                promoted = [m for m in memories if m.get("resident_objective_outcome") and m.get("run_id") == run]
                expected_count = 1 if leaf["stage"] == "complete" else 0
                require(len(run_outcomes) == len(promoted) == expected_count, "missing/duplicate outcome or failed result promoted")
                if run_outcomes:
                    outcome = run_outcomes[0]
                    require(outcome["objective_id"] == oid and outcome["context_use_receipt_id"] == assembly["use_receipt"]["receipt_id"], "wrong outcome binding")
                    require(outcome["selected_memory_refs"] == selected, "wrong outcome selection")
                    require(promoted[0]["knowledge_evaluation_reference"] == receipts[-2]["receipt_digest"], "promoted memory lacks accepted review")
                objective["leaves"][leaf["id"]] = {"authority": leaf, "context_digest": sha(bound[0].encode()), "prompts": prompt_evidence, "sessions": sessions, "uses": [u for u in uses if u.get("run_id") == run], "outcomes": run_outcomes, "promoted_memories": promoted}
            for row in db.execute("SELECT * FROM critic_corrections WHERE objective_id=? ORDER BY event_id", (oid,)):
                correction = dict(row)
                record = json.loads(correction["record_json"])
                parent = objective["leaves"][correction["parent_leaf_id"]]["authority"]
                child = objective["leaves"][correction["child_leaf_id"]]["authority"]
                rejected = read_json(root / "data/runs" / parent["execution_run_id"] / "execution-receipts/review.json")
                require(rejected["receipt_digest"] == record["rejected_digest"] and rejected["status"] == "failed", "correction lost rejection binding")
                require(record["parent_run_id"] == parent["execution_run_id"], "correction crossed runs")
                require(child["attempt_offset"] == parent["attempt"] + parent["attempt_offset"], "correction reset attempt budget")
                require(json.loads(child["execution_json"])["objective_plan_receipt"] == sha(correction["record_json"].encode()), "correction lost plan binding")
                objective["corrections"].append(correction)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--sessions-root", type=Path, default=Path.home() / ".local/share/arda/retained/sessions")
    parser.add_argument("--objective", action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    args = parser.parse_args()
    report = audit(args.root.resolve(), args.sessions_root, args.objective)
    if args.baseline:
        require(report == read_json(args.baseline), "canonical evidence changed since baseline")
    # Reports contain private canonical context; never overwrite an earlier report.
    with args.output.open("x") as out:
        args.output.chmod(0o600)
        json.dump(report, out, indent=2)
        out.write("\n")
    print("PASS: scoped memory, retained prompts, terminal lineage and unique outcomes" + ("; baseline unchanged" if args.baseline else ""))


if __name__ == "__main__":
    main()
