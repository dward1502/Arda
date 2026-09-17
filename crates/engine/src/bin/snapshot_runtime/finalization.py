"""One tool-free formatting turn, in the same durable Hermes session."""
import json

FINALIZE_PROMPT = (
    "Arda result finalization only. Do not execute work or call tools. Preserve the "
    "observed outcome and any blocking findings from the preceding turn; do not invent "
    "evidence, artifact hashes, or success. Return ONLY one JSON object with ALL keys: "
    'schema_version ("arda.hermes-job-result.v1"), status ("succeeded", "failed", or '
    '"cancelled"), summary (string), tool_evidence ([]), test_evidence ([]), artifacts '
    '([] unless outputs were actually created; entries contain path and digest). '
    "For review, summary starts VERDICT: APPROVE or VERDICT: BLOCK; a block requires "
    "failed status. Files merely read are not artifacts. Missing or failed work stays failed. "
    "Arda derives tool/test evidence from the saved transcript; those two arrays MUST "
    "be empty, never populated with commands or exit codes. Do not list an artifact "
    "unless its real SHA-256 was computed in the preceding tool execution. Never guess "
    "a hash. The exact response shape (replace only status, summary, and verified "
    'artifacts as appropriate) is: {"schema_version":"arda.hermes-job-result.v1",'
    '"status":"succeeded","summary":"Observed outcome",'
    '"tool_evidence":[],"test_evidence":[],"artifacts":[]}.'
)


def result_shape_valid(text):
    try:
        value = json.loads(text)
    except (TypeError, ValueError):
        return False
    return (isinstance(value, dict)
            and set(value) == {"schema_version", "status", "summary", "tool_evidence", "test_evidence", "artifacts"}
            and value["schema_version"] == "arda.hermes-job-result.v1"
            and value["status"] in ("succeeded", "failed", "cancelled")
            and isinstance(value["summary"], str) and bool(value["summary"].strip())
            and value["tool_evidence"] == [] and value["test_evidence"] == []
            and isinstance(value["artifacts"], list))


def install_finalization(agent_class, state):
    original = agent_class.run_conversation

    def run(agent, *args, **kwargs):
        result = original(agent, *args, **kwargs)
        if (result.get("failed") or result.get("interrupted")
                or not result.get("completed")
                or result_shape_valid(result.get("final_response"))):
            return result
        history = result.get("messages")
        if not isinstance(history, list) or not history:
            raise RuntimeError("finalization requires the original session history")
        session_id = agent.session_id
        saved = (agent.tools, agent.valid_tool_names, agent.max_iterations)
        state.update(active=True, requests=0)
        agent.tools, agent.valid_tool_names, agent.max_iterations = [], set(), 1
        try:
            finalized = original(agent, FINALIZE_PROMPT, conversation_history=history)
        finally:
            agent.tools, agent.valid_tool_names, agent.max_iterations = saved
            state["active"] = False
        if agent.session_id != session_id:
            raise RuntimeError("finalization changed the canonical session")
        if not result_shape_valid(finalized.get("final_response")):
            raise RuntimeError("bounded finalization returned invalid result JSON")
        final_value = json.loads(finalized["final_response"])
        draft = result.get("final_response", "")
        try:
            prior = json.loads(draft)
        except (TypeError, ValueError):
            prior = None
        prior_status = prior.get("status") if isinstance(prior, dict) else None
        if (prior_status in ("failed", "cancelled") and final_value["status"] != prior_status
                or "VERDICT: BLOCK" in (draft or "") and (final_value["status"] != "failed"
                    or not final_value["summary"].startswith("VERDICT: BLOCK"))):
            raise RuntimeError("finalization attempted to promote a failed outcome")
        return finalized

    agent_class.run_conversation = run
