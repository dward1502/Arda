"""Isolated routed specialist comparison; no production config edits."""
import os,json,time,pathlib,tempfile,socket,subprocess,urllib.request,urllib.error,hashlib,http.server,threading
B=pathlib.Path(__file__).parent
root=pathlib.Path(tempfile.mkdtemp(prefix='granite-routed-',dir='/var/home/mythos/.hermes/cache/scratch'))
remote='annunimas-server@100.102.250.115';unit='inference-assessment-granite-routed'
binary=pathlib.Path('/var/home/mythos/.local/state/arda/deployments/manwe-assessment-preservation/fresh-candidate')
result={'scope':'isolated candidate router, direct specialist comparison, not production promotion','binary_sha256':hashlib.file_digest(binary.open('rb'),'sha256').hexdigest(),'cases':[],'root':str(root)}
def ssh(command):return subprocess.run(['ssh','-o','BatchMode=yes','-o','ConnectTimeout=8',remote,command],capture_output=True,text=True,timeout=30)
def port():
 with socket.socket() as s:s.bind(('127.0.0.1',0));return s.getsockname()[1]
def get(url,payload=None):
 req=urllib.request.Request(url,data=None if payload is None else json.dumps(payload).encode(),headers={'Content-Type':'application/json'})
 try:
  with urllib.request.urlopen(req,timeout=40) as r:return {'status':r.status,'body':json.load(r),'headers':{k:v for k,v in r.headers.items() if k.lower().startswith('x-manwe')}}
 except urllib.error.HTTPError as e:return {'status':e.code,'error':e.read().decode()}
records=[];tunnel=None;proc=None;server=None
try:
 base='/var/home/annunimas-server/inference-assessment';model=base+'/granite42/granite-4.2-3b-Q4_K_M.gguf.part'
 verify=ssh('sha256sum '+model);result['remote_model_hash']=verify.stdout.strip();assert verify.returncode==0 and verify.stdout.split()[0]=='e0406663965846ae22a403456eb826ccce5f450840491f71952f18a7cb78e7d5'
 launch=f'systemd-run --user --unit={unit} -p RuntimeMaxSec=300 -p MemoryMax=8G -p MemorySwapMax=0 -p CPUQuota=400% --setenv=CUDA_VISIBLE_DEVICES= --setenv=LD_LIBRARY_PATH={base}/b11430-cuda/llama-b11430:{base}/b11430-cuda/cudart-llama-b11430-bin-ubuntu-cuda-12.8-x64 {base}/b11430-cuda/llama-b11430/llama-server -m {model} --alias granite-specialist --host 127.0.0.1 --port 18129 -c 4096 -np 1 -ngl 0 -t 4 --jinja'
 p=ssh(launch);result['launch']={'command':launch,'exit':p.returncode,'stderr':p.stderr};assert p.returncode==0
 tp=port();tunnel=subprocess.Popen(['ssh','-N','-o','BatchMode=yes','-o','ExitOnForwardFailure=yes','-L',f'127.0.0.1:{tp}:127.0.0.1:18129',remote],stdout=subprocess.DEVNULL,stderr=(root/'tunnel.log').open('w'))
 deadline=time.monotonic()+45
 while True:
  try:
   result['candidate_health']=get(f'http://127.0.0.1:{tp}/health');assert result['candidate_health']['status']==200;break
  except Exception:
   if time.monotonic()>deadline:raise
   time.sleep(.3)
 class Recorder(http.server.BaseHTTPRequestHandler):
  def log_message(self,*args):pass
  def do_POST(self):
   payload=json.loads(self.rfile.read(int(self.headers['Content-Length'])));row={'path':self.path,'request':payload};records.append(row)
   url=(f'http://127.0.0.1:{tp}' if self.path.startswith('/granite/') else 'http://100.102.250.115:8098')+'/v1/chat/completions'
   try:
    answer=get(url,payload);row['response']=answer;raw=json.dumps(answer.get('body',{'error':answer.get('error')})).encode();self.send_response(answer['status']);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(raw)
   except Exception as e:row['error']=str(e);self.send_error(502)
 server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Recorder);threading.Thread(target=server.serve_forever,daemon=True).start()
 models=[('edge_granite_specialist','granite-specialist',4096,'granite'),('edge_nanbeige_control','nanbeige4.2-3b-q4_k_m',32768,'control')]
 cfg=''
 for pid,mid,ctx,path in models:
  cfg+=f'''\n[[provider]]
id = "{pid}"
name = "Bounded role test"
base_url = "http://127.0.0.1:{server.server_port}/{path}/v1"
enabled = true
access_tier = "local"
quality_band = "medium"
requests_per_minute = 60
max_completion_tokens = 128
supports_tools = false
supports_structured_output = false
probe_model = "{mid}"
[[provider.model]]
id = "{mid}"
capable_tasks = ["chat", "background", "summary"]
context_window = {ctx}
is_default = true
capabilities = {{ tools = false, structured_output = false, streaming = false }}
'''
 (root/'providers.toml').write_text(cfg);rp=port();env={'PATH':os.environ['PATH'],'HOME':str(root),'ARDA_ROOT':str(root),'ARDA_HOME':str(root),'ARDA_MANWE_STATE_DIR':str(root/'state'),'ARDA_MANWE_PROVIDER_CONFIG':str(root/'providers.toml'),'ARDA_FLEET_BOOTSTRAP_STATE':str(root/'absent'),'ARDA_PROVIDER_INTELLIGENCE_PATH':str(root/'absent-intel')}
 proc=subprocess.Popen([str(binary),'--bind','127.0.0.1','--port',str(rp),'--config',str(root/'absent.toml')],env=env,cwd=root,stdout=(root/'router.log').open('w'),stderr=subprocess.STDOUT)
 deadline=time.monotonic()+20
 while True:
  try:result['router_health']=get(f'http://127.0.0.1:{rp}/healthz');break
  except Exception:
   if time.monotonic()>deadline:raise
   time.sleep(.2)
 fixtures=[('extract-missing','Return only a JSON object with exactly keys service, state, duration_seconds. Use null for missing information. Record: service=nightly-backup; state=failed. Duration is not recorded.',{'service':'nightly-backup','state':'failed','duration_seconds':None}),('classify-distractor','Return exactly HEALTHY or ATTENTION for the BACKUP job only. Web service is down. Backup completed successfully and verification passed.','HEALTHY')]
 for pid,mid,ctx,path in models:
  for label,prompt,expected in fixtures:
   payload={'model':pid+'/'+mid,'allow_forced_provider_fallback':False,'local_only':True,'workload_role':'background','context_window_target':4096,'max_tokens':128,'temperature':0,'stream':False,'chat_template_kwargs':{'enable_thinking':False},'messages':[{'role':'user','content':prompt}]}
   row={'provider':pid,'case':label,'request':payload,'expected':expected};result['cases'].append(row)
   row['admission']=get(f'http://127.0.0.1:{rp}/v1/chat/completions',dict(payload,dry_run=True))
   if row['admission']['status']!=200:row['passed']=False;continue
   start=time.monotonic();answer=get(f'http://127.0.0.1:{rp}/v1/chat/completions',payload);row['response']=answer;row['elapsed_s']=time.monotonic()-start
   try:
    text=answer['body']['choices'][0]['message']['content'].strip();row['passed']=(json.loads(text)==expected if isinstance(expected,dict) else text==expected) and answer['headers']['x-manwe-provider-id']==pid and answer['headers']['x-manwe-model-id']==mid
   except Exception as e:row.update(passed=False,error=str(e))
   (B/'granite-routed-role-result.json').write_text(json.dumps(result,indent=2))
