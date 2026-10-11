"""Bounded free-route semantic screening; synthetic public-safe evidence only."""
import json
from pathlib import Path
import time
import urllib.request
import urllib.error
import subprocess

OUT=Path(__file__).with_suffix('.json')
PROMPT='''Assess this synthetic monitoring packet, observation only. Return bare JSON with keys current_cpu_pressure (boolean or null), active_swapping (boolean or null), backups_recoverable (boolean or null), endpoint_responding (boolean), evidence (object mapping those four keys to source-ID arrays), limitation (string). No actions or recommendations.
S1: vmstat first row is since-boot average: CPU idle 2%, swap-in 200 KiB/s.
S2: second row is a one-second sample: CPU idle 98%, swap-in 0, swap-out 0. Occupied swap 200000 KiB; total swap capacity not provided.
S3: one GET /health returned HTTP 200. No workload test.
S4: backup utility is installed. No successful backup or restore evidence supplied.
Use S2 for current sampled CPU/swap activity. Unknown recovery must be null. Explain that the sample does not establish sustained health.'''

def main():
    result={'scope':'Synthetic semantic screening, not representative difficult-work qualification','runs':[]}
    result['desktop_before']=subprocess.check_output(['systemctl','--user','show','mesh-llm.service','-p','ActiveState','-p','MainPID'],text=True)
    for model in ['nvidia/nemotron-3-super-120b-a12b:free','poolside/laguna-xs-2.1:free']:
        payload={'model':'openrouter/'+model,'allow_forced_provider_fallback':False,'workload_role':'interactive','context_window_target':4096,'max_tokens':768,'stream':False,'messages':[{'role':'user','content':PROMPT}]}
        row={'request':payload,'passed':False}; result['runs'].append(row)
        OUT.write_text(json.dumps(result,indent=2)+'\n')
        start=time.monotonic()
        try:
            req=urllib.request.Request('http://127.0.0.1:7171/v1/chat/completions',data=json.dumps(payload).encode(),headers={'Content-Type':'application/json','x-manwe-task-type':'chat'})
            with urllib.request.urlopen(req,timeout=75) as response:
                row.update(status=response.status,body=json.load(response),headers={k.lower():v for k,v in response.headers.items() if k.lower().startswith('x-manwe')})
            choice=row['body']['choices'][0]
            answer=json.loads(choice['message']['content'])
            row['parsed']=answer
            row['mechanical_pass']=(answer.get('current_cpu_pressure') is False and answer.get('active_swapping') is False and answer.get('backups_recoverable','missing') is None and answer.get('endpoint_responding') is True and answer.get('evidence')=={'current_cpu_pressure':['S2'],'active_swapping':['S2'],'backups_recoverable':['S4'],'endpoint_responding':['S3']} and isinstance(answer.get('limitation'),str) and bool(answer['limitation'].strip()))
            row['identity_pass']=(row['headers'].get('x-manwe-provider-id')=='openrouter' and row['headers'].get('x-manwe-model-id')==model and choice['finish_reason']=='stop' and not choice['message'].get('tool_calls'))
            row['passed']=row['mechanical_pass'] and row['identity_pass']
        except urllib.error.HTTPError as e: row.update(status=e.code,error=e.read(8192).decode(errors='replace'))
        except Exception as e: row['error']=str(e)
        row['elapsed_s']=time.monotonic()-start
        OUT.write_text(json.dumps(result,indent=2)+'\n')
    result['desktop_after']=subprocess.check_output(['systemctl','--user','show','mesh-llm.service','-p','ActiveState','-p','MainPID'],text=True)
    result['review_required']=True
    OUT.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
    return 0 if all(r['passed'] for r in result['runs']) else 1

if __name__=='__main__': raise SystemExit(main())
