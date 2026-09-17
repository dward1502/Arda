"""Offline tests of the owned inference transport, never live provider responses."""
import os
import json
import socket
import struct
import unittest
from pathlib import Path
from types import SimpleNamespace

scope = {}
exec((Path(__file__).resolve().parents[2] / 'src/bin/snapshot_runtime/inference_transport.py').read_text(), scope)

class TransportTests(unittest.TestCase):
    def test_destination_and_size_are_denied_before_io(self):
        for method, url, body in [('GET', 'http://127.0.0.1:7171/v1/chat/completions', b'{}'), ('POST', 'http://evil.invalid/', b'{}'), ('POST', 'http://127.0.0.1:7171/v1/chat/completions', b'x' * 2097153)]:
            with self.assertRaises(RuntimeError):
                scope['broker_send'](None, SimpleNamespace(method=method, url=url, content=body))

    def test_private_channel_and_disconnect_without_fallback(self):
        parent, child = socket.socketpair()
        pid = os.fork()
        if pid == 0:
            try:
                parent.close()
                os.dup2(child.fileno(), 0)
                request = SimpleNamespace(method='POST', url='http://127.0.0.1:7171/v1/chat/completions', content=b'{"fixture":true}')
                result = scope['broker_send'](SimpleNamespace(Response=lambda status, **kw: (status, kw)), request)
                assert result[0] == 200 and result[1]['content'] == b'fixture SSE'
                try:
                    scope['broker_send'](None, request)
                except (RuntimeError, BrokenPipeError):
                    os._exit(0)
                os._exit(2)
            except BaseException:
                os._exit(3)
        child.close()
        with parent.makefile('rwb', buffering=0) as channel:
            size = struct.unpack('!I', channel.read(4))[0]
            self.assertEqual(json.loads(channel.read(size)), {'fixture': True})
            frame = json.dumps({'status':200, 'headers':{}, 'body':'fixture SSE'}).encode()
            channel.write(struct.pack('!I', len(frame)) + frame)
        parent.close()
        self.assertEqual(os.waitpid(pid, 0)[1], 0)

if __name__ == '__main__':
    unittest.main()
