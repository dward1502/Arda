import pathlib,subprocess,os,tempfile,socket,time,json,urllib.request,hashlib
repo=pathlib.Path('/var/home/mythos/Eregion/Arda');b=repo/'docs/audits/inference-assessment-2026-10-09'
root=pathlib.Path(tempfile.mkdtemp(prefix='negation-router-',dir=os.environ['TMPDIR']))
(root/'providers.toml').write_text('''[[provider]]
id="edge_backbone_coder"
name="Isolated existing Nanbeige"
base_url="http://100.102.250.115:8098/v1"
enabled=true
access_tier="local"
quality_band="medium"
requests_per_minute=60
max_completion_tokens=512
supports_tools=true
probe_model="nanbeige4.2-3b-q4_k_m"
[[provider.model]]
id="nanbeige4.2-3b-q4_k_m"
is_default=true
context_window=32768
capable_tasks=["chat","summary","background","code"]
capabilities={tools=true,structured_output=true,streaming=true,visible_reasoning=false}
''')
with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
env={'PATH':os.environ['PATH'],'HOME':str(root),'ARDA_ROOT':str(root),'ARDA_HOME':str(root),'ARDA_MANWE_STATE_DIR':str(root/'state'),'ARDA_MANWE_PROVIDER_CONFIG':str(root/'providers.toml'),'ARDA_FLEET_BOOTSTRAP_STATE':str(root/'absent'),'ARDA_PROVIDER_INTELLIGENCE_PATH':str(root/'absent')}
binary=pathlib.Path(os.environ.get('MANWE_CANDIDATE',str(repo/'target/debug/manwe')));result={'binary_sha256':hashlib.file_digest(binary.open('rb'),'sha256').hexdigest(),'source':'crates/spine/runtime/manwe/src/adaptive/transport/http.rs','checks':[],'scope':'isolated rebuilt router, no production replacement'}
p=subprocess.Popen([str(binary),'--bind','127.0.0.1','--port',str(port),'--config',str(root/'absent.toml')],env=env,cwd=root,stdout=(root/'log').open('w'),stderr=subprocess.STDOUT)
try:
 for _ in range(80):
  try:urllib.request.urlopen(f'http://127.0.0.1:{port}/healthz',timeout=1);break
  except Exception:time.sleep(.1)
 retained=json.loads((b/'home-mixed-workload-result.json').read_text());payload=retained['runs'][0]['request']
 payload['messages'][-1]['content']=payload['messages'][-1]['content'].replace('Do not change anything.','Do not make changes.')
 for dry in [True,False]:
  body=dict(payload,dry_run=True) if dry else payload
  req=urllib.request.Request(f'http://127.0.0.1:{port}/v1/chat/completions',data=json.dumps(body).encode(),headers={'Content-Type':'application/json'})
  t=time.monotonic()
  with urllib.request.urlopen(req,timeout=90) as r:result['checks'].append({'dry_run':dry,'request':body,'status':r.status,'headers':{k:v for k,v in r.headers.items() if k.lower().startswith('x-manwe')},'response':json.load(r),'elapsed_s':time.monotonic()-t})
except Exception as e:result['error']=str(e)
finally:
 p.terminate()
 try:p.wait(timeout=5)
 except subprocess.TimeoutExpired:p.kill();p.wait()
 result['process_exit']=p.returncode
 with socket.socket() as s:result['listener_absent']=s.connect_ex(('127.0.0.1',port))!=0
 result['passed']=not result.get('error') and len(result['checks'])==2 and result['listener_absent'] and all(c['headers'].get('x-manwe-provider-id')=='edge_backbone_coder' and c['headers'].get('x-manwe-execution-lane')=='background' for c in result['checks'])
 (b/os.environ.get('MANWE_RESULT','home-negation-fixed-router-result.json')).write_text(json.dumps(result,indent=2))
 print(json.dumps({'passed':result['passed'],'error':result.get('error'),'listener_absent':result['listener_absent'],'checks':[(c['dry_run'],c['status'],c['elapsed_s']) for c in result['checks']]}))
raise SystemExit(0 if result['passed'] else 1)
