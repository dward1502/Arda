import subprocess,json,concurrent.futures,pathlib,datetime
import argparse
p=argparse.ArgumentParser();p.add_argument('--out',type=pathlib.Path,required=True);OUT=p.parse_args().out
probe=r'''
import subprocess,pathlib,json,hashlib,urllib.request,datetime
out={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'commands':{},'processes':[],'apis':{}}
def run(args):
 try:
  r=subprocess.run(args,capture_output=True,text=True,timeout=15);return {'rc':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
 except Exception as e:return {'error':str(e)}
for args in [['hostname'],['lscpu'],['free','-b'],['df','-B1','/home'],['nvidia-smi','--query-gpu=name,memory.total,memory.used,driver_version','--format=csv'],['nvidia-smi','--query-compute-apps=pid,process_name,used_memory','--format=csv'],['ss','-ltnp']]:out['commands'][' '.join(args)]=run(args)
for unit in ['mesh-llm','llama-server','llama-server-light','llama-server-bonsai-q1','llama-server-nanbeige']:
 out['commands'][unit]=run(['systemctl','--user','show',unit+'.service','-p','MainPID','-p','ControlGroup','-p','ExecStart','-p','ActiveState','-p','SubState','-p','FragmentPath','-p','DropInPaths'])
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  comm=(p/'comm').read_text().strip()
  if not any(x in comm for x in ['llama','llmster']):continue
  exe=(p/'exe').resolve(); row={'pid':p.name,'comm':comm,'exe':str(exe),'cmdline':(p/'cmdline').read_bytes().replace(b'\0',b' ').decode(),'cgroup':(p/'cgroup').read_text()}
  row['exe_sha256']=hashlib.file_digest((p/'exe').open('rb'),'sha256').hexdigest()
  row['gguf_maps']=sorted(set(l.split()[-1] for l in (p/'maps').read_text().splitlines() if '.gguf' in l))
  out['processes'].append(row)
 except Exception as e:out['processes'].append({'pid':p.name,'error':str(e)})
for port in [9337,1234,8097,8098]:
 try:out['apis'][str(port)]=json.load(urllib.request.urlopen('http://127.0.0.1:%s/v1/models'%port,timeout=3))
 except Exception as e:out['apis'][str(port)]={'error':str(e)}
print(json.dumps(out))
'''
targets={'core':None,'server':'annunimas-server@100.102.250.115','beelink':'citadel@100.103.125.88','laptop':'mythos@100.97.15.88','warden':'numenor@100.110.85.37'}
from typing import Any
def collect(item) -> tuple[str, dict[str, Any]]:
 name,host=item
 cmd=['python3','-'] if host is None else ['ssh','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=6',host,'python3 -']
 try:
  r=subprocess.run(cmd,input=probe,text=True,capture_output=True,timeout=100)
  return name,{'rc':r.returncode,'stderr':r.stderr,'result':json.loads(r.stdout)}
 except Exception as e:return name,{'error':str(e)}
with concurrent.futures.ThreadPoolExecutor(max_workers=5) as ex: results=dict(ex.map(collect,targets.items()))
OUT.write_text(json.dumps(results,indent=2))
for name,r in results.items():
 print(name)
 d=r.get('result',{});print(json.dumps({'error':r.get('error'),'processes':d.get('processes'),'models':{k:[x['id'] for x in v.get('data',[])] if 'data' in v else v for k,v in d.get('apis',{}).items()}},indent=2))
print(OUT)
