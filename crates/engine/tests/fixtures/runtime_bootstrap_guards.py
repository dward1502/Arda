"""Offline tests of the actual owned HTTP wrapper; all HTTP is mocked."""
import ast
import contextlib
import importlib
import io
import json
from pathlib import Path
import sys
import unittest
import httpx

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "src/bin/snapshot_runtime/bootstrap.py"
PROFILE = json.loads((Path(__file__).resolve().parents[4] / "config/retained/hermes-profile.json").read_text())

class GuardTests(unittest.TestCase):
    def setUp(self):
        original = httpx.Client.send
        self.addCleanup(setattr, httpx.Client, "send", original)
        block = next(node for node in ast.parse(SOURCE.read_text()).body
                     if isinstance(node, ast.If) and ast.unparse(node.test) == "mode == 'chat'")
        scope = {"importlib": importlib, "profile": PROFILE, "json": json, "sys": sys}
        exec(compile(ast.Module(body=block.body, type_ignores=[]), str(SOURCE), "exec"), scope)

    def headers(self, route_class="tool_oriented"):
        return {"x-manwe-route-id": "mock-route-for-guard-test",
                "x-manwe-provider-id": "edge_core",
                "x-manwe-model-id": PROFILE["model"]["default"],
                "x-manwe-route-class": route_class}

    def test_tool_capable_chat_requires_first_tool_result(self):
        seen = []
        def transport(request):
            seen.append(json.loads(request.content))
            self.assertEqual(int(request.headers['content-length']), len(request.content))
            return httpx.Response(200, headers=self.headers())
        with httpx.Client(transport=httpx.MockTransport(transport)) as client:
            for messages in ([{'role': 'user', 'content': 'Review files'}],
                             [{'role': 'user', 'content': 'Review files'},
                              {'role': 'tool', 'tool_call_id': 'actual-call', 'content': 'actual output'}]):
                with contextlib.redirect_stderr(io.StringIO()):
                    client.post('http://127.0.0.1:7171/v1/chat/completions', json={
                        'messages': messages, 'tools': [{'type': 'function', 'function': {'name': 'read_file'}}],
                        'tool_choice': 'auto', 'stream': True})
        self.assertEqual(seen[0]['tool_choice'], 'required')
        self.assertEqual(seen[1]['tool_choice'], 'auto')
        self.assertTrue(all(body['stream'] for body in seen))

    def test_redirects_never_forward_inference_body(self):
        for status in (307, 308):
            seen = []
            def transport(request):
                seen.append(str(request.url))
                if len(seen) == 1:
                    return httpx.Response(status, headers={"location": "https://unapproved.invalid/completions"})
                return httpx.Response(200, headers=self.headers())
            with httpx.Client(transport=httpx.MockTransport(transport), follow_redirects=True) as client:
                with contextlib.redirect_stderr(io.StringIO()):
                    response = client.post("http://127.0.0.1:7171/v1/chat/completions", json={"fixture": "not a real inference request"})
                self.assertEqual(response.status_code, status)
                self.assertEqual(seen, ["http://127.0.0.1:7171/v1/chat/completions"])

    def test_unapproved_route_class_is_rejected(self):
        with httpx.Client(transport=httpx.MockTransport(lambda request: httpx.Response(200, headers=self.headers("unapproved-class")))) as client:
            with contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaisesRegex(RuntimeError, "unapproved route"):
                    client.post("http://127.0.0.1:7171/v1/chat/completions", json={})

if __name__ == "__main__":
    unittest.main()
