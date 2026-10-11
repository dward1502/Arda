"""Synthetic admission probes against isolated installed Manwe bytes."""
import sys,pathlib,os,json,time,socket,subprocess,tempfile,threading,http.server,urllib.request,urllib.error,hashlib
B=pathlib.Path(__file__).parent
ROOT=pathlib.Path(tempfile.mkdtemp(prefix='p4-admission-',dir=os.environ.get('TMPDIR',str(pathlib.Path.home()/'.hermes/cache/scratch'))))
# Build the reviewed source first: cargo build --release -p manwe --bin manwe.
# Pass its absolute executable path as MANWE_TEST_BINARY; its digest is retained.
binary=pathlib.Path(os.environ['MANWE_TEST_BINARY']).resolve();blob=binary.read_bytes()
result={'binary':str(binary),'sha256':hashlib.sha256(blob).hexdigest(),'scope':'synthetic isolated installed-binary admission; not model quality or production outage','cases':[]}
# Self-contained synthetic catalog: never derive test endpoints from live config.
block='''
id = "edge_core"
name = "Synthetic fixture"
base_url = "http://127.0.0.1:1/v1"
enabled = true
access_tier = "local"
quality_band = "high"
requests_per_minute = 60
max_completion_tokens = 2048
probe_model = "fixture"
probe_profile = "benchmark_validated_local"
[[provider.model]]
id = "fixture"
capable_tasks = ["code", "research", "reasoning", "chat", "summary", "background"]
context_window = 73728
is_default = true
capabilities = { tools = true, structured_output = true, streaming = true }
'''