except Exception as e:result['error']=str(e)
finally:
 result['cleanup_errors']=[]
 def cleanup(label,fn):
  try:return fn()
  except Exception as e:result['cleanup_errors'].append({'step':label,'error':str(e)});return None
 def stop_process(p):
  p.terminate()
  try:p.wait(timeout=5)
  except subprocess.TimeoutExpired:p.kill();p.wait(timeout=5)
 if proc:cleanup('router',lambda:stop_process(proc))
 if server:
  cleanup('recorder_shutdown',server.shutdown);cleanup('recorder_close',server.server_close)
 if tunnel:cleanup('tunnel',lambda:stop_process(tunnel))
 stopped=cleanup('candidate_stop',lambda:ssh('systemctl --user stop '+unit))
 state=cleanup('candidate_state',lambda:ssh('systemctl --user is-active '+unit))
 result['stop_exit']=None if stopped is None else stopped.returncode
 result['state_after']=None if state is None else state.stdout.strip()
 result['incumbent_health']={str(p):cleanup('health_'+str(p),lambda p=p:get(f'http://100.102.250.115:{p}/health')) for p in [8097,8098]};result['captures']=records
 def consistent(c,r):
  try:
   a=c['admission'];v=c['response'];h=a['headers']
   return a['status']==200 and a['body']['ok'] is True and h['x-manwe-provider-id']==c['provider'] and h['x-manwe-model-id']==v['headers']['x-manwe-model-id'] and a['body']['request']==r['request'] and all(r['response']['body'].get(k)==v['body'].get(k) for k in ['id','model','choices','usage']) and v['body']['_manwe_route']['provider_id']==c['provider'] and r['response']['status']==200
  except (KeyError,TypeError):return False
 result['capture_consistency']=len(records)==len(result['cases'])==4 and all(consistent(c,r) for c,r in zip(result['cases'],records))
 result['passed']=not result.get('error') and not result['cleanup_errors'] and result['capture_consistency'] and all(c.get('passed') for c in result['cases']) and result['stop_exit']==0 and result['state_after']=='inactive' and all(h and h['status']==200 and h['body'].get('status')=='ok' for h in result['incumbent_health'].values())
 (B/'granite-routed-role-result.json').write_text(json.dumps(result,indent=2));print(json.dumps({'passed':result['passed'],'error':result.get('error'),'cases':[{k:c.get(k) for k in ['provider','case','passed','elapsed_s','error']} for c in result['cases']],'state_after':result['state_after']}))
raise SystemExit(0 if result['passed'] else 1)
