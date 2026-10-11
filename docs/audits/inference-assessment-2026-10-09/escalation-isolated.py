"""Bounded isolated subscription probe; no production state or service changes."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.request
import urllib.error

REPO = Path('/var/home/mythos/Eregion/Arda')
OUT = Path(__file__).with_suffix('.json')
root = Path(tempfile.mkdtemp(prefix='escalation-isolated-', dir='/var/home/mythos/.hermes/cache/scratch'))
source = (REPO / 'config/manwe.providers.toml').read_text()
block = next(b for b in source.split('[[provider]]') if '\nid = "openai_sub"\n' in b)
(root / 'providers.toml').write_text('[[provider]]' + block)
binary = root / 'manwe'
shutil.copy2(REPO / 'target/debug/manwe', binary)
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    port = sock.getsockname()[1]
env = {'PATH': os.environ['PATH'], 'HOME': str(root), 'HERMES_HOME': '/var/home/mythos/.hermes',
       'ARDA_ROOT': str(root), 'ARDA_HOME': str(root), 'ARDA_MANWE_STATE_DIR': str(root / 'state'),
       'ARDA_MANWE_PROVIDER_CONFIG': str(root / 'providers.toml'),
       'ARDA_FLEET_BOOTSTRAP_STATE': str(root / 'absent-bootstrap'),
       'ARDA_PROVIDER_INTELLIGENCE_PATH': str(root / 'absent-intelligence')}
result = {'scope': 'isolated debug artifact text-only subscription transport probe; not production or role qualification',
          'binary': str(binary), 'sha256': hashlib.file_digest(binary.open('rb'), 'sha256').hexdigest(),
          'state_root': str(root), 'port': port, 'credential_override_present': False, 'passed': False}

def request(path, payload=None):
    req = urllib.request.Request(f'http://127.0.0.1:{port}' + path,
        data=None if payload is None else json.dumps(payload).encode(),
        headers={'Content-Type': 'application/json', 'x-manwe-task-type': 'chat'})
    with urllib.request.urlopen(req, timeout=90 if payload else 2) as response:
        return {'status': response.status, 'headers': {k.lower(): v for k, v in response.headers.items() if k.lower().startswith('x-manwe')}, 'body': json.load(response)}

with (root / 'process.log').open('w') as log:
    proc = subprocess.Popen([str(binary), '--bind', '127.0.0.1', '--port', str(port), '--config', str(root / 'absent.toml')],
                            cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    try:
        deadline = time.monotonic() + 20
        while True:
            if proc.poll() is not None:
                raise RuntimeError('isolated router exited during startup')
            try:
                request('/healthz')
                break
            except OSError:
                if time.monotonic() >= deadline:
                    raise
                time.sleep(.2)
        payload = {'model': 'openai_sub/gpt-6-astra', 'allow_forced_provider_fallback': False,
                   'workload_role': 'interactive', 'context_window_target': 4096, 'max_tokens': 64,
                   'stream': False, 'messages': [{'role': 'user', 'content': 'Reply with exactly ROUTE_READY.'}]}
        result['request'] = payload
        OUT.write_text(json.dumps(result, indent=2)+'\n')
        started = time.monotonic()
        result['response'] = response = request('/v1/chat/completions', payload)
        result['elapsed_s'] = time.monotonic() - started
        choice = response['body']['choices'][0]
        result['passed'] = (choice['message']['content'].strip() == 'ROUTE_READY' and choice['finish_reason'] == 'stop'
                            and not choice['message'].get('tool_calls')
                            and response['headers'].get('x-manwe-provider-id') == 'openai_sub'
                            and response['headers'].get('x-manwe-model-id') == 'gpt-6-astra')
    except urllib.error.HTTPError as error:
        result.update(status=error.code, error=error.read(16384).decode(errors='replace'))
    except Exception as error:
        result['error'] = str(error)
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=10)
        result['candidate_stopped'] = proc.poll() is not None
        with socket.socket() as sock:
            result['listener_absent'] = sock.connect_ex(('127.0.0.1', port)) != 0
        result['desktop_state'] = subprocess.check_output(['systemctl', '--user', 'show', 'mesh-llm.service', '-p', 'ActiveState', '-p', 'MainPID'], text=True).strip()
        OUT.write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
raise SystemExit(0 if result['passed'] and result['candidate_stopped'] and result['listener_absent'] else 1)
