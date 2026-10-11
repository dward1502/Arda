#!/usr/bin/env python3
"""Bounded operational experiment; run on the authorized server only.

Stops only Bonsai; compares Nanbeige on GPU0, loopback only, then restores it.
Not a benchmark qualification. An independent timer restores Bonsai if interrupted.
"""
import hashlib
import json
import os
import sys
from pathlib import Path
import subprocess
import time
import urllib.request
from datetime import datetime, timezone

H = Path.home()
CANDIDATE = '--saluki' in sys.argv
OUT = H / ('inference-assessment/saluki-cuda-extended.json' if CANDIDATE else 'inference-assessment/runtime-cuda-extended.json')
UNIT = 'inference-assessment-smoke'
RESTORE = 'inference-assessment-restore'
SERVICE = 'llama-server-bonsai-q1.service'
MODEL = H / 'models/Nanbeige4.2-3B-GGUF/Nanbeige4.2-3B-Q4_K_M.gguf'
FIXTURES = [
    ('coding', 'Diagnose this Rust error. Reply only A or B. Error: future cannot be sent between threads safely, MutexGuard used across await. A: stale Cargo.lock. B: std::sync::MutexGuard alive across await.', 'B'),
    ('research', 'Use only the following synthetic sources. [A] January: service runs on port 7000. [B] February correction: port 7001 replaces 7000. What is the corrected port? Reply exactly 7001 [B].', '7001 [B]'),
    ('abstention', 'The complete supplied repository has only Cargo.toml and src/main.rs. What function is in src/router.rs? Reply exactly CANNOT_VERIFY if absent.', 'CANNOT_VERIFY'),
]

def run(*args, check=True):
    return subprocess.run(args, capture_output=True, text=True, check=check, timeout=30)

def request(path, payload=None):
    data = None if payload is None else json.dumps(payload).encode()
    return json.load(urllib.request.urlopen(urllib.request.Request(
        'http://127.0.0.1:18099' + path, data=data,
        headers={'Content-Type': 'application/json'}), timeout=120))

