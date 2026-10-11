import hashlib,json,os,socket,subprocess,tempfile,time,urllib.request
from pathlib import Path
repo=Path('/var/home/mythos/Eregion/Arda')
out=repo/'docs/audits/inference-assessment-2026-10-09/p4-hermes-candidate-tool-loop.json'
root=Path(tempfile.mkdtemp(prefix='manwe-tool-candidate-',dir='/var/home/mythos/.hermes/cache/scratch'))
source=(repo/'config/manwe.providers.toml').read_text()
block=next(b for b in source.split('[[provider]]') if '\nid = "edge_core"\n' in b)
(root/'providers.toml').write_text('[[provider]]'+block)
with socket.socket() as sock:
 sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
binary=repo/'target/debug/manwe'
env={'PATH':os.environ['PATH'],'HOME':str(root),'ARDA_ROOT':str(root),'ARDA_HOME':str(root),'ARDA_MANWE_STATE_DIR':str(root/'state'),'ARDA_MANWE_PROVIDER_CONFIG':str(root/'providers.toml'),'ARDA_FLEET_BOOTSTRAP_STATE':str(root/'absent-bootstrap.json'),'ARDA_PROVIDER_INTELLIGENCE_PATH':str(root/'absent-intelligence.json')}
result={'scope':'isolated debug candidate, not deployed router or Hermes harness','binary_sha256':hashlib.file_digest(binary.open('rb'),'sha256').hexdigest(),'state_root':str(root),'port':port,'steps':[]}
def request(path,payload=None):
 req=urllib.request.Request(f'http://127.0.0.1:{port}'+path,data=None if payload is None else json.dumps(payload).encode(),headers={'Content-Type':'application/json'})
 with urllib.request.urlopen(req,timeout=120) as response:
  return {'status':response.status,'headers':{k:v for k,v in response.headers.items() if k.lower().startswith('x-manwe-')},'body':json.load(response)}
with (root/'process.log').open('w') as log:
 process=subprocess.Popen([str(binary),'--bind','127.0.0.1','--port',str(port),'--config',str(root/'missing.toml')],env=env,cwd=root,stdout=log,stderr=subprocess.STDOUT)
 try:
  deadline=time.monotonic()+20
  while True:
   if process.poll() is not None: raise RuntimeError('candidate startup failed; inspect process.log')
   try:
    result['health']=request('/healthz');break
   except OSError:
    if time.monotonic()>deadline: raise
    time.sleep(.2)
  tools=[{'type':'function','function':{'name':'read_file','description':'Read a UTF-8 file; offset is a one-based line number.','parameters':{'type':'object','properties':{'path':{'type':'string'},'offset':{'type':'integer'}},'required':['path','offset']}}}]
  payload={'model':'auto','local_only':True,'force_provider_id':'edge_core','allow_forced_provider_fallback':False,'temperature':0,'max_tokens':256,'stream':False,'chat_template_kwargs':{'enable_thinking':False},'tools':tools,'tool_choice':'auto','messages':[{'role':'user','content':'Call read_file for Cargo.toml, offset 17. Then summarize the returned content in one sentence. Do not invent file contents.'}]}
  first=request('/v1/chat/completions',payload);result['steps'].append({'request':payload.copy(),'response':first})
  message=first['body']['choices'][0]['message'];calls=message.get('tool_calls',[])
  assert len(calls)==1, f'Expected one tool call, got {message}'
  call=calls[0];args=json.loads(call['function']['arguments'])
  assert call['function']['name']=='read_file' and args=={'path':'Cargo.toml','offset':17}, args
  lines=(repo/'Cargo.toml').read_text().splitlines();content='\n'.join(lines[16:22])
  payload['messages']=payload['messages']+[message,{'role':'tool','tool_call_id':call['id'],'content':content}]
  final=request('/v1/chat/completions',payload);result['steps'].append({'request':payload.copy(),'response':final})
  answer=final['body']['choices'][0]['message'];assert answer.get('content') and not answer.get('tool_calls'),answer
  import shutil
  hermes_home=root/'hermes';hermes_home.mkdir()
  fixture=root/'fixture.txt';fixture.write_text('The service owner is the otter. The approved port is 43219.\n')
  hermes_env=dict(env,HERMES_HOME=str(hermes_home),CUSTOM_BASE_URL=f'http://127.0.0.1:{port}/v1',OPENAI_API_KEY='local-fixture-no-secret')
  command=['/var/home/mythos/.hermes/installs/93d02a9d25b9a6b1/environments/cc6a35ab872f408d8744bd78c6cd77ab/venv/bin/python','-m','hermes_cli.main','chat','--safe-mode','--provider','custom','-m','edge_core/Qwen3.5-9B-Q4_K_M','-t','file','--max-turns','4','--run-budget','120','--oneshot','--format','stream-json','--in',str(root),'-q',f'Use read_file to read {fixture}. Do not modify anything. Reply with only the service owner and approved port from that file.']
  child=subprocess.run(command,env=hermes_env,cwd=root,capture_output=True,text=True,timeout=150)
  result['hermes']={'command':command,'exit_code':child.returncode,'stdout':child.stdout,'stderr':child.stderr}
  assert child.returncode==0, 'Hermes CLI failed; inspect retained stdout/stderr'
  assert 'otter' in child.stdout and '43219' in child.stdout, 'Hermes output omitted expected fixture values'
  result['passed']=True
 except Exception as exc:
  result['passed']=False;result['error']=str(exc)
 finally:
  process.terminate()
  try: process.wait(timeout=10)
  except subprocess.TimeoutExpired: process.kill();process.wait(timeout=10)
  result['candidate_stopped']=process.poll() is not None
  out.write_text(json.dumps(result,indent=2))
print(json.dumps(result,indent=2))
raise SystemExit(0 if result.get('passed') else 1)
