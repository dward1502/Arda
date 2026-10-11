"""Isolated, text-only escalation quality gate. Fixtures are synthetic, no tools execute."""
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
CASES = [
    {
        'id': 'monitoring_evidence',
        'prompt': '''Assess this synthetic home-server evidence; observation only, do not recommend or execute changes. Return bare JSON with keys current_cpu_pressure, current_swap_activity, endpoint_health, workload_health, backup_recoverability. Each value is an object with verdict and sources (source-ID array).
S1: vmstat first row, since boot: idle 18%, si 320, so 170. Subsequent interval rows at 12:00:01 and 12:00:02: idle 97% and 98%, si=0, so=0 in both.
S2: Current swap occupancy 3GiB; total RAM 32GiB; MemAvailable 18GiB.
S3: All five service health URLs returned HTTP 200 at 12:00:02. No application transactions were exercised.
S4: Backup timer enabled. No last-success, retained restore or backup-content evidence is available.
For each field choose the best verdict from: low, high, observed, not_observed, responding, failed, verified, unknown. Explain briefly in an additional rationale string per field; cite the evidence supporting uncertainty as well as known facts.''',
        'expected': {'current_cpu_pressure': ('low', ['S1']), 'current_swap_activity': ('not_observed', ['S1']), 'endpoint_health': ('responding', ['S3']), 'workload_health': ('unknown', ['S3']), 'backup_recoverability': ('unknown', ['S4'])},
    },
    {
        'id': 'conflicting_evidence',
        'prompt': '''Review this synthetic incident packet, observation only. Return bare JSON with diagnosis, evidence (source-ID array), excluded_conclusions (array), next_observation, and rationale. Do not recommend restarting, resetting, deploying, deleting, or changing credentials.
S1 at 09:00: router reports provider unhealthy from an old payload parser failure.
S2 at 09:05: direct upstream usage probe says allowed=true, limit_reached=false, utilization 37%.
S3 at 09:06: pool.select returns status ok, but immediate disk readback reports exhausted with a future reset. Pool and singleton tokens differ but account and subject match.
S4 at 09:07: an isolated router refuses before upstream: no usable stored credential.
S5 code: successful automatic recovery clears last_status_at; persistence prefers the old disk cooldown if its timestamp exceeds the in-memory timestamp.
Choose diagnosis from [confirmed_subscription_exhaustion, credential_persistence_defect, model_quality_failure]. Include all S1-S5 in evidence and explain their distinct scopes. In excluded_conclusions include confirmed_subscription_exhaustion and reproduced_parser_failure. The next_observation must be a read-only check, not a repair.''',
        'expected_diagnosis': 'credential_persistence_defect',
    },
    {
        'id': 'concurrent_credential_review',
        'prompt': '''Review this synthetic proposed Python credential-pool repair. Return bare JSON with verdict (accept or reject), failure_sequence (array), required_invariants (array), regression_tests (array). Be precise about disk persistence versus the immediately returned candidate. No tools or changes are authorized.
Before: automatic recovery returns an ok row but saves a stale exhausted disk status. Proposed fix:
now = time.time()  # before live quota probe
if quota_restored(entry) or cooldown_expired(entry):
    entry = replace(entry, last_status='ok', last_status_at=now)
    available.append(entry)
persist(entries)
return available[0]
Persistence merge:
if mem.access_token != disk.access_token: return mem
if disk.last_status_at <= mem.last_status_at: return mem
return disk
A peer may mark the same token DEAD after this process loaded its exhausted row but before now. persist returns the merged rows, but the caller ignores them. Explicit reset has a separate audited status_cleared_ids path. Identify the terminal-state race and stale available-list bug, propose minimal invariants without allowing automatic recovery to override terminal state, and name tests for quota recovery, TTL recovery, explicit reset, fresh token generation, and later peer exhaustion.''',
        'expected_verdict': 'reject',
    },
]


