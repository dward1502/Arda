#!/usr/bin/env python3
"""Read-only qualification of the Aule consumer against Beelink's /proc and systemd.
No inference requests, service changes or automatic actions. Run from repository root.
"""
import json
from pathlib import Path
import shlex
import subprocess
import time

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
REMOTE = '''import json,time,subprocess,pathlib
mem={k:int(v.split()[0])*1024 for k,v in (line.split(':',1) for line in pathlib.Path('/proc/meminfo').read_text().splitlines())}
stat=pathlib.Path('/proc/stat').read_text()
print(json.dumps({'observed_at':time.time(),'meminfo_bytes':mem,'stat':stat,
'vmstat':pathlib.Path('/proc/vmstat').read_text(),
'pressure':{n:pathlib.Path('/proc/pressure/'+n).read_text() for n in ['cpu','memory','io']},
'failed_system':json.loads(subprocess.check_output(['systemctl','--system','list-units','--all','--state=failed','--output=json','--no-pager','--plain'])),
'failed_user':json.loads(subprocess.check_output(['systemctl','--user','list-units','--all','--state=failed','--output=json','--no-pager','--plain']))}))'''


def main():
    started = time.time()
    command = [str(ROOT / 'target/debug/arda-cli'), 'metrics', 'home-server',
               '--prometheus-url', 'http://100.103.125.88:9090', '--node', 'beelink',
               '--instance', '100.103.125.88:9100', '--ssh-target', 'beelink']
    report = json.loads(subprocess.check_output(command, timeout=45))
    direct = json.loads(subprocess.check_output([
        'ssh', '-o', 'BatchMode=yes', '-o', 'StrictHostKeyChecking=yes', '-o', 'ConnectTimeout=5',
        'beelink', 'python3 -c ' + shlex.quote(REMOTE)], timeout=20))
    observations = report['observations']
    checks = {}
    checks['fresh_identity'] = all(o['status'] == 'measured'
        and o['labels']['node'] == 'beelink'
        and o['labels']['instance'] == '100.103.125.88:9100'
        and 0 <= report['generated_at'] - o['sample_end'] <= report['max_sample_age_seconds']
        for o in observations) and bool(observations)
    cpu = [o for o in observations if o['metric'] == 'node_cpu_seconds_total']
    direct_cpu = {l.split()[0][3:] for l in direct['stat'].splitlines() if l.startswith('cpu') and l[3:4].isdigit()}
    checks['cpu_series_match_proc'] = {o['labels']['cpu'] for o in cpu} == direct_cpu
    by_name = {o['metric']: o for o in observations}
    checks['memory_and_swap_totals_match_proc'] = all(by_name[name]['value'] == direct['meminfo_bytes'][key]
        for name, key in [('node_memory_MemTotal_bytes', 'MemTotal'), ('node_memory_SwapTotal_bytes', 'SwapTotal')])
    checks['unit_names_match_direct_systemctl'] = all(
        u['status'] == 'measured' and sorted(v['unit'] for v in u['failed_units']) == sorted(v['unit'] for v in direct[key])
        for u, key in zip(report['units'], ['failed_system', 'failed_user'])) and len(report['units']) == 2
    checks['every_failed_unit_in_summary'] = all(v['unit'] in report['summary'] for key in ['failed_system', 'failed_user'] for v in direct[key])
    checks['no_model_or_actions'] = report['model_interpretation'] is None and report['actions'] == [] and report['read_only'] is True
    checks['source_derivations_match'] = True
    for o in observations:
        raw = report['sources']['prometheus']['raw']['data']['result'][o['source_row']]
        if o['value'] is None:
            checks['source_derivations_match'] = False
            continue
        values = raw['values']
        expected = float(values[-1][1])
        if o['unit'] != 'bytes':
            expected = (expected - float(values[0][1])) / (values[-1][0] - values[0][0])
            if o['unit'] == 'idle_percent':
                expected *= 100
        checks['source_derivations_match'] &= abs(expected - o['value']) < 1e-8
    # Proximity comparison only: live MemAvailable changes between observations.
    memory_delta = direct['meminfo_bytes']['MemAvailable'] - by_name['node_memory_MemAvailable_bytes']['value']
    result = {'started_at': started, 'finished_at': time.time(), 'command': command,
              'checks': checks, 'observations_count': len(observations),
              'direct': direct, 'mem_available_later_minus_sample_bytes': memory_delta,
              'scope': 'single read-only live qualification; not contention, backup, application health or model competence acceptance'}
    (OUT / 'home-server-measurements.json').write_text(json.dumps(report, indent=2) + '\n')
    (OUT / 'home-server-qualification.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'checks': checks, 'observations': len(observations),
          'cpu_series': len(cpu), 'mean_observed_idle_percent': sum(o['value'] for o in cpu) / len(cpu),
          'memory_available_gib': by_name['node_memory_MemAvailable_bytes']['value'] / 2**30,
          'swap_occupied_gib': (by_name['node_memory_SwapTotal_bytes']['value'] - by_name['node_memory_SwapFree_bytes']['value']) / 2**30,
          'swap_in_pages_per_second': by_name['node_vmstat_pswpin']['value'],
          'swap_out_pages_per_second': by_name['node_vmstat_pswpout']['value'],
          'failed_system': [u['unit'] for u in direct['failed_system']]}, indent=2))
    if not all(checks.values()):
        raise SystemExit('qualification failed; inspect retained raw evidence')


if __name__ == '__main__':
    main()
