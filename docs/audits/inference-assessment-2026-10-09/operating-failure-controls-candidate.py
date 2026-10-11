"""Synthetic admission probes against isolated installed Manwe bytes."""
import pathlib,os,json,time,socket,subprocess,tempfile,threading,http.server,urllib.request,urllib.error,hashlib
B=pathlib.Path(__file__).parent
ROOT=pathlib.Path(tempfile.mkdtemp(prefix='p4-admission-',dir='/var/home/mythos/.hermes/cache/scratch'))
binary=pathlib.Path('/var/home/mythos/.hermes/cache/scratch/manwe-release-dc3f3a67/release/manwe');blob=binary.read_bytes()
result={'binary':str(binary),'sha256':hashlib.sha256(blob).hexdigest(),'scope':'synthetic isolated installed-binary admission; not model quality or production outage','cases':[]}
source=pathlib.Path('/var/home/mythos/Eregion/Arda/config/manwe.providers.toml').read_text();block=next(x for x in source.split('[[provider]]') if '\nid = "edge_core"\n' in x)
calls=[];unavailable=False;paths=[]
class H(http.server.BaseHTTPRequestHandler):
 def log_message(self,format,*args):pass
 def do_POST(self):
  data=json.loads(self.rfile.read(int(self.headers['Content-Length'])));calls.append(data);paths.append(self.path)
  if self.path.startswith('/primary') or unavailable:
   if label=='malformed-failover':
    self.send_response(200);self.end_headers();self.wfile.write(b'{invalid');return
   self.send_error(429 if label=='quota-failover' else 503,'controlled fixture unavailable');return
  payload=json.dumps({'id':'fixture','object':'chat.completion','model':'fixture','choices':[{'index':0,'message':{'role':'assistant','content':'fixture accepted'},'finish_reason':'stop'}]}).encode()
  self.send_response(200);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(payload)
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),H);threading.Thread(target=server.serve_forever,daemon=True).start()
body={'model':'edge_core/fixture','stream':False,'max_tokens':16,'messages':[{'role':'user','content':'Read the local record.'},{'role':'assistant','content':None,'tool_calls':[{'id':'tc1','type':'function','function':{'name':'read_file','arguments':'{"path":"fixture.txt"}'}}]},{'role':'tool','tool_call_id':'tc1','content':'synthetic record'}],'tools':[{'type':'function','function':{'name':'read_file','description':'read local fixture','parameters':{'type':'object','properties':{'path':{'type':'string'}},'required':['path']}}}],'tool_choice':'auto','local_only':True}
try:
 for label,ctx,tools,unavailable in [('failover',73728,True,False),('remote-excluded',73728,True,False),('quota-failover',73728,True,False),('malformed-failover',73728,True,False)]:
  paths.clear();calls.clear();home=ROOT/label;home.mkdir();cfg='[[provider]]'+block.replace('http://100.78.138.113:9337/v1',f'http://127.0.0.1:{server.server_port}/v1').replace('Qwen3.5-9B-Q4_K_M','fixture').replace('context_window = 73728',f'context_window = {ctx}').replace('tools = true',f'tools = {str(tools).lower()}')
  if label.startswith('auto-'):
   extra=cfg.replace('edge_core','undersized_alternative').replace(f'context_window = {ctx}','context_window = 32768')
   disabled=cfg.replace('edge_core','tools_disabled_alternative').replace(f'context_window = {ctx}','context_window = 100096').replace('tools = true','tools = false')
   cfg+='\n'+extra+'\n'+disabled
  body['model']='auto' if label.startswith('auto-') else 'edge_core/fixture'
  primary=cfg.replace('/v1','/primary/v1')
  backup=cfg.replace('edge_core','backup_local').replace('/v1','/backup/v1').replace('fixture','backup_model')
  if label in ('failover','quota-failover','malformed-failover'):backup=backup.replace('backup_local','edge_backup_local')
  backup=backup.replace('priority = 1','priority = 99')
  cfg=primary+'\n'+backup
  body['model']='auto';body.pop('allow_forced_provider_fallback',None)
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
   req=urllib.request.Request(f'http://127.0.0.1:{port}/v1/chat/completions',data=json.dumps(body).encode(),headers={'Content-Type':'application/json'})
   try:
    with urllib.request.urlopen(req,timeout=20) as response:row.update(status=response.status,body=response.read().decode(),headers={k:v for k,v in response.headers.items() if k.lower().startswith('x-manwe')})
   except urllib.error.HTTPError as e:row.update(status=e.code,body=e.read().decode())
   row['upstream_requests']=list(calls);row['upstream_paths']=list(paths)
   decoded=json.loads(row['body'])
   if label in ('failover','quota-failover','malformed-failover'):
    row['passed']=row['status']==200 and paths==['/primary/v1/chat/completions','/backup/v1/chat/completions'] and decoded['_manwe_route']['provider_id']=='edge_backup_local' and row['headers']['x-manwe-provider-id']=='edge_backup_local'
    row['passed'] &= all(c['messages']==json.loads(json.dumps(body['messages']).replace('tc1','tc0000001')) and c['tools']==body['tools'] for c in calls)
   else:row['passed']=row['status']==502 and paths==['/primary/v1/chat/completions'] and decoded['error']['type']=='manwe_error'

  except Exception as e:row['error']=str(e);row['passed']=False
  finally:
   p.terminate()
   try:p.wait(timeout=5)
   except subprocess.TimeoutExpired:p.kill();p.wait()
   log.close();row['router_log']=(home/'manwe.log').read_text();result['cases'].append(row);(B/'operating-failure-controls-candidate.json').write_text(json.dumps(result,indent=2))
finally:server.shutdown();server.server_close()
result['passed']=len(result['cases'])==4 and all(r['passed'] for r in result['cases']);(B/'operating-failure-controls-candidate.json').write_text(json.dumps(result,indent=2));print(json.dumps([{k:r.get(k) for k in ['name','status','passed','error']} for r in result['cases']]));raise SystemExit(0 if result['passed'] else 1)
