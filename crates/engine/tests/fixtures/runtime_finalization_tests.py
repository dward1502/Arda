"""Worker finalization tests; no provider or ambient Hermes state."""
import json
import pathlib
import runpy
import unittest

module = runpy.run_path(str(pathlib.Path(__file__).parents[2] / "src/bin/snapshot_runtime/finalization.py"))
VALID = json.dumps(dict(schema_version="arda.hermes-job-result.v1", status="succeeded", summary="Observed work", tool_evidence=[], test_evidence=[], artifacts=[]))


class FinalizationTests(unittest.TestCase):
    def agent(self, first, last=VALID):
        state = {"active": False}
        class Agent:
            def __init__(self):
                self.tools = ["terminal"]
                self.valid_tool_names = {"terminal"}
                self.max_iterations = 10
                self.session_id = "canonical"
                self.calls = []
            def run_conversation(self, prompt, **kwargs):
                self.calls.append((prompt, kwargs, list(self.tools), self.max_iterations))
                if len(self.calls) == 1:
                    return first
                assert state["active"] and not self.valid_tool_names
                return dict(completed=True, final_response=last, messages=kwargs["conversation_history"])
        module["install_finalization"](Agent, state)
        return Agent(), state

    def test_missing_field_gets_one_tool_free_turn_with_original_history(self):
        history = [{"role": "assistant", "content": "missing artifacts"}]
        agent, state = self.agent(dict(completed=True, final_response='{}', messages=history))
        self.assertEqual(agent.run_conversation("work")["final_response"], VALID)
        self.assertEqual(len(agent.calls), 2)
        self.assertIs(agent.calls[1][1]["conversation_history"], history)
        self.assertEqual(agent.calls[1][2:], ([], 1))
        self.assertEqual(agent.tools, ["terminal"])
        self.assertFalse(state["active"])

    def test_valid_response_and_failed_turn_do_not_retry(self):
        for first in [dict(completed=True, final_response=VALID), dict(completed=False, failed=True, final_response="error")]:
            agent, _ = self.agent(first)
            self.assertIs(agent.run_conversation("work"), first)
            self.assertEqual(len(agent.calls), 1)

    def test_invalid_finalization_fails_closed_and_restores_tools(self):
        agent, state = self.agent(dict(completed=True, final_response="prose", messages=[{}]), "bad")
        with self.assertRaisesRegex(RuntimeError, "invalid result JSON"):
            agent.run_conversation("work")
        self.assertEqual(len(agent.calls), 2)
        self.assertEqual(agent.tools, ["terminal"])
        self.assertFalse(state["active"])


    def test_finalization_cannot_promote_failure_or_block(self):
        for draft in ('{"status":"failed"}', '{"status":"cancelled"}', 'VERDICT: BLOCK\nMissing evidence'):
            agent, _ = self.agent(dict(completed=True, final_response=draft, messages=[{}]))
            with self.assertRaisesRegex(RuntimeError, "promote a failed outcome"):
                agent.run_conversation("work")


if __name__ == "__main__":
    unittest.main()
