import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('bounded', Path(__file__).with_name('bounded-server-check.py'))
assert spec is not None and spec.loader is not None
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

class VerdictTests(unittest.TestCase):
    def run_case(self, rows, cleanup_code=0):
        props = {'CPUQuotaPerSecUSec':'250ms','MemoryMax':'134217728','MemorySwapMax':'0','TasksMax':'16','TimeoutStartUSec':'25s','RuntimeMaxUSec':'30s','TimeoutStopUSec':'5s'}
        def result(code=0, text=''):
            return subprocess.CompletedProcess([],code,text,'')
        replies=[result(text=json.dumps({'checks':rows,'properties':props})),result(),result(),result(cleanup_code,'MainPID=0\nActiveState=inactive\n')]
        with tempfile.TemporaryDirectory() as directory, patch.object(m,'HERE',Path(directory)), patch.object(m,'desktop',return_value='MainPID=0\nActiveState=inactive'), patch.object(m,'ssh',side_effect=replies), patch('builtins.print'):
            return m.main()
    def rows(self):
        return [{'service':name,'url':url,'http_status':200,'status':'endpoint_responding'} for name,url in [
          ('prometheus','http://100.103.125.88:9090/-/ready'),('grafana','http://100.103.125.88:3000/api/health'),('minio','http://100.103.125.88:9000/minio/health/ready'),('server_nanbeige','http://127.0.0.1:8098/health'),('server_bonsai','http://127.0.0.1:8097/health')]]
    def test_valid(self): self.assertEqual(self.run_case(self.rows()),0)
    def test_missing(self): self.assertEqual(self.run_case([]),1)
    def test_failed(self):
        rows=self.rows(); rows[0]['http_status']=503
        self.assertEqual(self.run_case(rows),1)
    def test_duplicate(self):
        rows=self.rows(); rows[0]=rows[1]
        self.assertEqual(self.run_case(rows),1)
    def test_bad_cleanup_read(self): self.assertEqual(self.run_case(self.rows(),1),1)

if __name__=='__main__': unittest.main()
