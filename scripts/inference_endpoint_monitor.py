"""Bounded deterministic endpoint observations; no inference or remediation.
Installed beside the Hermes cron entrypoint; state is outside the repository.
"""
import argparse
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shlex
import signal
import selectors
import subprocess
import time
import uuid

SERVICES=('prometheus','grafana','minio','server_nanbeige','server_bonsai')
MAX_BYTES=65536
SSH=['ssh','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=5','-o','ServerAliveInterval=5','-o','ServerAliveCountMax=1','annunimas-server@100.102.250.115']
SCOPE='Endpoint self-reports only; application workload health, backup recovery and model quality remain unknown. No remediation.'
REMOTE=r'''
import datetime,json,urllib.request,urllib.error
class NoRedirect(urllib.request.HTTPRedirectHandler):
 def redirect_request(self,*a,**k):return None
opener=urllib.request.build_opener(NoRedirect)
rows=[]
for name,url in [('prometheus','http://100.103.125.88:9090/-/ready'),('grafana','http://100.103.125.88:3000/api/health'),('minio','http://100.103.125.88:9000/minio/health/ready'),('server_nanbeige','http://127.0.0.1:8098/health'),('server_bonsai','http://127.0.0.1:8097/health')]:
 row={'service':name,'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'http_status':None,'status':'unavailable'}
 try:
  with opener.open(url,timeout=3) as r:
   row.update(http_status=r.status,status='endpoint_responding' if r.status==200 else 'unexpected_status',bytes_read=len(r.read(4096)))
 except urllib.error.HTTPError as e:row.update(http_status=e.code,status='unexpected_status')
 except Exception as e:row['error']=type(e).__name__
 rows.append(row)
 print(json.dumps({'check':row}),flush=True)
import subprocess,sys
props=subprocess.check_output(['systemctl','--user','show',sys.argv[1],'-pCPUQuotaPerSecUSec','-pMemoryMax','-pMemorySwapMax','-pTasksMax','-pTimeoutStartUSec','-pTimeoutStopUSec','-pMemoryPeak','-pCPUUsageNSec'],text=True,timeout=3)
print(json.dumps({'checks':rows,'properties':dict(x.split('=',1) for x in props.splitlines() if '=' in x)}))
'''

def now():return dt.datetime.now(dt.timezone.utc).isoformat()

def decode(text):
    def unique(pairs):
        out={}
        for k,v in pairs:
            if k in out:raise ValueError('duplicate JSON key')
            out[k]=v
        return out
    return json.loads(text,object_pairs_hook=unique)

def freshness(record,max_age):
    try:
        age=(dt.datetime.now(dt.timezone.utc)-dt.datetime.fromisoformat(record['finished_at'])).total_seconds()
        return {'age_s':age,'stale':age<0 or age>max_age}
    except (KeyError,TypeError,ValueError):return {'age_s':None,'stale':True}

def classify(rows):
    complete=(isinstance(rows,list) and all(isinstance(x,dict) for x in rows) and [x.get('service') for x in rows]==list(SERVICES))
    complete=complete and all(not freshness({'finished_at':x.get('observed_at')},120)['stale'] for x in rows)
    good=complete and all(x.get('status')=='endpoint_responding' and x.get('http_status')==200 for x in rows)
    return {'status':'endpoints_responding' if good else 'degraded' if complete else 'collection_failed','application_workload_health':None,'backup_recoverability':None,'scope':SCOPE}

