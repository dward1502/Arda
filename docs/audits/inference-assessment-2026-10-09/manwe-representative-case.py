import hashlib,json,os,socket,subprocess,tempfile,time,urllib.request
from pathlib import Path
repo=Path('/var/home/mythos/Eregion/Arda')
out=repo/('docs/audits/inference-assessment-2026-10-09/p4-representative-'+os.environ['ASSESS_CASE']+'-'+os.environ['ASSESS_REPEAT']+'.json')
root=Path(tempfile.mkdtemp(prefix='manwe-tool-candidate-',dir='/var/home/mythos/.hermes/cache/scratch'))
source=(repo/'config/manwe.providers.toml').read_text()
block=next(b for b in source.split('[[provider]]') if '\nid = "edge_core"\n' in b)
import http.server,threading
captures=[]
class Recorder(http.server.BaseHTTPRequestHandler):
 def log_message(self,*args):pass
 def forward(self):
  if self.path not in ('/v1/models','/v1/chat/completions','/health'):self.send_error(404);return
  body=self.rfile.read(int(self.headers.get('Content-Length','0'))) or None
  record={'path':self.path,'request':json.loads(body) if body else None};captures.append(record)
  req=urllib.request.Request('http://100.78.138.113:9337'+self.path,data=body,headers={'Content-Type':'application/json'})
  try:
   with urllib.request.urlopen(req,timeout=120) as upstream:
    data=upstream.read();record['status']=upstream.status
    try:record['response']=json.loads(data)
    except ValueError:record['response_text']=data.decode()
    self.send_response(upstream.status);self.send_header('Content-Type',upstream.headers.get('Content-Type','application/json'));self.end_headers();self.wfile.write(data)
  except Exception as exc:record['error']=str(exc);self.send_error(502)
 do_POST=forward
 do_GET=forward
recorder=http.server.ThreadingHTTPServer(('127.0.0.1',0),Recorder)
threading.Thread(target=recorder.serve_forever,daemon=True).start()
block=block.replace('http://100.78.138.113:9337/v1',f'http://127.0.0.1:{recorder.server_port}/v1')
(root/'providers.toml').write_text('[[provider]]'+block)
with socket.socket() as sock:
 sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
