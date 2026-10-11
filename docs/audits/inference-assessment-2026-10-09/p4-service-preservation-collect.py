"""Read-only process identity collection; never reads credential environments."""
import concurrent.futures,datetime,json,pathlib,subprocess
BASE=pathlib.Path(__file__).parent
REMOTE=r'''
import pathlib,json,hashlib,os,datetime,subprocess
allowed={'-m','--model','-c','--ctx-size','-np','--parallel','--port','--host','-ngl','--n-gpu-layers','--chat-template-file','--chat-template','--alias','-a','--flash-attn','-fa','--cache-type-k','--cache-type-v','--threads','-t','--threads-batch','-tb','--batch-size','-b','--ubatch-size','-ub','--reasoning-format'}
standalone={'--jinja','--no-warmup','--no-mmap','--mlock','--no-context-shift','--cont-batching'}
def digest(path):
 h=hashlib.sha256()
 with open(path,'rb') as f:
  for block in iter(lambda:f.read(4*1024*1024),b''):h.update(block)
 return h.hexdigest()
rows=[]
for proc in pathlib.Path('/proc').iterdir():
 if not proc.name.isdigit():continue
 try:
  exe=os.readlink(proc/'exe')
  if 'llama-server' not in pathlib.Path(exe).name:continue
  args=(proc/'cmdline').read_bytes().decode().split('\0');safe=[];files=[]
  for i,arg in enumerate(args):
   if arg in allowed and i+1<len(args):
    val=args[i+1];safe.extend([arg,val])
    if arg in ['-m','--model','--chat-template-file']:files.append(val)
   elif arg in standalone:safe.append(arg)
  row={'pid':int(proc.name),'executable':exe,'executable_sha256':digest(proc/'exe'),'selected_arguments':safe,'cgroup':(proc/'cgroup').read_text(),'files':[]}
  for value in files:
   path=pathlib.Path(value)
   if not path.is_absolute():path=pathlib.Path(os.readlink(proc/'cwd'))/path
   try:row['files'].append({'path':str(path),'bytes':path.stat().st_size,'sha256':digest(path)})
   except Exception as e:row['files'].append({'path':str(path),'error':str(e)})
  units=[part for part in row['cgroup'].replace('\n','/').split('/') if part.endswith('.service')]
  row['service_units']=[]
  row['environment_names']=sorted(x.split(b'=',1)[0].decode(errors='replace') for x in (proc/'environ').read_bytes().split(b'\0') if b'=' in x)
  for unit in units:
   cmd=['systemctl']+(['--user'] if '/user.slice/' in row['cgroup'] else [])+['show',unit,'-p','Id','-p','FragmentPath','-p','DropInPaths','-p','EnvironmentFiles','-p','ActiveState','-p','MainPID']
   check=subprocess.run(cmd,capture_output=True,text=True,timeout=10)
   properties=dict(line.split('=',1) for line in check.stdout.splitlines() if '=' in line)
   entry={'properties':properties,'exit':check.returncode,'source_files':[]}
   for name in [properties.get('FragmentPath','')]+properties.get('DropInPaths','').split():
    if name and pathlib.Path(name).is_file():entry['source_files'].append({'path':name,'sha256':digest(name)})
   row['service_units'].append(entry)
  rows.append(row)
 except (PermissionError,FileNotFoundError,ProcessLookupError):continue
print(json.dumps({'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'processes':rows}))
'''
def collect(item):
 node,target=item
 try:
  command=['python3','-c',REMOTE] if node=='Core' and subprocess.check_output(['hostname'],text=True).strip()=='annunimas-core' else ['ssh','-o','BatchMode=yes','-o','ConnectTimeout=8',target,'python3 -']
  p=subprocess.run(command,input=REMOTE,text=True,capture_output=True,timeout=180)
  payload=next((json.loads(line) for line in reversed(p.stdout.splitlines()) if line.startswith('{')),None)
  return {'node':node,'ssh_exit':p.returncode,'data':payload,'stderr':p.stderr,'parse_error':None if payload else 'No JSON payload; SSH or remote interpreter failed'}
 except Exception as e:return {'node':node,'error':str(e)}
nodes=[('Core','mythos@100.78.138.113'),('Server','annunimas-server@100.102.250.115'),('Beelink','citadel@100.103.125.88'),('Warden','numenor@100.110.85.37')]
rows=list(concurrent.futures.ThreadPoolExecutor(max_workers=4).map(collect,nodes))
(BASE/'p4-service-preservation.json').write_text(json.dumps({'scope':'read-only executable/model/template file hashes and selected nonsecret arguments; unit source locations/hashes and environment variable names only; values and full ExecStart withheld; not a complete portable rollback package','nodes':rows},indent=2))
for row in rows:print(row['node'],row.get('error') or [(p['pid'],p['executable'],len(p['files'])) for p in row['data']['processes']])
