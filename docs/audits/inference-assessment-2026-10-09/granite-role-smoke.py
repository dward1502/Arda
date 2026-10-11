"""Server-only bounded CPU specialist smoke; never stops incumbents."""
import pathlib,hashlib,json,subprocess,time,urllib.request,datetime
H=pathlib.Path.home()
assert H.name=='annunimas-server'
D=H/'inference-assessment/granite42'; model=D/'granite-4.2-3b-Q4_K_M.gguf.part'
bindir=H/'inference-assessment/b11430-cuda/llama-b11430';binary=bindir/'llama-server'
sha=hashlib.file_digest(model.open('rb'),'sha256').hexdigest()
assert sha=='e0406663965846ae22a403456eb826ccce5f450840491f71952f18a7cb78e7d5'
unit='inference-assessment-granite-role'
out=D/'cpu-role-result.json'
result={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'direct CPU specialist smoke, not routed or placement qualification','model_sha256':sha,'binary_sha256':hashlib.file_digest(binary.open('rb'),'sha256').hexdigest(),'cases':[]}
def cmd(*args):return subprocess.run(args,text=True,capture_output=True,timeout=30)
def api(path,payload=None):
 req=urllib.request.Request('http://127.0.0.1:18129'+path,data=None if payload is None else json.dumps(payload).encode(),headers={'Content-Type':'application/json'})
 with urllib.request.urlopen(req,timeout=45) as r:return json.load(r)
fixtures=[('extract','Return only JSON with keys service and state. Record: service=backup; state=failed. No other facts are available.',{'service':'backup','state':'failed'}),('classify','Reply exactly ATTENTION or HEALTHY. Backup job failed; the web service is healthy. Does the backup job require attention?','ATTENTION'),('source-summary','Use only these sources. [A] January: port 7000. [B] February correction: port 7001 replaces 7000. Reply exactly 7001 [B].','7001 [B]'),('abstain','The supplied record gives service name backup but no execution time. Return exactly UNKNOWN for its execution time.','UNKNOWN')]
try:
 result['baseline']={str(p):json.load(urllib.request.urlopen(f'http://127.0.0.1:{p}/health',timeout=5)) for p in [8097,8098]}
 args=['systemd-run','--user','--unit='+unit,'--property=RuntimeMaxSec=240','--property=MemoryMax=8G','--property=MemorySwapMax=0','--property=CPUQuota=400%','--setenv=CUDA_VISIBLE_DEVICES=','--setenv=LD_LIBRARY_PATH='+str(bindir)+':'+str(H/'inference-assessment/b11430-cuda/cudart-llama-b11430-bin-ubuntu-cuda-12.8-x64'),str(binary),'-m',str(model),'--alias','granite-specialist','--host','127.0.0.1','--port','18129','-c','4096','-np','1','-ngl','0','-t','4','--jinja']
 result['launch']=args;p=cmd(*args);result['start']={'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr};assert p.returncode==0
 deadline=time.monotonic()+50
 while True:
  try:result['health']=api('/health');break
  except Exception:
   if time.monotonic()>deadline:raise
   time.sleep(.3)
 props=api('/props');result['props']=props
 for name,prompt,expected in fixtures:
  row={'name':name,'prompt':prompt,'expected':expected};result['cases'].append(row);start=time.monotonic()
  try:
   response=api('/v1/chat/completions',{'model':'granite-specialist','stream':False,'temperature':0,'max_tokens':128,'chat_template_kwargs':{'enable_thinking':False},'messages':[{'role':'user','content':prompt}]});row['response']=response
   text=response['choices'][0]['message'].get('content','').strip();row['passed']=(json.loads(text)==expected) if isinstance(expected,dict) else text==expected
  except Exception as e:row.update(passed=False,error=str(e))
  row['elapsed_s']=time.monotonic()-start;out.write_text(json.dumps(result,indent=2))
except Exception as e:result['error']=str(e)
finally:
 result['resources']=cmd('systemctl','--user','show',unit,'-p','MemoryPeak','-p','CPUUsageNSec').stdout
 result['stop_exit']=cmd('systemctl','--user','stop',unit).returncode
 result['state_after']=cmd('systemctl','--user','is-active',unit).stdout.strip()
 result['log']=cmd('journalctl','--user','-u',unit,'-n','60','--no-pager').stdout
 result['post_health']={}
 for p in [8097,8098]:
  try:result['post_health'][str(p)]=json.load(urllib.request.urlopen(f'http://127.0.0.1:{p}/health',timeout=5))
  except Exception as e:result['post_health'][str(p)]={'error':str(e)}
 result['passed']=len(result['cases'])==4 and all(r.get('passed') for r in result['cases']) and result['state_after']=='inactive' and all(v.get('status')=='ok' for v in result['post_health'].values())
 out.write_text(json.dumps(result,indent=2));print(json.dumps({'passed':result['passed'],'error':result.get('error'),'cases':[{k:r.get(k) for k in ['name','passed','elapsed_s','error']} for r in result['cases']],'state_after':result['state_after'],'post_health':result['post_health']}))
raise SystemExit(0 if result['passed'] else 1)