def command(args,timeout):
    start=time.monotonic();buffers={'stdout':bytearray(),'stderr':bytearray()}
    p=subprocess.Popen(args,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
    timed_out=False;overflow=False
    try:
        with selectors.DefaultSelector() as selector:
            for name,pipe in [('stdout',p.stdout),('stderr',p.stderr)]:selector.register(pipe,selectors.EVENT_READ,name)
            while selector.get_map():
                remaining=timeout-(time.monotonic()-start)
                if remaining<=0:timed_out=True;break
                for key,_ in selector.select(min(remaining,0.1)):
                    chunk=os.read(key.fd,8192)
                    if not chunk:selector.unregister(key.fileobj);continue
                    buf=buffers[key.data];available=MAX_BYTES-len(buf)
                    buf.extend(chunk[:available])
                    if len(chunk)>available:overflow=True;break
                if overflow:break
            if not (timed_out or overflow):
                try:p.wait(timeout=max(0.01,timeout-(time.monotonic()-start)))
                except subprocess.TimeoutExpired:timed_out=True
    finally:
        # Also runs for interrupted callers; descendants share this group.
        try:os.killpg(p.pid,signal.SIGKILL)
        except ProcessLookupError:pass
        p.wait();p.stdout.close();p.stderr.close()
    return {'exit_code':p.returncode,'timed_out':timed_out,'elapsed_s':time.monotonic()-start,'output_truncated':overflow,**{k:v.decode(errors='replace') for k,v in buffers.items()}}

def atomic(path,data):
    temp=path.with_name(path.name+'.'+uuid.uuid4().hex+'.tmp')
    with temp.open('w') as f:
        json.dump(data,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
    os.replace(temp,path)

def collect():
    unit='arda-endpoint-monitor-'+uuid.uuid4().hex[:12]+'.service'
    r={'unit':unit,'checks':[],'cleanup':{}}
    args=['systemd-run','--user','--unit='+unit,'--wait','--pipe','-p','Type=oneshot','-p','CPUQuota=25%','-p','MemoryMax=128M','-p','MemorySwapMax=0','-p','TasksMax=16','-p','TimeoutStartSec=25s','-p','TimeoutStopSec=5s','-p','NoNewPrivileges=yes','-p','CPUAccounting=yes','-p','MemoryAccounting=yes','/usr/bin/python3','-I','-c',REMOTE,unit]
    try:
        r['execution']=command(SSH+[shlex.join(args)],38)
        run=r['execution']
        for line in run['stdout'].splitlines():
            try:record=decode(line)
            except ValueError:continue
            if 'check' in record:r['checks'].append(record['check'])
            if 'properties' in record:r['properties']=record['properties']
        if run['exit_code']!=0 or run['output_truncated'] or run['timed_out']:r['error']='collector execution incomplete'
        props=r.get('properties',{})
        expected={'CPUQuotaPerSecUSec':'250ms','MemoryMax':'134217728','MemorySwapMax':'0','TasksMax':'16','TimeoutStartUSec':'25s','TimeoutStopUSec':'5s'}
        r['bounds_verified']=all(props.get(k)==v for k,v in expected.items())
    except Exception as e:r['error']=type(e).__name__+': '+str(e)
    finally:
        for name,cmd in [('stop','systemctl --user stop '+unit),('readback','systemctl --user show '+unit+' -p ActiveState -p MainPID')]:
            try:r['cleanup'][name]=command(SSH+[cmd],8)
            except Exception as e:r['cleanup'][name]={'error':type(e).__name__}
    r.update(classify(r['checks']))
    cleanup=r['cleanup'].get('readback',{})
    r['cleanup_verified']=cleanup.get('exit_code')==0 and 'MainPID=0' in cleanup.get('stdout','') and 'ActiveState=inactive' in cleanup.get('stdout','')
    if not r['cleanup_verified'] or not r.get('bounds_verified') or r.get('error'):r['status']='collection_failed'
    return r

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--state-dir',type=Path,default=Path.home()/'.local/state/arda/inference-monitor');parser.add_argument('--status',action='store_true');a=parser.parse_args()
    state=a.state_dir;state.mkdir(parents=True,exist_ok=True,mode=0o700)
    if a.status:
        try:latest=decode((state/'latest.json').read_text())
        except (OSError,ValueError):latest={}
        current=freshness(latest,900)
        print(json.dumps({'latest':latest,'freshness_now':current}))
        return 1 if current['stale'] else 0
    lock=(state/'run.lock').open('a')
    try:fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:
        lock.close();print(json.dumps({'status':'overlap_skipped','observed_at':now()}));return 2
    try:return run_locked(state)
    finally:lock.close()

def run_locked(state):
    receipt={'schema_version':1,'run_id':uuid.uuid4().hex,'started_at':now(),'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'inference_requests':0,'inference_cost':0,'invocation':'scheduler provenance requires Hermes execution record'}
    interrupted=state/'interrupted';interrupted.mkdir(exist_ok=True)
    if (state/'inflight.json').exists():os.replace(state/'inflight.json',interrupted/(uuid.uuid4().hex+'.json'))
    for abandoned in state.glob('*.tmp'):abandoned.unlink()
    atomic(state/'inflight.json',receipt)
    try:receipt.update(collect())
    except Exception as e:receipt.update(status='collection_failed',error=type(e).__name__)
    receipt['finished_at']=now()
    if receipt['status']=='endpoints_responding':atomic(state/'last-success.json',receipt)
    previous=state/'last-success.json'
    try:
        prior=decode(previous.read_text()) if previous.exists() else {}
        fresh=freshness(prior,1800)
        receipt['last_success_at']=prior.get('finished_at')
    except (ValueError,TypeError,AttributeError,OSError):
        fresh={'age_s':None,'stale':True};receipt['last_success_at']=None;receipt['prior_success_error']='invalid or unreadable'
    receipt['last_success_age_s']=fresh['age_s']
    receipt['stale_last_success']=fresh['stale']
    receipt['freshness_evaluated_at']=now()
    runs=state/'runs';runs.mkdir(exist_ok=True)
    atomic(runs/(receipt['run_id']+'.json'),receipt);atomic(state/'latest.json',receipt)
    (state/'inflight.json').unlink(missing_ok=True)
    for p in sorted(runs.glob('*.json'),key=lambda p:p.stat().st_mtime,reverse=True)[288:]:p.unlink()
    print(json.dumps({k:receipt.get(k) for k in ('run_id','status','started_at','finished_at','checks','scope','last_success_at','stale_last_success','cleanup_verified')}))
    for p in sorted(interrupted.glob('*.json'),key=lambda p:p.stat().st_mtime,reverse=True)[288:]:p.unlink()
    return 0 if receipt['status']=='endpoints_responding' else 1

if __name__=='__main__':raise SystemExit(main())
