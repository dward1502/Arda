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
    def test_readonly_tools_filter_definitions_and_deny_dispatch(self):
        from types import SimpleNamespace
        scope = {}
        exec((SOURCE.parent / "readonly_tools.py").read_text(), scope)
        names = ["read_file", "search_files", "write_file", "patch", "terminal"]
        called = []
        tools = SimpleNamespace(get_tool_definitions=lambda **kw: [
            {"function": {"name": name}} for name in names])
        registry = SimpleNamespace(dispatch=lambda name, args, **kw: called.append(name) or "ok")
        scope["enforce_readonly_tools"](tools, registry)
        self.assertEqual([t["function"]["name"] for t in tools.get_tool_definitions()], names[:2])
        for name in names:
            result = registry.dispatch(name, {})
            self.assertIn("error", json.loads(result))
        self.assertEqual(called, [])

    def test_readonly_chat_bootstrap_installs_mutation_guard(self):
        from types import SimpleNamespace
        called = []
        registry = SimpleNamespace(dispatch=lambda name, args, **kw: called.append(name) or "ok")
        tools = SimpleNamespace(get_tool_definitions=lambda **kw: [])
        modules = {"model_tools": tools, "tools.registry": SimpleNamespace(registry=registry)}
        scope = {"mode": "chat_read_only", "importlib": SimpleNamespace(import_module=modules.__getitem__)}
        exec((SOURCE.parent / "readonly_tools.py").read_text(), scope)
        block = next(node for node in ast.parse(SOURCE.read_text()).body
                     if isinstance(node, ast.If) and ast.unparse(node.test) == "mode == 'chat_read_only'")
        exec(compile(ast.Module(body=[block], type_ignores=[]), str(SOURCE), "exec"), scope)
        self.assertEqual(scope["mode"], "chat")
        for name in ("write_file", "patch", "terminal"):
            self.assertNotEqual(registry.dispatch(name, {}), "ok")
        self.assertEqual(called, [])
        self.assertIn("error", json.loads(registry.dispatch("read_file", {})))
        self.assertEqual(called, [])

    def setUp(self):
        original = httpx.Client.send
        self.addCleanup(setattr, httpx.Client, "send", original)
        block = next(node for node in ast.parse(SOURCE.read_text()).body
                     if isinstance(node, ast.If) and ast.unparse(node.test) == "mode == 'chat'")
        from types import SimpleNamespace
        agent = type("Agent", (), {"run_conversation": lambda *args, **kw: None})
        registry = SimpleNamespace(dispatch=lambda *args, **kw: "ok")
        def load(name):
            return ({"run_agent": SimpleNamespace(AIAgent=agent),
                     "tools.registry": SimpleNamespace(registry=registry)}.get(name)
                    or importlib.import_module(name))
        scope = {"importlib": SimpleNamespace(import_module=load), "profile": PROFILE, "json": json, "sys": sys}
        scope["broker_send"] = lambda httpx_module, request: self.transport(request)
        exec((SOURCE.parent / "finalization.py").read_text(), scope)
        exec(compile(ast.Module(body=block.body, type_ignores=[]), str(SOURCE), "exec"), scope)
        self.scope = scope

    def test_finalization_denies_tools_and_second_http_request(self):
        self.scope["finalization_state"]["active"] = True
        with self.assertRaisesRegex(RuntimeError, "tools forbidden"):
            self.scope["registry"].dispatch("read_file", {})
        seen = []
        def transport(request):
            seen.append(json.loads(request.content))
            return httpx.Response(200, headers=self.headers())
        self.transport = transport
        with httpx.Client(transport=httpx.MockTransport(transport)) as client:
            body = {"messages": [], "tools": [{"type": "function"}], "tool_choice": "required"}
            client.post("http://127.0.0.1:7171/v1/chat/completions", json=body)
            with self.assertRaisesRegex(RuntimeError, "budget exhausted"):
                client.post("http://127.0.0.1:7171/v1/chat/completions", json=body)
        self.assertEqual(len(seen), 1)
        self.assertNotIn("tools", seen[0])
        self.assertNotIn("tool_choice", seen[0])

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
        self.transport = transport
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
            self.transport = transport
            with httpx.Client(transport=httpx.MockTransport(transport), follow_redirects=True) as client:
                with contextlib.redirect_stderr(io.StringIO()):
                    response = client.post("http://127.0.0.1:7171/v1/chat/completions", json={"fixture": "not a real inference request"})
                self.assertEqual(response.status_code, status)
                self.assertEqual(seen, ["http://127.0.0.1:7171/v1/chat/completions"])

    def test_unapproved_route_class_is_rejected(self):
        self.transport = lambda request: httpx.Response(200, headers=self.headers("unapproved-class"))
        with httpx.Client(transport=httpx.MockTransport(lambda request: httpx.Response(200, headers=self.headers("unapproved-class")))) as client:
            with contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaisesRegex(RuntimeError, "unapproved route"):
                    client.post("http://127.0.0.1:7171/v1/chat/completions", json={})

if __name__ == "__main__":
    unittest.main()
