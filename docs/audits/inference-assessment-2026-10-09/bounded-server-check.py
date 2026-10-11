"""One-shot deterministic service checks on Server in a capped transient unit.
No inference, remediation, timer installation, or production service changes.
"""
import datetime
import json
from pathlib import Path
import shlex
import subprocess
import uuid

HERE = Path(__file__).resolve().parent
SSH = ['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=8', 'annunimas-server@100.102.250.115']
REMOTE = r'''
import datetime,json,urllib.request,urllib.error
checks=[('prometheus','http://100.103.125.88:9090/-/ready'),
('grafana','http://100.103.125.88:3000/api/health'),
('minio','http://100.103.125.88:9000/minio/health/ready'),
('server_nanbeige','http://127.0.0.1:8098/health'),
('server_bonsai','http://127.0.0.1:8097/health')]
class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self,*args,**kwargs): return None
opener=urllib.request.build_opener(NoRedirect)
rows=[]
for name,url in checks:
    row={'service':name,'url':url,'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat()}
    try:
        with opener.open(url,timeout=3) as response:
            row.update(http_status=response.status,bytes_read=len(response.read(4096)),status='endpoint_responding' if response.status==200 else 'unexpected_status')
    except Exception as error: row.update(status='unavailable',error=type(error).__name__)
    rows.append(row)
print(json.dumps({'checks':rows,'scope':'Endpoint self-reports only; not application workload, backups, model quality or sustained health. No remediation.'}))
'''

def ssh(command):
    return subprocess.run(SSH+[command],capture_output=True,text=True,timeout=45)

def desktop():
    return subprocess.check_output(['systemctl','--user','show','mesh-llm.service','-p','ActiveState','-p','MainPID'],text=True,timeout=5).strip()

def main():
    unit='arda-assessment-check-'+uuid.uuid4().hex[:10]+'.service'
    result: dict = {'unit':unit,'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'desktop_before':desktop(),
            'scope':'One-shot manually invoked capped deterministic Server job; not an installed schedule or model qualification.'}
    assert 'MainPID=0' in result['desktop_before'] and 'ActiveState=inactive' in result['desktop_before']
    cmd=['systemd-run','--user','--unit='+unit,'--wait','--pipe',
         '-p','Type=oneshot','-p','RemainAfterExit=no','-p','CPUQuota=25%',
         '-p','MemoryMax=128M','-p','MemorySwapMax=0','-p','TasksMax=16',
         '-p','TimeoutStartSec=25s','-p','TimeoutStopSec=5s',
         '-p','NoNewPrivileges=yes','-p','CPUAccounting=yes','-p','MemoryAccounting=yes',
         '/usr/bin/python3','-I','-c',REMOTE.replace("print(json.dumps({'checks':rows,", "import subprocess\nprops=subprocess.check_output(['systemctl','--user','show',"+repr(unit)+",'-pCPUQuotaPerSecUSec','-pMemoryMax','-pMemorySwapMax','-pTasksMax','-pTimeoutStartUSec','-pTimeoutStopUSec','-pMemoryPeak','-pCPUUsageNSec'],text=True)\nprint(json.dumps({'properties':dict(line.split('=',1) for line in props.splitlines() if '=' in line),'checks':rows,") ]
    try:
        run=ssh(shlex.join(cmd))
        result.update(exit_code=run.returncode,stderr=run.stderr)
        if run.returncode==0: result['report']=json.loads(run.stdout)
        else: result['stdout']=run.stdout
        fields=['ActiveState','Result','ExecMainStatus','CPUQuotaPerSecUSec','MemoryMax','MemorySwapMax','TasksMax','TimeoutStartUSec','RuntimeMaxUSec','MemoryPeak','CPUUsageNSec']
        read=ssh('systemctl --user show '+shlex.quote(unit)+' '+shlex.join(['-p'+field for field in fields]))
        result['post_run_read']={'exit_code':read.returncode,'stdout':read.stdout,'stderr':read.stderr}
        result['unit_properties']=result.get('report',{}).get('properties',{})
        props=result['unit_properties']
        result['bounds_verified']=(read.returncode==0 and props.get('CPUQuotaPerSecUSec')=='250ms' and props.get('MemoryMax')=='134217728' and props.get('MemorySwapMax')=='0' and props.get('TasksMax')=='16' and props.get('TimeoutStartUSec')=='25s' and props.get('TimeoutStopUSec')=='5s')
    finally:
        stop=ssh('systemctl --user stop '+shlex.quote(unit))
        after=ssh('systemctl --user show '+shlex.quote(unit)+' -p ActiveState -p MainPID')
        result['cleanup']={'stop_exit':stop.returncode,'stop_stdout':stop.stdout,'stop_stderr':stop.stderr,'read_exit':after.returncode,'state':after.stdout.strip(),'read_stderr':after.stderr}
        result['desktop_after']=desktop()
        expected=[('prometheus','http://100.103.125.88:9090/-/ready'),('grafana','http://100.103.125.88:3000/api/health'),('minio','http://100.103.125.88:9000/minio/health/ready'),('server_nanbeige','http://127.0.0.1:8098/health'),('server_bonsai','http://127.0.0.1:8097/health')]
        rows=result.get('report',{}).get('checks',[])
        result['endpoints_verified']=([(r.get('service'),r.get('url')) for r in rows]==expected and all(r.get('http_status')==200 and r.get('status')=='endpoint_responding' for r in rows))
        result['passed']=result['endpoints_verified'] and after.returncode==0 and result.get('exit_code')==0 and result.get('bounds_verified',False) and 'MainPID=0' in after.stdout and 'ActiveState=inactive' in after.stdout and result['desktop_after']==result['desktop_before']
        (HERE/'bounded-server-check.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
    return 0 if result['passed'] else 1

if __name__=='__main__':
    raise SystemExit(main())