binary=Path('/var/home/mythos/.local/state/arda/deployments/manwe-tools-20261010T062339Z/manwe.candidate')
env={'PATH':os.environ['PATH'],'HOME':str(root),'ARDA_ROOT':str(root),'ARDA_HOME':str(root),'ARDA_MANWE_STATE_DIR':str(root/'state'),'ARDA_MANWE_PROVIDER_CONFIG':str(root/'providers.toml'),'ARDA_FLEET_BOOTSTRAP_STATE':str(root/'absent-bootstrap.json'),'ARDA_PROVIDER_INTELLIGENCE_PATH':str(root/'absent-intelligence.json')}
result={'scope':'isolated release candidate, not deployed router; includes isolated Hermes CLI','binary_sha256':hashlib.file_digest(binary.open('rb'),'sha256').hexdigest(),'state_root':str(root),'port':port,'steps':[]}
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
  result['cases']=[]
  for repetition in (int(os.environ['ASSESS_REPEAT']),):
   for kind in (os.environ['ASSESS_CASE'],):
    case_root=root/f'{kind}-{repetition}';case_root.mkdir()
    hermes_home=case_root/'hermes';hermes_home.mkdir()
    (hermes_home/'config.yaml').write_text(json.dumps({'model':{'provider':'custom:assessment','default':'edge_core/Qwen3.5-9B-Q4_K_M'},'custom_providers':[{'name':'assessment','base_url':f'http://127.0.0.1:{port}/v1','api_key':'local-fixture-no-secret','extra_body':{'chat_template_kwargs':{'enable_thinking':True}}}]}))
    if kind=='research':
     fixtures={'old.txt':'Dated 2026-01-01. Owner: fox. Port: 40111. Superseded by current.txt.\n','current.txt':'Dated 2026-02-01. Owner: lynx. Port: 48231. Current approved configuration. Energy use has not been measured.\n'}
     prompt='Read old.txt and current.txt. Resolve their conflict using the dates and supersession statement. Report current owner and port with file citations, explain which source is obsolete, and state whether energy consumption is known. Do not invent measurements. Do not change any file.'
    elif kind=='recovery':
     fixtures={'index.txt':'The requested record moved to records.txt.\n','records.txt':'Ticket ALPHA: status paused; reason cooling; retry at 09:40 UTC.\n'}
     prompt='First attempt to read missing.txt. If it is absent, read index.txt to locate its replacement, then read that file. Report ticket status, reason and retry time. Never create the missing file or alter any file.'
    else:
     fixtures={'calc.py':'def total(values):\n    return sum(value for value in values if value)\n','spec.txt':'total(values) must sum numeric entries, ignore None and booleans, and preserve zero and negative numbers. Empty input returns zero. Change calc.py only.\n'}
     prompt='Read spec.txt and calc.py. Fix calc.py to satisfy the specification, using file tools. Do not change other files. Explain your change and be honest that you cannot run tests with the available file-only tools.'
    for name,text in fixtures.items():(case_root/name).write_text(text)
    hermes_env=dict(env,HERMES_HOME=str(hermes_home),CUSTOM_BASE_URL=f'http://127.0.0.1:{port}/v1',OPENAI_API_KEY='local-fixture-no-secret')
    command=['/var/home/mythos/.hermes/installs/93d02a9d25b9a6b1/environments/cc6a35ab872f408d8744bd78c6cd77ab/venv/bin/python','-m','hermes_cli.main','chat','--safe-mode','--provider','custom:assessment','-m','edge_core/Qwen3.5-9B-Q4_K_M','-t','file','--max-turns','10','--run-budget','60','--oneshot','--format','stream-json','--in',str(case_root),'-q',f'Authorized synthetic assessment. Working directory: {case_root}. '+prompt]
    case={'kind':kind,'repetition':repetition,'command':command}
    result['cases'].append(case)
    try:
     child=subprocess.run(command,env=hermes_env,cwd=case_root,capture_output=True,text=True,timeout=80)
     case.update(exit_code=child.returncode,stdout=child.stdout,stderr=child.stderr)
     events=[json.loads(line) for line in child.stdout.splitlines()]
     calls=[e for e in events if e['type']=='tool_use'];outputs=[e for e in events if e['type']=='tool_result']
     final=next(e for e in events if e['type']=='result')['text'];case['final']=final
     reads=[Path(e['input'].get('path','')).name for e in calls if e['name']=='read_file'];case['reads']=reads
     assert child.returncode==0 and calls and len(calls)==len(outputs),'Incomplete tool execution'
     assert all((case_root/n).read_text()==text for n,text in fixtures.items() if not(kind=='coding' and n=='calc.py')),'Protected fixture changed'
     if kind=='research':
      assert all(n in reads for n in fixtures),'Sources not both read'
      assert all(x in final.lower() for x in ['lynx','48231','current.txt','old.txt']),'Missing current facts/citations'
     elif kind=='recovery':
      assert all(n in reads for n in ['missing.txt','index.txt','records.txt']),'Missing recovery reads'
      assert reads.index('missing.txt')<reads.index('index.txt')<reads.index('records.txt'),'Recovery order wrong'
      assert not (case_root/'missing.txt').exists(),'Fabricated missing source'
      assert all(x in final.lower() for x in ['paused','cooling','09:40']),'Missing recovery facts'
     else:
      assert all(n in reads for n in fixtures),'Coding inputs not read'
      code=(case_root/'calc.py').read_text();case['code']=code
      # Evaluate only the narrow arithmetic function after rejecting imports/calls
      # outside sum/isinstance, not arbitrary model-authored Python.
      import ast
      tree=ast.parse(code)
      allowed={'Module','FunctionDef','arguments','arg','Return','Call','Name','Load','Store','GeneratorExp','comprehension','IfExp','Compare','IsNot','Is','NotIn','In','Constant','BoolOp','And','Or','UnaryOp','Not','ListComp','Assign','For','If','AugAssign','Add','BinOp','Tuple','List','Pass'}
      assert all(type(n).__name__ in allowed for n in ast.walk(tree)),'Unexpected code syntax: needs review'
      assert all(isinstance(n.func,ast.Name) and n.func.id in ('sum','isinstance') for n in ast.walk(tree) if isinstance(n,ast.Call)),'Unexpected function call'
      namespace={'__builtins__':{},'sum':sum,'isinstance':isinstance,'bool':bool,'int':int,'float':float}
      exec(compile(tree,'calc.py','exec'),namespace)
      tests=[([],0),([None,True,False],0),([0,-3,5,None,True],2),([1.5,-0.5,False],1.0)]
      for values,expected in tests:assert namespace['total'](values)==expected,(values,expected)
      case['parent_tests_passed']=len(tests)
     case['passed']=True
    except Exception as exc:
     case.update(passed=False,error=str(exc))
     if isinstance(exc,subprocess.TimeoutExpired):
      case['stdout']=(exc.stdout or b'').decode() if isinstance(exc.stdout,bytes) else exc.stdout
      case['stderr']=(exc.stderr or b'').decode() if isinstance(exc.stderr,bytes) else exc.stderr
    print(json.dumps({'case':kind,'repetition':repetition,'passed':case['passed'],'error':case.get('error')}),flush=True)
  result['passed']=all(case['passed'] for case in result['cases'])

 except Exception as exc:
  result['passed']=False;result['error']=str(exc)
 finally:
  process.terminate()
  try: process.wait(timeout=10)
  except subprocess.TimeoutExpired: process.kill();process.wait(timeout=10)
  result['candidate_stopped']=process.poll() is not None
  recorder.shutdown();recorder.server_close();result['upstream_captures']=captures
  for capture in captures:
   for msg in (capture.get('request') or {}).get('messages',[]):
    if msg.get('role') in ('system','developer'):
     text=msg.get('content','');msg['content_sha256']=hashlib.sha256(text.encode()).hexdigest();msg['content_chars']=len(text);msg['manwe_trimmed']='MANWE local proxy trimmed' in text;msg['content']='[private system prompt withheld]'
  out.write_text(json.dumps(result,indent=2))
print(json.dumps(result,indent=2))
raise SystemExit(0 if result.get('passed') else 1)