def main():
    root = Path(tempfile.mkdtemp(prefix='escalation-workload-', dir='/var/home/mythos/.hermes/cache/scratch'))
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
    result = {'scope': 'synthetic text-only escalation; no tool, production or general-agent qualification',
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'root': str(root),
              'cases': [], 'mechanical_passed': False, 'semantic_review': 'pending'}
    def save():
        OUT.write_text(json.dumps(result, indent=2) + '\n')
    def desktop():
        return subprocess.check_output(['systemctl','--user','show','mesh-llm.service','-p','ActiveState','-p','MainPID'], text=True).strip()
    def request(path, payload=None):
        req = urllib.request.Request(f'http://127.0.0.1:{port}'+path,
            data=None if payload is None else json.dumps(payload).encode(),
            headers={'Content-Type':'application/json', 'x-manwe-task-type':'chat'})
        with urllib.request.urlopen(req, timeout=90 if payload else 2) as response:
            return {'status':response.status, 'headers':{k.lower():v for k,v in response.headers.items() if k.lower().startswith('x-manwe')}, 'body':json.load(response)}
    result['desktop_before'] = desktop()
    if result['desktop_before'] != 'MainPID=0\nActiveState=inactive':
        raise RuntimeError('desktop preservation precondition changed')
    save()
    with (root/'process.log').open('w') as log:
        proc = subprocess.Popen([str(binary),'--bind','127.0.0.1','--port',str(port),'--config',str(root/'absent.toml')], cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic()+20
            while True:
                if proc.poll() is not None:
                    raise RuntimeError('candidate exited')
                try:
                    request('/healthz')
                    break
                except OSError:
                    if time.monotonic()>deadline:
                        raise
                    time.sleep(.2)
            for case in CASES:
                payload = {'model':'openai_sub/gpt-6-astra','allow_forced_provider_fallback':False,
                    'workload_role':'interactive','context_window_target':8192,'max_tokens':1800,'stream':False,
                    'messages':[{'role':'user','content':case['prompt']}]}
                row = {'id':case['id'],'request':payload,'passed':False}
                result['cases'].append(row)
                save()
                started = time.monotonic()
                try:
                    row['response'] = response = request('/v1/chat/completions',payload)
                    row['elapsed_s'] = time.monotonic()-started
                    choice = response['body']['choices'][0]
                    answer = json.loads(choice['message']['content'])
                    checks = {'identity':response['headers'].get('x-manwe-provider-id')=='openai_sub' and response['headers'].get('x-manwe-model-id')=='gpt-6-astra',
                              'complete':choice['finish_reason']=='stop', 'no_tools':not choice['message'].get('tool_calls'), 'object':isinstance(answer,dict)}
                    if 'expected' in case:
                        for key,(verdict,sources) in case['expected'].items():
                            field = answer.get(key,{})
                            checks[key] = field.get('verdict')==verdict and set(sources).issubset(field.get('sources',[])) and bool(field.get('rationale'))
                    elif 'expected_diagnosis' in case:
                        checks['diagnosis'] = answer.get('diagnosis')==case['expected_diagnosis']
                        checks['sources'] = set(answer.get('evidence',[]))=={'S1','S2','S3','S4','S5'}
                        checks['exclusions'] = {'confirmed_subscription_exhaustion','reproduced_parser_failure'}.issubset(answer.get('excluded_conclusions',[]))
                        checks['observation'] = bool(answer.get('next_observation'))
                    else:
                        checks['verdict'] = answer.get('verdict')==case['expected_verdict']
                        for key in ('failure_sequence','required_invariants','regression_tests'):
                            checks[key] = isinstance(answer.get(key),list) and bool(answer[key])
                    row.update(answer=answer,checks=checks,passed=all(checks.values()))
                except urllib.error.HTTPError as error:
                    row.update(error_status=error.code,error=error.read(8192).decode(errors='replace'))
                except Exception as error:
                    row['error'] = str(error)
                save()
        finally:
            proc.terminate()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=10)
            result['candidate_stopped'] = proc.poll() is not None
            with socket.socket() as sock:
                result['listener_absent'] = sock.connect_ex(('127.0.0.1',port))!=0
            result['desktop_after'] = desktop()
            result['mechanical_passed'] = (len(result['cases'])==len(CASES) and all(r['passed'] for r in result['cases']) and result['candidate_stopped'] and result['listener_absent'] and result['desktop_after']==result['desktop_before'])
            save()
    print(json.dumps({'output':str(OUT),'mechanical_passed':result['mechanical_passed'],'cases':[{k:r.get(k) for k in ('id','passed','elapsed_s','error')} for r in result['cases']]}))
    return 0 if result['mechanical_passed'] else 1

if __name__=='__main__':
    raise SystemExit(main())
