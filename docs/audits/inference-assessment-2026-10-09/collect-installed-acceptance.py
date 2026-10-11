import datetime,hashlib,json,subprocess,urllib.request
from pathlib import Path
BASE=Path('/var/home/mythos/Eregion/Arda/docs/audits/inference-assessment-2026-10-09')
def load(p):return json.loads(p.read_text())
a=load(BASE/'subscription-hermes-acceptance.json')
r={'scope':'Bounded installed file-only Hermes evidence assessment overlapping timer-triggered capped Server observations with remote interpretation. Not full-profile, voice, gaming, automatic-routing or long-term stability qualification.','collected_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'hermes':a,'sources':{p.name:p.read_text() for p in Path(a['root']).iterdir() if p.is_file() and p.name!='allocation-report.txt'},'cycles':[{'path':str(p),'result':load(p)} for p in sorted((BASE/'scheduled-acceptance').glob('*/result.json'))]}
def ts(s):return datetime.datetime.fromisoformat(s).timestamp()
start=ts(a['started_at']);end=ts(a['finished_at'])
r['hermes_interval']=[start,end]
r['overlap_seconds']=[{'path':c['path'],'seconds':max(0,min(end,ts(c['result']['finished_at']))-max(start,ts(c['result']['started_at'])))} for c in r['cycles']]
with urllib.request.urlopen('http://127.0.0.1:7171/route_history',timeout=10) as res:r['route_history']=json.load(res)
with urllib.request.urlopen('http://127.0.0.1:7171/providers',timeout=10) as res:
    p=next(p for p in json.load(res)['providers'] if p['id']=='openai_sub')
    r['provider_after']={k:p.get(k) for k in ('id','enabled','healthy','last_error','in_cooldown','consecutive_failures')}
r['desktop_after']=subprocess.check_output(['systemctl','--user','show','mesh-llm.service','-p','ActiveState','-p','MainPID'],text=True).strip()
r['deployment']=load(BASE/'escalation-timeout-deployment.json')
r['binary_sha256']=hashlib.file_digest(open('/proc/'+str(r['deployment']['new_pid'])+'/exe','rb'),'sha256').hexdigest()
r['mechanical_pass']=a['passed'] and len(r['cycles'])==3 and all(c['result']['passed'] for c in r['cycles']) and any(x['seconds']>0 for x in r['overlap_seconds']) and r['provider_after']['healthy'] and not r['provider_after']['last_error'] and r['desktop_after']=='MainPID=0\nActiveState=inactive' and r['binary_sha256']==r['deployment']['running_sha256']
import sqlite3,runpy
cron=Path('/var/home/mythos/.hermes/cron'); connection=sqlite3.connect('file:'+str(cron/'executions.db')+'?mode=ro',uri=True);connection.row_factory=sqlite3.Row
r['scheduler_executions']=[dict(x) for x in connection.execute('select * from executions where job_id=?',('fc6b7719f468',))]
connection.close()
jobs=json.loads((cron/'jobs.json').read_text()); rows=jobs if isinstance(jobs,list) else jobs['jobs']
r['scheduler_job']=next(x for x in rows if x['id']=='fc6b7719f468' or x.get('job_id')=='fc6b7719f468')
r['scheduler_outputs']=[{'path':str(p),'text':p.read_text()} for p in sorted((cron/'output/fc6b7719f468').glob('*.md'))]
for c in r['cycles']:
    cr=c['result'];matches=[x for x in r['scheduler_executions'] if ts(x['started_at'])<=ts(cr['started_at'])<=ts(cr['finished_at'])<=ts(x['finished_at'])]
    assert len(matches)==1 and matches[0]['source']=='builtin' and matches[0]['status']=='completed' and matches[0]['scheduled_instant']
    outputs=[x for x in r['scheduler_outputs'] if c['path'] in x['text']];assert len(outputs)==1
    emitted=json.loads(next(line for line in outputs[0]['text'].splitlines() if line.startswith('{')))
    assert emitted['passed'] and emitted['packet']==cr['packet'] and emitted['answer']==cr['answer']
    assert cr['headers']['x-manwe-execution-lane']=='background'
    assert set(cr['answer'])=={'endpoint_observations','application_workload_health','backup_recoverability','limitation'}
    c['scheduler_execution_id']=matches[0]['id']
    c['semantic_review']='parent readback: exact cited endpoint facts, unknown workload/backup status, limitation only; no action advice'
assert len(r['scheduler_executions'])==3 and len(r['scheduler_outputs'])==3 and not r['scheduler_job']['enabled'] and r['scheduler_job']['next_run_at'] is None
r['strict_event_validation']=runpy.run_path(str(BASE/'validate-installed-acceptance.py'))['validate'](a,r['sources'])
r['review']={'delegation_id':'deleg_5ba72afa','reviewed_aggregate_sha256':'1993479dbd64b1710f5ae56c720b36782f9c561690c2059706f92dc2bddb7336','reviewed_report_sha256':'a5b9b64474acfa6be4478a32051b84460b342014816a5f59fae71f3e2f3eedd6','scope':'Independent report/source/event review of first two cycles; final third cycle and scheduler correlation verified by parent, not independently reviewed.'}
r['review']['reviewed_report_sha256']='a5b9b64474acfa6be4478a32051b84460b342014816a5f59fae71f3e2f3eedd6'
assert hashlib.sha256((BASE/'installed-operating-acceptance.json').read_bytes()).hexdigest()==r['review']['reviewed_aggregate_sha256']
assert hashlib.sha256(a['report'].encode()).hexdigest()==r['review']['reviewed_report_sha256']
r['qualifications']=['Original report phrase No inference requests means no additional probes initiated by the assessment agent; Hermes itself used inference.','Nested collector manually invoked scope describes the collector entrypoint; scheduler executions above establish timer dispatch of its wrapper.','First cycle predates timeout repair; pause/resume and three bounded fires do not establish contiguous sustained operation.','File-only configuration is not a filesystem sandbox.','Mechanical assertions do not establish semantic correctness for future responses.']
r['mechanical_pass']=r['mechanical_pass'] and r['strict_event_validation']
out=BASE/'installed-operating-acceptance-final.json';out.write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({k:r[k] for k in ('mechanical_pass','overlap_seconds','provider_after','desktop_after','binary_sha256')}))
print('evidence',out)
