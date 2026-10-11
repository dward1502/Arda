"""Offline contracts for the observation-only production monitor."""
import importlib.util
import contextlib
import io
import json
import os
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path
spec=importlib.util.spec_from_file_location('monitor',Path(__file__).with_name('inference_endpoint_monitor.py'))
m=importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

class MonitorTests(unittest.TestCase):
    def setUp(self):
        # All state stays in the caller's TMPDIR; never use live collectors.
        scratch = Path(os.environ.get('TMPDIR', str(Path.home()/'.hermes/cache/scratch')))
        scratch.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.real_collect = m.collect
        self.collector = self.enterContext(patch.object(
            m, 'collect', side_effect=AssertionError('unmocked collector')))

    def invoke(self):
        with patch.object(sys, 'argv', ['monitor', '--state-dir', str(self.root)]), \
                contextlib.redirect_stdout(io.StringIO()) as output:
            code = m.main()
        return code, json.loads(output.getvalue())

    def assert_lock_available(self):
        with (self.root / 'run.lock').open('a') as lock:
            m.fcntl.flock(lock, m.fcntl.LOCK_EX | m.fcntl.LOCK_NB)
            m.fcntl.flock(lock, m.fcntl.LOCK_UN)

    def test_lock_contention_then_release(self):
        self.collector.side_effect = None
        self.collector.return_value = {'status': 'degraded', 'checks': []}
        with (self.root / 'run.lock').open('a') as lock:
            m.fcntl.flock(lock, m.fcntl.LOCK_EX | m.fcntl.LOCK_NB)
            code, result = self.invoke()
            self.assertEqual(code, 2)
            self.assertEqual(result['status'], 'overlap_skipped')
            self.collector.assert_not_called()
            self.assertEqual({p.name for p in self.root.iterdir()}, {'run.lock'})
        self.assertEqual(self.invoke()[0], 1)
        self.collector.assert_called_once_with()
        self.assert_lock_available()

    def test_lock_released_when_run_raises(self):
        with patch.object(m, 'run_locked', side_effect=RuntimeError('injected')):
            with self.assertRaisesRegex(RuntimeError, 'injected'):
                self.invoke()
        self.collector.assert_not_called()
        self.assert_lock_available()

    def test_degraded_receipt_preserves_last_success(self):
        rows = [{'service': n, 'status': 'endpoint_responding',
                 'http_status': 200, 'observed_at': m.now()} for n in m.SERVICES]
        good = dict(m.classify(rows), checks=rows)
        bad_rows = [dict(row) for row in rows]
        bad_rows[0].update(status='unavailable', http_status=None)
        bad = dict(m.classify(bad_rows), checks=bad_rows)
        self.collector.side_effect = [good, bad]
        self.assertEqual(self.invoke()[0], 0)
        success_path = self.root / 'last-success.json'
        success_bytes = success_path.read_bytes()
        success = json.loads(success_bytes)
        code, summary = self.invoke()
        self.assertEqual(code, 1)
        latest = json.loads((self.root / 'latest.json').read_text())
        self.assertEqual(summary['run_id'], latest['run_id'])
        self.assertEqual(latest['status'], 'degraded')
        self.assertEqual(latest['checks'], bad_rows)
        self.assertNotEqual(latest['run_id'], success['run_id'])
        self.assertEqual(success_path.read_bytes(), success_bytes)
        self.assertEqual(latest['last_success_at'], success['finished_at'])
        self.assertFalse(latest['stale_last_success'])
        self.assertGreaterEqual(latest['last_success_age_s'], 0)
        archived = self.root / 'runs' / (latest['run_id'] + '.json')
        self.assertEqual(json.loads(archived.read_text()), latest)
        self.assertEqual(len(list((self.root / 'runs').glob('*.json'))), 2)
        self.assertFalse((self.root / 'inflight.json').exists())
        self.assertFalse(list(self.root.glob('*.tmp')))

    def test_retention_rollover_for_runs_and_interrupted(self):
        for name in ('runs', 'interrupted'):
            folder = self.root / name
            folder.mkdir()
            for index in range(288):
                path = folder / f'old-{index:03d}.json'
                path.write_text(json.dumps({'index': index}))
                os.utime(path, (1000 + index, 1000 + index))
        (self.root / 'inflight.json').write_text('{"run_id":"abandoned"}')
        (self.root / 'abandoned.tmp').write_text('partial')
        self.collector.side_effect = None
        self.collector.return_value = {'status': 'degraded', 'checks': []}
        self.assertEqual(self.invoke()[0], 1)
        latest = json.loads((self.root / 'latest.json').read_text())
        for name in ('runs', 'interrupted'):
            with self.subTest(folder=name):
                paths = list((self.root / name).glob('*.json'))
                names = {p.name for p in paths}
                self.assertEqual(len(paths), 288)
                self.assertNotIn('old-000.json', names)
                self.assertTrue({f'old-{i:03d}.json' for i in range(1, 288)} <= names)
                new = [p for p in paths if not p.name.startswith('old-')]
                self.assertEqual(len(new), 1)
                expected = latest if name == 'runs' else {'run_id': 'abandoned'}
                self.assertEqual(json.loads(new[0].read_text()), expected)
        self.assertFalse((self.root / 'inflight.json').exists())
        self.assertFalse(list(self.root.glob('*.tmp')))

    def test_command_interruption_reaps_child_and_closes_pipes(self):
        spawned = []
        real_popen = subprocess.Popen
        real_selector = m.selectors.DefaultSelector

        def spawn(*args, **kwargs):
            child = real_popen(*args, **kwargs)
            spawned.append(child)
            return child

        def interrupt_selector():
            selector = real_selector()
            selector.select = unittest.mock.Mock(side_effect=KeyboardInterrupt('injected'))
            return selector

        try:
            with patch.object(m.subprocess, 'Popen', side_effect=spawn), \
                    patch.object(m.selectors, 'DefaultSelector', side_effect=interrupt_selector), \
                    patch.object(m.os, 'killpg', wraps=os.killpg) as killpg:
                with self.assertRaisesRegex(KeyboardInterrupt, 'injected'):
                    m.command([sys.executable, '-c', 'import time; time.sleep(30)'], 5)
                self.assertEqual(len(spawned), 1)
                child = spawned[0]
                killpg.assert_called_once_with(child.pid, signal.SIGKILL)
                self.assertEqual(child.returncode, -signal.SIGKILL)
                self.assertTrue(child.stdout.closed)
                self.assertTrue(child.stderr.closed)
                with self.assertRaises(ChildProcessError):
                    os.waitpid(child.pid, os.WNOHANG)
        finally:
            # Also clean up if a future regression causes an assertion failure.
            for child in spawned:
                if child.poll() is None:
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                child.wait(timeout=5)
                child.stdout.close()
                child.stderr.close()

    def test_complete_is_not_workload_health(self):
        rows=[{'service':n,'status':'endpoint_responding','http_status':200,'observed_at':m.now()} for n in m.SERVICES]
        result=m.classify(rows)
        self.assertEqual(result['status'],'endpoints_responding')
        self.assertIsNone(result['application_workload_health'])
        self.assertIsNone(result['backup_recoverability'])
    def test_degraded_is_retained(self):
        rows=[{'service':n,'status':'unavailable','http_status':None,'observed_at':m.now()} for n in m.SERVICES]
        self.assertEqual(m.classify(rows)['status'],'degraded')
    def test_missing_and_duplicate_are_collection_failures(self):
        for rows in ([],[{'service':m.SERVICES[0]}]*len(m.SERVICES)):
            self.assertEqual(m.classify(rows)['status'],'collection_failed')
    def test_json_duplicate_rejected(self):
        with self.assertRaises(ValueError):m.decode('{"checks":[],"checks":[]}')
    def test_hard_deadline(self):
        import sys,time
        start=time.monotonic()
        r=m.command([sys.executable,'-c','import time;time.sleep(30)'],0.2)
        self.assertTrue(r['timed_out'])
        self.assertLess(time.monotonic()-start,3)
    def test_output_cap(self):
        import sys
        r=m.command([sys.executable,'-c','print("x"*100000)'],5)
        self.assertTrue(r['output_truncated'])
        self.assertLessEqual(len(r['stdout']),m.MAX_BYTES)

    def test_stale_observations_fail(self):
        rows=[{'service':n,'status':'endpoint_responding','http_status':200,'observed_at':'2000-01-01T00:00:00+00:00'} for n in m.SERVICES]
        self.assertEqual(m.classify(rows)['status'],'collection_failed')
    def test_partial_results_survive_property_failure(self):
        from unittest.mock import patch
        import json
        run={'exit_code':1,'stdout':json.dumps({'check':{'service':'prometheus'}})+'\n','timed_out':False,'output_truncated':False}
        cleanup={'exit_code':0,'stdout':'MainPID=0\nActiveState=inactive'}
        with patch.object(m,'command',side_effect=[run,cleanup,cleanup]):result=self.real_collect()
        self.assertEqual(result['checks'],[{'service':'prometheus'}])
        self.assertEqual(result['status'],'collection_failed')
    def test_corrupt_prior_success_and_interruption(self):
        from unittest.mock import patch
        import tempfile,json,sys
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);(root/'last-success.json').write_text('{bad');(root/'inflight.json').write_text('{"run_id":"prior"}')
            with patch.object(sys,'argv',['monitor','--state-dir',temp]),patch.object(m,'collect',return_value={'status':'degraded','checks':[]}):
                self.assertEqual(m.main(),1)
            receipt=json.loads((root/'latest.json').read_text())
            self.assertTrue(receipt['stale_last_success'])
            self.assertTrue(list((root/'interrupted').glob('*.json')))
    def test_missing_run_freshness(self):
        self.assertTrue(m.freshness({'finished_at':'2000-01-01T00:00:00+00:00'},300)['stale'])
        self.assertTrue(m.freshness({},300)['stale'])

if __name__=='__main__':unittest.main()
