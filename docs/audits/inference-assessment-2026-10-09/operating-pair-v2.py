"""Matched public-safe interpretation and capped Server endpoint check.
Experimental, manually invoked, no scheduler or promotion. No reasoning display.
"""
import concurrent.futures
import importlib.util
import json
from pathlib import Path
import subprocess
import time
import urllib.request
import urllib.error

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('screen',HERE/'escalation-semantic-alternatives.py')
assert spec and spec.loader
screen=importlib.util.module_from_spec(spec); spec.loader.exec_module(screen)
OUT=Path(__file__).with_suffix('.json')
PROMPT=screen.PROMPT+'\nCitation rule v2: cite the source establishing an unknown/null judgment too; backups_recoverable must cite S4. Return only the six requested keys. No recommendations.'
PAYLOAD={'model':'openrouter/nvidia/nemotron-3-super-120b-a12b:free','allow_forced_provider_fallback':False,'workload_role':'interactive','context_window_target':4096,'max_tokens':1024,'stream':False,'reasoning':{'exclude':True},'messages':[{'role':'user','content':PROMPT}]}

def desktop():
    return subprocess.check_output(['systemctl','--user','show','mesh-llm.service','-p','ActiveState','-p','MainPID'],text=True,timeout=5).strip()

def remote():
    row={'started_ns':time.monotonic_ns(),'request':PAYLOAD,'passed':False}
    try:
        req=urllib.request.Request('http://127.0.0.1:7171/v1/chat/completions',data=json.dumps(PAYLOAD).encode(),headers={'Content-Type':'application/json','x-manwe-task-type':'chat'})
        with urllib.request.urlopen(req,timeout=75) as response:
            body=json.load(response); row['body']=body
            row['headers']={k.lower():v for k,v in response.headers.items() if k.lower().startswith('x-manwe')}
            row['status']=response.status
        choice=body['choices'][0]; msg=choice['message']
        # Only final content is eligible for the experiment's answer surface.
        row['final_content']=msg.get('content')
        row['reasoning_absent']=not msg.get('reasoning') and not msg.get('reasoning_details') and not msg.get('reasoning_content')
        a=json.loads(msg['content']); row['parsed']=a
        expected={'current_cpu_pressure':['S2'],'active_swapping':['S2'],'backups_recoverable':['S4'],'endpoint_responding':['S3']}
        row['contract_pass']=(set(a)==set(expected)|{'evidence','limitation'} and a['current_cpu_pressure'] is False and a['active_swapping'] is False and a['backups_recoverable'] is None and a['endpoint_responding'] is True and a['evidence']==expected and isinstance(a['limitation'],str) and bool(a['limitation'].strip()))
        row['identity_pass']=(row['headers'].get('x-manwe-provider-id')=='openrouter' and row['headers'].get('x-manwe-model-id')==PAYLOAD['model'].removeprefix('openrouter/') and body.get('model')==PAYLOAD['model'].removeprefix('openrouter/') and choice['finish_reason']=='stop' and not msg.get('tool_calls'))
        row['passed']=row['contract_pass'] and row['identity_pass'] and row['reasoning_absent']
    except urllib.error.HTTPError as e: row.update(status=e.code,error=e.read(16384).decode(errors='replace'))
    except Exception as e: row['error']=str(e)
    row['ended_ns']=time.monotonic_ns()
    return row

def background():
    row: dict = {'started_ns':time.monotonic_ns()}
    run=subprocess.run(['python',str(HERE/'bounded-server-check.py')],capture_output=True,text=True,timeout=140)
    row.update(ended_ns=time.monotonic_ns(),exit_code=run.returncode,stderr=run.stderr)
    row['evidence']=json.loads(run.stdout)
    row['passed']=run.returncode==0 and row['evidence'].get('passed') is True
    return row

def main():
    r: dict = {'scope':'Matched synthetic interpretation plus endpoint-only Server job; client intervals do not prove overlapping upstream compute. Not scheduled, no promotion. Semantic/authority review required.','desktop_before':desktop()}
    def save(): OUT.write_text(json.dumps(r,indent=2)+'\n')
    if r['desktop_before']!='MainPID=0\nActiveState=inactive': raise RuntimeError('desktop guard failed')
    r['sequential_remote']=remote(); save()
    # Do not advance past a failed reasoning-surface or contract preflight.
    if r['sequential_remote']['passed']:
        r['sequential_background']=background(); save()
        if r['sequential_background']['passed']:
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
                a=pool.submit(remote); b=pool.submit(background)
                r['concurrent_remote']=a.result(); r['concurrent_background']=b.result()
            a=r['concurrent_remote']; b=r['concurrent_background']
            r['client_overlap_ns']=max(0,min(a['ended_ns'],b['ended_ns'])-max(a['started_ns'],b['started_ns']))
    r['desktop_after']=desktop()
    r['mechanical_pass']=all(r.get(k,{}).get('passed',False) for k in ['sequential_remote','sequential_background','concurrent_remote','concurrent_background']) and r.get('client_overlap_ns',0)>0 and r['desktop_before']==r['desktop_after']
    save()
    print(json.dumps({k:v for k,v in r.items() if not isinstance(v,dict)},indent=2))
    for k,v in r.items():
        if isinstance(v,dict): print(k,json.dumps({field:v.get(field) for field in ['passed','reasoning_absent','parsed','error','started_ns','ended_ns']},ensure_ascii=False))
    return 0 if r['mechanical_pass'] else 1

if __name__=='__main__': raise SystemExit(main())