invalids = {'empty-object':{}, 'null':None, 'scalar':3, 'array':[], 'error-object':{'error':{'message':'CANARY_SECRET'}}, 'empty-choices':{'choices':[]}, 'wrong-choices':{'choices':{}}, 'missing-message':{'choices':[{}]}, 'wrong-message':{'choices':[{'message':'bad'}]}, 'wrong-content':{'choices':[{'message':{'role':'assistant','content':7}}]}}
positives = {'positive-text':{'role':'assistant','content':'ok'}, 'positive-tool':{'role':'assistant','content':None,'tool_calls':[{'id':'t','type':'function','function':{'name':'read_file','arguments':'{}'}}]}, 'positive-refusal':{'role':'assistant','content':None,'refusal':'no'}, 'positive-legacy':{'role':'assistant','content':None,'function_call':{'name':'read_file','arguments':'{}'}}}
positives['positive-delayed']={'role':'assistant','content':'delayed body'}
labels = list(invalids)+list(positives)+['malformed-forced','malformed-excluded','incomplete-body','complete-json-incomplete-body','truncated-chunk','body-timeout']+[f'{kind}-{code}' for kind in ('http','truncated') for code in (400,401,403,429,503)]
fallbacks=set(invalids)|{'incomplete-body','complete-json-incomplete-body','truncated-chunk','body-timeout','http-429','http-503','truncated-429','truncated-503'}
calls=[];unavailable=False;paths=[]
class H(http.server.BaseHTTPRequestHandler):
 def log_message(self,format,*args):pass
 def do_POST(self):
  data=json.loads(self.rfile.read(int(self.headers['Content-Length'])));calls.append(data);paths.append(self.path)
  if self.path.startswith('/primary') or unavailable:
   if label in invalids or label in positives:
    value=invalids[label] if label in invalids else {'choices':[{'index':0,'message':positives[label],'finish_reason':'stop'}]}
    self.send_response(200);self.end_headers();self.wfile.flush()
    if label=='positive-delayed':time.sleep(.2)
    self.wfile.write(json.dumps(value).encode());return
   if label.startswith(('http-','truncated-')) and label!='truncated-chunk':
    self.send_response(int(label.split('-')[1]));
    if label.startswith('truncated-'):self.send_header('Content-Length','1000')
    self.end_headers();self.wfile.write(b'{');self.close_connection=True;return
   if label=='body-timeout':
    self.send_response(200);self.send_header('Content-Length','100');self.end_headers();self.wfile.flush();time.sleep(32);self.close_connection=True;return
   if label=='truncated-chunk':
    self.protocol_version='HTTP/1.1'
    self.send_response(200);self.send_header('Transfer-Encoding','chunked');self.end_headers();self.wfile.flush();time.sleep(.2);self.wfile.write(b'20\r\n{}');self.close_connection=True;return
   if label=='complete-json-incomplete-body':
    self.send_response(200);self.send_header('Content-Length','1000');self.end_headers();self.wfile.write(b'{"choices":[{"message":{"role":"assistant","content":"ok"}}]}');self.close_connection=True;return
   if label in ('malformed-failover','malformed-forced','malformed-excluded','invalid-envelope'):
    self.send_response(200);self.end_headers();self.wfile.write(b'{}' if label=='invalid-envelope' else b'{invalid');return
   if label=='incomplete-body':
    self.send_response(200);self.send_header('Content-Length','100');self.end_headers();self.wfile.write(b'{');self.close_connection=True;return
   self.send_error(429 if label=='quota-failover' else 503,'controlled fixture unavailable');return
  payload=json.dumps({'id':'fixture','object':'chat.completion','model':'fixture','choices':[{'index':0,'message':{'role':'assistant','content':'fixture accepted'},'finish_reason':'stop'}]}).encode()
  self.send_response(200);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(payload)
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),H);threading.Thread(target=server.serve_forever,daemon=True).start()
body={'model':'edge_core/fixture','stream':False,'max_tokens':16,'messages':[{'role':'user','content':'Read the local record.'},{'role':'assistant','content':None,'tool_calls':[{'id':'tc1','type':'function','function':{'name':'read_file','arguments':'{"path":"fixture.txt"}'}}]},{'role':'tool','tool_call_id':'tc1','content':'synthetic record'}],'tools':[{'type':'function','function':{'name':'read_file','description':'read local fixture','parameters':{'type':'object','properties':{'path':{'type':'string'}},'required':['path']}}}],'tool_choice':'auto','local_only':True}
try:
 for label in labels:
  ctx=73728;tools=True;unavailable=False
  paths.clear();calls.clear();home=ROOT/label;home.mkdir();cfg='[[provider]]'+block.replace('http://127.0.0.1:1/v1',f'http://127.0.0.1:{server.server_port}/v1')
  if label.startswith('auto-'):
   extra=cfg.replace('edge_core','undersized_alternative').replace(f'context_window = {ctx}','context_window = 32768')
   disabled=cfg.replace('edge_core','tools_disabled_alternative').replace(f'context_window = {ctx}','context_window = 100096').replace('tools = true','tools = false')
   cfg+='\n'+extra+'\n'+disabled
  body['model']='auto' if label.startswith('auto-') else 'edge_core/fixture'
  primary=cfg.replace('/v1','/primary/v1')
  backup=cfg.replace('edge_core','backup_local').replace('/v1','/backup/v1').replace('fixture','backup_model')
  if label in fallbacks or label in positives or label=='malformed-forced':backup=backup.replace('backup_local','edge_backup_local')
  backup=backup.replace('quality_band = "high"','quality_band = "low"');body['route_spread_top_cap']=1;body['route_spread_score_band']=0.0
  cfg=primary+'\n'+backup
  body['model']='edge_core/fixture' if label=='malformed-forced' else 'auto';body.pop('allow_forced_provider_fallback',None)
  cfg=cfg.replace('edge_core','edge_fixture');body['model']=body['model'].replace('edge_core','edge_fixture')
  import tomllib,urllib.parse
  fixture_providers=tomllib.loads(cfg)['provider']
  assert len(fixture_providers)==2
  for item in fixture_providers:
   endpoint=urllib.parse.urlsplit(item['base_url'])
   assert endpoint.scheme=='http' and endpoint.hostname=='127.0.0.1' and endpoint.port==server.server_port
  fixture_ids={item['id'] for item in fixture_providers}
  (home/'providers.toml').write_text(cfg)
  with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
  env={'PATH':os.environ['PATH'],'HOME':str(home),'ARDA_ROOT':str(home),'ARDA_HOME':str(home),'ARDA_MANWE_STATE_DIR':str(home/'state'),'ARDA_MANWE_PROVIDER_CONFIG':str(home/'providers.toml'),'ARDA_FLEET_BOOTSTRAP_STATE':str(home/'absent'),'ARDA_PROVIDER_INTELLIGENCE_PATH':str(home/'absent-intel')}
  log=(home/'manwe.log').open('w');p=subprocess.Popen([str(binary),'--bind','127.0.0.1','--port',str(port),'--config',str(home/'missing.toml')],env=env,cwd=home,stdout=log,stderr=subprocess.STDOUT)
  row={'name':label,'context':ctx,'tools':tools}
  try:
   deadline=time.monotonic()+15
   while True:
    try:urllib.request.urlopen(f'http://127.0.0.1:{port}/healthz',timeout=1).close();break
    except OSError:
     if time.monotonic()>deadline:raise
     time.sleep(.1)
   row['before']=json.load(urllib.request.urlopen(f'http://127.0.0.1:{port}/providers'))
   body['exclude_provider_ids']=[item['id'] for item in row['before']['providers'] if item['id'] not in fixture_ids]
   for item in row['before']['providers']:
    if item['id'] in fixture_ids:
     endpoint=urllib.parse.urlsplit(item['base_url'])
     assert endpoint.hostname=='127.0.0.1' and endpoint.port==server.server_port
   began=time.monotonic()
   req=urllib.request.Request(f'http://127.0.0.1:{port}/v1/chat/completions',data=json.dumps(body).encode(),headers={'Content-Type':'application/json'})
   try:
    with urllib.request.urlopen(req,timeout=40) as response:row.update(status=response.status,body=response.read().decode(),headers={k:v for k,v in response.headers.items() if k.lower().startswith('x-manwe')})
   except urllib.error.HTTPError as e:row.update(status=e.code,body=e.read().decode())
   row['elapsed_seconds']=time.monotonic()-began
   row['after']=json.load(urllib.request.urlopen(f'http://127.0.0.1:{port}/providers'))
   row['upstream_requests']=list(calls);row['upstream_paths']=list(paths)
   decoded=json.loads(row['body'])
   if label in positives:
    row['passed']=row['status']==200 and paths==['/primary/v1/chat/completions'] and decoded['_manwe_route']['provider_id']=='edge_fixture'
    message=decoded['choices'][0]['message']
    if label=='positive-legacy':row['passed'] &= message['content'] is None and message['tool_calls'][0]['function']==positives[label]['function_call'] and decoded['choices'][0]['finish_reason']=='tool_calls'
    else:row['passed'] &= message==positives[label]
    if label=='positive-delayed':row['passed'] &= decoded['_manwe_route']['latency_ms']>=180
   elif label in fallbacks:
    row['passed']=row['status']==200 and paths==['/primary/v1/chat/completions','/backup/v1/chat/completions'] and decoded['_manwe_route']['provider_id']=='edge_backup_local' and row['headers']['x-manwe-provider-id']=='edge_backup_local'
    row['passed'] &= all(c['messages']==json.loads(json.dumps(body['messages']).replace('tc1','tc0000001')) and c['tools']==body['tools'] for c in calls)
   else:row['passed']=row['status']==502 and paths==['/primary/v1/chat/completions'] and decoded['error']['type']=='manwe_error'

   primary_state=next(p for p in row['after']['providers'] if p['id']=='edge_fixture')
   row['passed'] &= primary_state['active_connections']==0 and 'CANARY_SECRET' not in row['body']
   if label=='truncated-chunk':row['passed'] &= (primary_state['avg_latency_ms'] or 0)>=180 and 'HTTP 502' in (primary_state['last_error'] or '')
   if label=='body-timeout':row['passed'] &= primary_state['healthy'] and primary_state['error_count']==0 and 19 <= row['elapsed_seconds'] < 30
   elif label in invalids or label in ('malformed-forced','malformed-excluded','incomplete-body','complete-json-incomplete-body','truncated-chunk'):row['passed'] &= primary_state['consecutive_successes']==0
   if label in ('http-429','truncated-429'):row['passed'] &= primary_state['healthy'] and primary_state['error_count']==0
   elif label.startswith(('http-','truncated-')) and label!='truncated-chunk':row['passed'] &= ('HTTP '+label.split('-')[1]) in (primary_state['last_error'] or '')

  except Exception as e:row['error']=str(e);row['passed']=False
  finally:
   p.terminate()
   try:p.wait(timeout=5)
   except subprocess.TimeoutExpired:p.kill();p.wait()
   log.close();row['router_log']=(home/'manwe.log').read_text();result['cases'].append(row);(B/('operating-envelope-matrix-'+sys.argv[1]+'.json')).write_text(json.dumps(result,indent=2))
finally:server.shutdown();server.server_close()
result['passed']=len(result['cases'])==len(labels) and all(r['passed'] for r in result['cases']);(B/('operating-envelope-matrix-'+sys.argv[1]+'.json')).write_text(json.dumps(result,indent=2));print(json.dumps([{k:r.get(k) for k in ['name','status','passed','error']} for r in result['cases']]));raise SystemExit(0 if result['passed'] else 1)
