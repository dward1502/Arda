"""Isolated Manwe/Hermes real streaming and synthetic broken-stream control."""
import os,json,pathlib,tempfile,subprocess,socket,time,urllib.request,urllib.error,http.server,threading,hashlib
B=pathlib.Path(__file__).parent
root=pathlib.Path(tempfile.mkdtemp(prefix='mimo-stream-',dir='/var/home/mythos/.hermes/cache/scratch'))
result={'root':str(root),'scope':'real MiMo via isolated retained Manwe candidate and installed Hermes; synthetic negative is separate','captures':[],'cases':[],'passed':False}
mode='live'
def save(): (B/'mimo-broken-stream-local-result.json').write_text(json.dumps(result,indent=2))
class Handler(http.server.BaseHTTPRequestHandler):
 def log_message(self,format,*args):pass
 def forward(self):
  body=self.rfile.read(int(self.headers.get('Content-Length','0'))) or None
  record={'mode':mode,'path':self.path,'request':json.loads(body) if body else None,'started':time.monotonic()};result['captures'].append(record);save()
  if mode=='broken' and self.path.endswith('/chat/completions'):
   if not record['request'].get('stream'):
    self.send_error(502,'synthetic negative: nonstream fallback prohibited');return
   packet={'id':'broken-stream-control','object':'chat.completion.chunk','choices':[{'index':0,'delta':{'tool_calls':[{'index':0,'id':'broken-call','type':'function','function':{'name':'read_file','arguments':'{'}}]},'finish_reason':None}]}
   data=('data: '+json.dumps(packet)+'\n\n').encode();record['injected']=packet
   self.send_response(200);self.send_header('Content-Type','text/event-stream');self.send_header('Content-Length',str(len(data)+1000));self.end_headers();self.wfile.write(data);self.wfile.flush();self.close_connection=True;record['aborted']=True;save();return
  if self.path.endswith('/models'):
   data=json.dumps({'object':'list','data':[{'id':'mimo9b-q4km','object':'model'}]}).encode();self.send_response(200);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(data);return
  self.send_error(502,'synthetic-only probe: real upstream prohibited');return
  try:
   req=urllib.request.Request('http://127.0.0.1:18109'+self.path,data=body,headers={'Content-Type':'application/json'})
   with urllib.request.urlopen(req,timeout=100) as upstream:
    record['status']=upstream.status;record['content_type']=upstream.headers.get('Content-Type','');self.send_response(upstream.status);self.send_header('Content-Type',record['content_type']);self.end_headers()
    chunks=[]
    while True:
     data=upstream.read1(4096)
     if not data:break
     chunks.append(data);self.wfile.write(data);self.wfile.flush()
    record['response_text']=b''.join(chunks).decode();record['complete']=time.monotonic();save()
  except Exception as e:record['error']=str(e);save();self.close_connection=True
 do_POST=forward
 do_GET=forward
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler);server.daemon_threads=True
threading.Thread(target=server.serve_forever,daemon=True).start()
source=pathlib.Path('/var/home/mythos/Eregion/Arda/config/manwe.providers.toml').read_text();block=next(x for x in source.split('[[provider]]') if '\nid = "edge_core"\n' in x)
block=block.replace('http://100.78.138.113:9337/v1',f'http://127.0.0.1:{server.server_port}/v1').replace('Qwen3.5-9B-Q4_K_M','mimo9b-q4km');(root/'providers.toml').write_text('[[provider]]'+block)
with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
binary=pathlib.Path('/var/home/mythos/.local/state/arda/deployments/manwe-tools-20261010T062339Z/manwe.candidate')
env={'PATH':os.environ['PATH'],'HOME':str(root),'ARDA_ROOT':str(root),'ARDA_HOME':str(root),'ARDA_MANWE_EMULATE_TOOL_STREAMING':'false','ARDA_MANWE_STATE_DIR':str(root/'state'),'ARDA_MANWE_PROVIDER_CONFIG':str(root/'providers.toml'),'ARDA_FLEET_BOOTSTRAP_STATE':str(root/'absent.json'),'ARDA_PROVIDER_INTELLIGENCE_PATH':str(root/'absent-intelligence.json')}
result['manwe_sha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
log=(root/'manwe.log').open('w');proc=subprocess.Popen([str(binary),'--bind','127.0.0.1','--port',str(port),'--config',str(root/'missing.toml')],env=env,cwd=root,stdout=log,stderr=subprocess.STDOUT)
try:
 deadline=time.monotonic()+20
 while True:
  try:json.load(urllib.request.urlopen(f'http://127.0.0.1:{port}/healthz',timeout=2));break
  except OSError:
   if time.monotonic()>deadline:raise
   time.sleep(.2)
 for label in ['broken']:
  mode=label;case=root/label;case.mkdir();home=case/'hermes';home.mkdir();fixture=case/'record.txt';fixture.write_text('Ticket DELTA: owner otter; status paused; retry 11:25 UTC.\n')
  (home/'config.yaml').write_text(json.dumps({'model':{'provider':'custom:assessment','default':'edge_core/mimo9b-q4km'},'custom_providers':[{'name':'assessment','base_url':f'http://127.0.0.1:{port}/v1','api_key':'local-fixture-no-secret','extra_body':{'chat_template_kwargs':{'enable_thinking':True},'parallel_tool_calls':False,'temperature':0}}]}))
  cmd=['/var/home/mythos/.hermes/installs/93d02a9d25b9a6b1/environments/cc6a35ab872f408d8744bd78c6cd77ab/venv/bin/python','-c',"import faulthandler,runpy;faulthandler.dump_traceback_later(20,repeat=True);runpy.run_module('hermes_cli.main',run_name='__main__')",'chat','--safe-mode','--provider','custom:assessment','-m','edge_core/mimo9b-q4km','-t','file','--max-turns','4','--run-budget','15','--oneshot','--format','stream-json','--in',str(case),'-q',f'Read {fixture} with read_file. Report ticket owner, status and retry time using only the file. Do not change files.']
  row={'mode':label,'command':cmd};result['cases'].append(row);save()
  try:
   p=subprocess.run(cmd,env=dict(env,HERMES_HOME=str(home),CUSTOM_BASE_URL=f'http://127.0.0.1:{port}/v1',OPENAI_API_KEY='local-fixture-no-secret'),cwd=case,capture_output=True,text=True,timeout=45)
   row.update(exit_code=p.returncode,stdout=p.stdout,stderr=p.stderr)
  except subprocess.TimeoutExpired as e:row.update(timeout=True,stdout=(e.stdout or b'').decode(),stderr=(e.stderr or b'').decode())
  events=[]
  for line in row['stdout'].splitlines():
   try:events.append(json.loads(line))
   except ValueError:pass
  calls=[e for e in events if e.get('type')=='tool_use'];outputs=[e for e in events if e.get('type')=='tool_result'];final=next((e.get('text','') for e in events if e.get('type')=='result'),'');row.update(calls=calls,outputs=outputs,final=final)
  streams=[c for c in result['captures'] if c['mode']==label and (c.get('request') or {}).get('stream')]
  if label=='live':row['passed']=row.get('exit_code')==0 and bool(streams) and bool(calls) and len(calls)==len(outputs) and all(x in final.lower() for x in ['otter','paused','11:25'])
  else:
   row['no_execution_observed']=bool(streams) and any(c.get('aborted') for c in streams) and not calls and not outputs
   row['passed']=row['no_execution_observed'] and not row.get('timeout',False)
  assert fixture.read_text()=='Ticket DELTA: owner otter; status paused; retry 11:25 UTC.\n'
  save()
 result['passed']=all(c['passed'] for c in result['cases'])
except Exception as e:result['error']=repr(e)
finally:
 save();proc.terminate()
 try:proc.wait(timeout=10)
 except subprocess.TimeoutExpired:proc.kill();proc.wait()
 server.shutdown();server.server_close();log.close();result['cleanup_complete']=True;save()
print(json.dumps({'passed':result['passed'],'cases':[{k:c.get(k) for k in ['mode','passed','final','exit_code','timeout']} for c in result['cases']],'error':result.get('error')}))
raise SystemExit(0 if result['passed'] else 1)
