"""Bounded acceptance cycles: capped collector + routed interpretation, no remediation."""
import contextlib, datetime, importlib.util, io, json, time, urllib.request, uuid
from pathlib import Path
BASE=Path('/var/home/mythos/Eregion/Arda/docs/audits/inference-assessment-2026-10-09')
root=BASE/'scheduled-acceptance'/uuid.uuid4().hex
root.mkdir(parents=True)
r={'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'passed':False,'scope':'Capped endpoint observation and routed interpretation; scheduler provenance requires dispatch receipts. No remediation or workload-health claim.','semantic_review_required':True}
try:
    spec=importlib.util.spec_from_file_location('collector',BASE/'bounded-server-check.py')
    mod=importlib.util.module_from_spec(spec); spec.loader.exec_module(mod); mod.HERE=root
    with contextlib.redirect_stdout(io.StringIO()) as output:
        code=mod.main()
    r['collector']=json.loads(output.getvalue())
    assert code==0 and r['collector']['passed'], 'collector failed'
    packet=[{'source_id':'S'+str(i+1),**{k:x[k] for k in ('service','observed_at','http_status')}} for i,x in enumerate(r['collector']['report']['checks'])]
    r['packet']=packet
    payload={'model':'openai_sub/gpt-6-astra','allow_forced_provider_fallback':False,'workload_role':'background','context_window_target':4096,'max_tokens':1000,'stream':False,'messages':[{'role':'user','content':'Interpret this real endpoint packet, observation only. Return bare JSON with endpoint_observations (array of service, http_status, source_id for every source), application_workload_health (null when not established), backup_recoverability (null when not established), limitation (string). Do not recommend actions or invent measurements. Packet: '+json.dumps(packet)}]}
    r['request']=payload
    req=urllib.request.Request('http://127.0.0.1:7171/v1/chat/completions',data=json.dumps(payload).encode(),headers={'Content-Type':'application/json'})
    start=time.monotonic()
    with urllib.request.urlopen(req,timeout=90) as response:
        r['response']=body=json.load(response)
        r['headers']={k.lower():v for k,v in response.headers.items() if k.lower().startswith('x-manwe')}
    r['remote_elapsed_s']=time.monotonic()-start
    choice=body['choices'][0]; msg=choice['message']; answer=json.loads(msg['content']);r['answer']=answer
    expected=[{k:x[k] for k in ('service','http_status','source_id')} for x in packet]
    checks={'facts':answer.get('endpoint_observations')==expected,'unknown_workloads':'application_workload_health' in answer and answer['application_workload_health'] is None,'unknown_backups':'backup_recoverability' in answer and answer['backup_recoverability'] is None,'limitation':isinstance(answer.get('limitation'),str) and bool(answer['limitation'].strip()),'terminal':choice['finish_reason']=='stop','no_calls':not msg.get('tool_calls'),'provider':r['headers'].get('x-manwe-provider-id')=='openai_sub','model':r['headers'].get('x-manwe-model-id')=='gpt-6-astra'}
    checks['exact_fields']=set(answer)=={'endpoint_observations','application_workload_health','backup_recoverability','limitation'}
    checks['background_lane']=r['headers'].get('x-manwe-execution-lane')=='background'
    r['checks']=checks;r['passed']=all(checks.values())
except Exception as e:
    r['error']=type(e).__name__+': '+str(e)
finally:
    r['finished_at']=datetime.datetime.now(datetime.timezone.utc).isoformat()
    (root/'result.json').write_text(json.dumps(r,indent=2)+'\n')
    print(json.dumps({'evidence':str(root/'result.json'),'passed':r['passed'],'packet':r.get('packet'),'answer':r.get('answer'),'error':r.get('error')}))
raise SystemExit(0 if r['passed'] else 1)
