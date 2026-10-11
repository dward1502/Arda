#!/usr/bin/env python3
"""Read-only CLI/direct liveness comparison. No installed policy or remediation."""
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SSH = ['ssh', '-o', 'BatchMode=yes', '-o', 'StrictHostKeyChecking=yes', '-o', 'ConnectTimeout=5', 'beelink']

def run(command):
    return subprocess.run(command, check=True, capture_output=True, text=True, timeout=90).stdout

def main():
    direct = [json.loads(line) for line in run(SSH + [
        "podman ps -a --format '{\"name\":{{json .Names}},\"state\":{{json .State}}}'"
    ]).splitlines()]
    names = sorted(row['name'] for row in direct)
    # All extant containers are selected for comparison, including an exited init
    # container as a negative control. This does not declare it a required daemon.
    command = [str(ROOT / 'target/debug/arda-cli'), 'metrics', 'home-server',
               '--prometheus-url', 'http://100.103.125.88:9090', '--node', 'beelink',
               '--instance', '100.103.125.88:9100', '--ssh-target', 'beelink',
               '--expect-system-unit', 'sshd.service', '--expect-user-unit', 'llama-server-light.service',
               '--expect-user-unit', 'node_exporter.service']
    for name in names:
        command += ['--expect-container', name]
    report = json.loads(run(command))
    (HERE / 'home-inventory-measurements.json').write_text(json.dumps(report, indent=2) + '\n')
    checks = []
    def check(name, passed):
        checks.append({'check': name, 'passed': bool(passed)})
    check('no_actions', report['read_only'] and report['actions'] == [] and report['model_interpretation'] is None)
    for inventory in report['inventory']:
        check(inventory['scope'] + '_fresh', inventory['status'] == 'measured')
        if inventory['scope'] == 'podman_user':
            expected = {row['name']: row['state'] for row in direct}
            actual = {row['name']: row['observed']['state'] for row in inventory['checks']}
            check('all_container_states_equal_independent_read', expected == actual)
        else:
            for item in inventory['checks']:
                raw = run(SSH + [f"systemctl --{inventory['scope']} show {item['name']} --property=LoadState,ActiveState,SubState"])
                fields = dict(line.split('=', 1) for line in raw.splitlines())
                expected = fields == {'LoadState': 'loaded', 'ActiveState': 'active', 'SubState': 'running'}
                check(item['name'] + '_independent_show', (item['liveness'] == 'running') == expected)
    result = {'read_only': True, 'installed_policy_changed': False,
              'selected_container_count': len(names), 'checks': checks,
              'negative_controls': ['minio-init is an exited initializer, not a required running daemon',
                                    'user node_exporter.service is inactive; node-exporter container runs'],
              'passed': all(item['passed'] for item in checks),
              'limitations': 'Snapshot liveness only; no application, backup or sustained-contention acceptance.'}
    (HERE / 'home-inventory-qualification.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))
    if not result['passed']:
        raise SystemExit(1)

if __name__ == '__main__':
    main()
