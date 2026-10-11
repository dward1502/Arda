import datetime,json,os,subprocess,tempfile,time,signal
from pathlib import Path
REPO=Path('/var/home/mythos/Eregion/Arda')
BASE=REPO/'docs/audits/inference-assessment-2026-10-09'
root=Path(tempfile.mkdtemp(prefix='subscription-hermes-',dir='/var/home/mythos/.hermes/cache/scratch'))
home=root/'home'; home.mkdir()
config={'model':{'provider':'custom','default':'openai_sub/gpt-6-astra'},'agent':{'auto_recovery_cycles':0}}
(home/'config.yaml').write_text(json.dumps(config))
# Source facts are copied from retained real evidence, not invented results.
prior=json.loads((BASE/'real-endpoint-interpretation.json').read_text())
(root/'endpoint-packet.json').write_text(json.dumps(prior['remote_packet'],indent=2))
deployment=json.loads((BASE/'escalation-deployment-authorized.json').read_text())
(root/'route-status.json').write_text(json.dumps({k:deployment[k] for k in ('passed','provider_readback','parent_unchanged','desktop_after')},indent=2))
plan=(REPO/'docs/plans/INFERENCE_ASSESSMENT.md').read_text()
start=plan.index('The [explicit-format follow-up]'); end=plan.index('The next [bounded deterministic Server check]')
(root/'local-model-findings.txt').write_text(plan[start:end])
(root/'index.txt').write_text('Current monitoring facts are in endpoint-packet.json. That is retained historical evidence, not a live sample.\n')
protected={p.name:p.read_bytes() for p in root.iterdir() if p.is_file()}
prompt=f'''Work only in {root}. This is an authorized bounded file-only assessment using retained real Arda evidence. First read latest-monitoring.json; if absent consult index.txt to locate the retained packet. Read route-status.json and local-model-findings.txt too. Write allocation-report.txt recommending an operating arrangement that preserves the desktop GPU, distinguishes demonstrated roles from failed ones, and states what the timestamps can and cannot establish. Address structured extraction, remote escalation, endpoint health versus workload health, backup recoverability and remaining operating acceptance. Cite file names for factual claims. Do not change source files, invent live observations, run commands or propose purchases. Your final answer should identify the report and disclose unavailable evidence. You have only file tools; do not claim tests or live checks you did not perform.'''
args=['hermes','chat','--provider','custom','-m','openai_sub/gpt-6-astra','-t','file','--ignore-rules','--max-turns','12','--run-budget','180','--oneshot','--format','stream-json','--in',str(root),'-q',prompt]
cmd=json.loads(subprocess.check_output(['hermes','--print-runtime-command'],text=True))
prefix=cmd[-1].split('runpy.run_module(')[0]
cmd[-1]=prefix+'sys.argv='+repr(args)+'; runpy.run_module("hermes_cli.main",run_name="__main__")'
env=dict(os.environ,HERMES_HOME=str(home),CUSTOM_BASE_URL='http://127.0.0.1:7171/v1',CUSTOM_API_KEY='local-assessment-no-secret',OPENAI_API_KEY='local-assessment-no-secret')
for key in list(env):
    if key.endswith('_API_KEY') and key not in ('OPENAI_API_KEY','CUSTOM_API_KEY'): env.pop(key)
r={'root':str(root),'scope':'Installed Hermes file-only assessment of retained real evidence; not full default-profile or live-source discovery qualification.','started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'passed':False,'prompt':prompt}
OUT=BASE/'subscription-hermes-acceptance.json'
OUT.write_text(json.dumps(r,indent=2))
proc=subprocess.Popen(cmd,cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True)
try:
    stdout,stderr=proc.communicate(timeout=210)
    r.update(exit_code=proc.returncode,stdout=stdout,stderr=stderr)
    events=[]
    for line in stdout.splitlines():
        try: events.append(json.loads(line))
        except ValueError: pass
    r['events']=events
    calls=[e for e in events if e.get('type')=='tool_use']
    outputs=[e for e in events if e.get('type')=='tool_result']
    finals=[e for e in events if e.get('type')=='result']
    r['final']=finals[-1] if finals else None
    report=root/'allocation-report.txt'
    r['report']=report.read_text() if report.exists() else None
    r['checks']={'exit_ok':proc.returncode==0,'calls':bool(calls),'results_complete':len(calls)==len(outputs),'final':bool(finals) and finals[-1].get('exit_code')==0 and not finals[-1].get('error'),'report_written':bool(r['report']),'sources_preserved':all((root/name).read_bytes()==data for name,data in protected.items()),'missing_not_created':not (root/'latest-monitoring.json').exists()}
    import runpy
    validate=runpy.run_path(str(BASE/'validate-installed-acceptance.py'))['validate']
    r['checks']['required_event_chain']=validate(r,{name:data.decode() for name,data in protected.items()})
    r['passed']=all(r['checks'].values())
except subprocess.TimeoutExpired:
    os.killpg(proc.pid,signal.SIGTERM)
    try: stdout,stderr=proc.communicate(timeout=10)
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid,signal.SIGKILL);stdout,stderr=proc.communicate()
    r.update(error='timeout',stdout=stdout,stderr=stderr)
finally:
    r['finished_at']=datetime.datetime.now(datetime.timezone.utc).isoformat()
    OUT.write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({k:r.get(k) for k in ('root','passed','checks','error','final')}))
raise SystemExit(0 if r['passed'] else 1)