def main():
    if H.name != 'annunimas-server':
        raise SystemExit('Run only on annunimas-server')
    global MODEL
    if CANDIDATE:
        MODEL = H / 'inference-assessment/models/Underdog-Saluki-27B-1.0-IQ2-mix.gguf'
    result = {'utc': datetime.now(timezone.utc).isoformat(),
              'scope': 'same model, GPU0, context 4096, sequential smoke; not promotion',
              'fixtures': FIXTURES, 'runs': []}
    run('systemctl', '--user', 'is-active', SERVICE)
    run('systemd-run', '--user', '--unit=' + RESTORE, '--on-active=1200s',
        '/usr/bin/systemctl', '--user', 'start', SERVICE)
    try:
        run('systemctl', '--user', 'stop', SERVICE)
        for label, binary, lib in [
            ('old', H / 'llama.cpp-nanbeige/build-cuda/bin/llama-server',
             str(H / 'miniforge3/lib') + ':/usr/local/cuda/lib64:/usr/lib64'),
            ('b11430', H / 'inference-assessment/b11430-cuda/llama-b11430/llama-server',
             str(H / 'inference-assessment/b11430-cuda/llama-b11430') + ':' + str(H / 'inference-assessment/b11430-cuda/cudart-llama-b11430-bin-ubuntu-cuda-12.8-x64')),
        ]:
            if '--new-only' in sys.argv and label == 'old':
                continue
            row = {'runtime': label, 'binary': str(binary),
                   'binary_sha256': hashlib.file_digest(binary.open('rb'), 'sha256').hexdigest(),
                   'checks': []}
            result['runs'].append(row)
            args = [str(binary), '--model', str(MODEL), '--alias', 'control',
                    '--host', '127.0.0.1', '--port', '18099', '--ctx-size', '4096',
                    '--parallel', '1', '--n-gpu-layers', '40' if CANDIDATE else '999', '--flash-attn', 'on',
                    '--cache-type-k', 'q4_0', '--cache-type-v', 'q4_0', '--jinja',
                    '--chat-template-kwargs', '{"enable_thinking":false}']
            row['argv'] = args
            row['model_sha256'] = hashlib.file_digest(MODEL.open('rb'), 'sha256').hexdigest()
            row['model_bytes'] = MODEL.stat().st_size
            try:
                devices = subprocess.run([str(binary), '--list-devices'], env=dict(os.environ, CUDA_VISIBLE_DEVICES='0', LD_LIBRARY_PATH=lib), capture_output=True, text=True, timeout=30)
                row['devices'] = devices.stdout + devices.stderr
                if devices.returncode or 'CUDA0:' not in row['devices']:
                    raise RuntimeError('CUDA backend unavailable; do not accept CPU fallback')
                run('systemd-run', '--user', '--unit=' + UNIT, '--collect',
                    '--property=RuntimeMaxSec=540', '--property=TimeoutStopSec=10',
                    '--setenv=CUDA_VISIBLE_DEVICES=0', '--setenv=LD_LIBRARY_PATH=' + lib,
                    *args)
                deadline = time.monotonic() + 50
                while True:
                    try:
                        row['health'] = request('/health')
                        break
                    except Exception:
                        state = run('systemctl', '--user', 'is-active', UNIT, check=False)
                        if state.returncode or time.monotonic() > deadline:
                            raise RuntimeError('Startup failed or exceeded 50 seconds')
                        time.sleep(1)
                row['models'] = request('/v1/models')
                row['props'] = request('/props')
                pid = run('systemctl', '--user', 'show', UNIT, '-p', 'MainPID', '--value').stdout.strip()
                row['actual_exe'] = os.readlink('/proc/' + pid + '/exe')
                row['loaded_libraries'] = sorted(set(line.split()[-1] for line in Path('/proc/' + pid + '/maps').read_text().splitlines() if any(n in line for n in ['libggml', 'libcuda', 'libcublas'])))
                row['gpu'] = run('nvidia-smi', '--query-gpu=index,uuid,memory.used,memory.total',
                                 '--format=csv').stdout
                for name, prompt, expected in FIXTURES:
                    start = time.monotonic()
                    response = request('/v1/chat/completions', {
                        'model': 'control', 'messages': [{'role': 'user', 'content': prompt}],
                        'temperature': 0, 'seed': 42, 'max_tokens': 96, 'stream': False,
                        'chat_template_kwargs': {'enable_thinking': False}})
                    row['checks'].append({'case': name, 'response': response,
                        'elapsed_seconds': time.monotonic() - start,
                        'passed': (response['choices'][0]['message'].get('content') or '').strip() == expected})
                import benchmark_local_model as bench
                saved_check = bench.run_check
                bench.run_check = lambda name, fn, category: saved_check(name, fn, category) if category == 'tools' else {'category': category, 'skipped': True}
                client = bench.Client('http://127.0.0.1:18099', 'control', 120)
                raw = []
                original = client._request
                def retained_request(path, payload, stream=False):
                    response = original(path, payload, stream)
                    if not stream:
                        raw.append({'request': payload, 'response': response})
                    return response
                client._request = retained_request
                try:
                    row['tools'] = [c for c in bench.correctness_suite(client, 1024, True) if c.get('category') == 'tools']
                    row['stream'] = client.streamed([{'role': 'user', 'content': 'Reply exactly READY'}], 32)
                finally:
                    row['tool_raw'] = raw
                    bench.run_check = saved_check
            except Exception as exc:
                row['error'] = str(exc)
            finally:
                run('systemctl', '--user', 'stop', UNIT, check=False)
                row['journal'] = run('journalctl', '--user', '-u', UNIT, '-n', '100',
                                     '--no-pager', check=False).stdout
                OUT.write_text(json.dumps(result, indent=2))
    finally:
        run('systemctl', '--user', 'stop', UNIT, check=False)
        restored = run('systemctl', '--user', 'start', SERVICE, check=False)
        result['restore_start_exit'] = restored.returncode
        result['restore_state'] = run('systemctl', '--user', 'is-active', SERVICE, check=False).stdout
        OUT.write_text(json.dumps(result, indent=2))
        if restored.returncode == 0:
            deadline = time.monotonic() + 60
            while time.monotonic() < deadline:
                try:
                    result['restored_health'] = json.load(urllib.request.urlopen('http://127.0.0.1:8097/health', timeout=3))
                    if result['restored_health'].get('status') == 'ok':
                        run('systemctl', '--user', 'stop', RESTORE + '.timer', check=False)
                        break
                except Exception:
                    time.sleep(1)
            OUT.write_text(json.dumps(result, indent=2))
    print(json.dumps({'out': str(OUT), 'restore': result.get('restored_health'), 'runs': [{'runtime': r['runtime'], 'error': r.get('error'), 'checks': [(c['case'], c['passed']) for c in r['checks']], 'tools': r.get('tools'), 'stream': r.get('stream'), 'gpu': r.get('gpu')} for r in result['runs']]}))

if __name__ == '__main__':
    main()
