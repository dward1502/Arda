"""One read-only real-packet cycle. Only sanitized endpoint facts sent remotely."""
import json
from pathlib import Path
import subprocess
import urllib.request
import urllib.error
import time

HERE=Path(__file__).resolve().parent
OUT=Path(__file__).with_suffix('.json')

def main():
    r={'scope':'Real endpoint self-reports only. No remediation, scheduling, private URLs or credentials in remote packet.','passed':False}
    try:
        run=subprocess.run(['python',str(HERE/'bounded-server-check.py')],capture_output=True,text=True,timeout=140)
        r['collector_exit']=run.returncode
        r['collector']=json.loads(run.stdout)
        if run.returncode or not r['collector'].get('passed'): raise RuntimeError('collector acceptance failed; no remote submission')
        packet=[{'source_id':'S'+str(i+1),'service':x['service'],'observed_at':x['observed_at'],'http_status':x['http_status']} for i,x in enumerate(r['collector']['report']['checks'])]
        r['remote_packet']=packet
        prompt='''Interpret this real timestamped endpoint packet, observation only. Return bare JSON with exactly endpoint_observations (array with service, http_status, source_id for every source), application_workload_health (null), backup_recoverability (null), limitation (string). HTTP 200 establishes only a successful endpoint response at that timestamp, not sustained health, inference quality, workload correctness or recoverable backups. State those limitations; no recommendations, actions, or additional facts. Packet: '''+json.dumps(packet)
        payload={'model':'openrouter/nvidia/nemotron-3-super-120b-a12b:free','allow_forced_provider_fallback':False,'workload_role':'interactive','context_window_target':4096,'max_tokens':1400,'stream':False,'reasoning':{'exclude':True},'messages':[{'role':'user','content':prompt}]}
        r['request']=payload
        OUT.write_text(json.dumps(r,indent=2)+'\n')
        start=time.monotonic()
        request=urllib.request.Request('http://127.0.0.1:7171/v1/chat/completions',data=json.dumps(payload).encode(),headers={'Content-Type':'application/json','x-manwe-task-type':'chat'})
        with urllib.request.urlopen(request,timeout=90) as response:
            r['body']=body=json.load(response)
            r['headers']={k.lower():v for k,v in response.headers.items() if k.lower().startswith('x-manwe')}
        r['remote_elapsed_s']=time.monotonic()-start
        msg=body['choices'][0]['message']; a=json.loads(msg['content']); r['answer']=a
        expected=[{k:s[k] for k in ('service','http_status','source_id')} for s in packet]
        r['passed']=(set(a)=={'endpoint_observations','application_workload_health','backup_recoverability','limitation'} and a['endpoint_observations']==expected and a['application_workload_health'] is None and a['backup_recoverability'] is None and isinstance(a['limitation'],str) and bool(a['limitation'].strip()) and not any(msg.get(k) for k in ['reasoning','reasoning_details','reasoning_content','tool_calls']) and body['choices'][0]['finish_reason']=='stop' and r['headers'].get('x-manwe-provider-id')=='openrouter' and r['headers'].get('x-manwe-model-id')=='nvidia/nemotron-3-super-120b-a12b:free' and body.get('model')=='nvidia/nemotron-3-super-120b-a12b:free')
    except urllib.error.HTTPError as e: r.update(error=e.read(16384).decode(errors='replace'),status=e.code)
    except Exception as e: r['error']=str(e)
    finally: OUT.write_text(json.dumps(r,indent=2)+'\n')
    # Always emit context so a scheduled agent can interpret success or failure.
    print(json.dumps({'scope':r['scope'],'passed':r['passed'],'packet':r.get('remote_packet'),'answer':r.get('answer'),'error':r.get('error'),'evidence_path':str(OUT)},indent=2))
    return 0 if r['passed'] else 1

if __name__=='__main__': raise SystemExit(main())
