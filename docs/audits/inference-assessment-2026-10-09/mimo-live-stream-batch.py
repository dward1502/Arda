import os,subprocess,time,json,urllib.request,hashlib
from pathlib import Path
b=Path('/var/home/mythos/Eregion/Arda/docs/audits/inference-assessment-2026-10-09')
ssh=['ssh','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','annunimas-server@100.102.250.115']
result={'scope':'authorized single GPU MiMo9B test; original 60-second task budget','cases':[]}
tunnel=None

def remote(command):
    return subprocess.run(ssh+[command],text=True,capture_output=True,timeout=45,check=True).stdout

def health(url,seconds=60):
    end=time.monotonic()+seconds
    while True:
        try:
            with urllib.request.urlopen(url,timeout=5) as r:
                value=json.load(r)
                if value.get('status')=='ok':return value
        except Exception:
            if time.monotonic()>=end:raise
        time.sleep(2)

try:
    print(remote('systemd-run --user --unit=inference-assessment-mimo-stream-restore --on-active=660s /usr/bin/systemctl --user start llama-server-bonsai-q1.service && systemctl --user stop llama-server-bonsai-q1.service && systemd-run --user --unit=inference-assessment-mimo-stream --collect --property=RuntimeMaxSec=600 --property=TimeoutStopSec=10 --setenv=CUDA_VISIBLE_DEVICES=0 --setenv=LD_LIBRARY_PATH=/var/home/annunimas-server/inference-assessment/b11430-cuda/llama-b11430:/var/home/annunimas-server/inference-assessment/b11430-cuda/cudart-llama-b11430-bin-ubuntu-cuda-12.8-x64 /var/home/annunimas-server/inference-assessment/b11430-cuda/llama-b11430/llama-server --model /var/home/annunimas-server/.local/share/inference-assessment/mimo9b-2ed8dfba/MiMo-V2.6-Distill-Qwen-9B.Q4_K_M.gguf --alias mimo9b-q4km --host 127.0.0.1 --port 18099 --ctx-size 73728 --parallel 1 --n-gpu-layers 24 --split-mode none --fit off --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0 --batch-size 256 --ubatch-size 128 --jinja --reasoning auto --reasoning-budget 256'),flush=True)
    tunnel=subprocess.Popen(ssh[0:5]+['-N','-o','ExitOnForwardFailure=yes','-L','127.0.0.1:18109:127.0.0.1:18099',ssh[-1]])
    result['candidate_health']=health('http://127.0.0.1:18109/health',seconds=180)
    with urllib.request.urlopen('http://127.0.0.1:18109/props',timeout=10) as response:result['props']=json.load(response)
    assert result['props']['default_generation_settings']['n_ctx']==73728, 'Actual context mismatch'
    result['template_sha256']=hashlib.sha256(result['props']['chat_template'].encode()).hexdigest()
    assert result['template_sha256']=='9f36ab8f70ce878872cf86a0e278336f3090302a52154d8f0313196706e3bb4f', 'Embedded template mismatch'
    with (b/'mimo-live-stream-probe.log').open('w') as log:
        child=subprocess.run(['python3',str(b/'mimo-live-stream-probe.py')],stdout=log,stderr=subprocess.STDOUT,timeout=230)
    probe=json.loads((b/'mimo-live-stream-result.json').read_text())
    result['cases']=[{'mechanical_pass':probe['passed'],'exit_code':child.returncode}]

except Exception as exc:
    result['error']=str(exc)
    print('BATCH ERROR',exc,flush=True)
finally:
    try:
        result['candidate_log']=remote('journalctl --user -u inference-assessment-mimo-stream.service --no-pager -n 150')
    except Exception as exc:result['log_error']=str(exc)
    try:
        result['restore']=remote('systemctl --user stop inference-assessment-mimo-stream.service; systemctl --user start llama-server-bonsai-q1.service && systemctl --user stop inference-assessment-mimo-stream-restore.timer')
        # Use the known mesh addresses for read-back; no production tunnel needed.
        result['health_after']={str(port):health(f'http://100.102.250.115:{port}/health') for port in [8097,8098]}
        result['core_health_after']=health('http://127.0.0.1:9337/health')
    except Exception as exc:result['restore_error']=str(exc)
    if tunnel is not None:
        tunnel.terminate()
        try:tunnel.wait(timeout=10)
        except subprocess.TimeoutExpired:tunnel.kill();tunnel.wait()
    (b/'mimo-live-stream-batch.json').write_text(json.dumps(result,indent=2))
    print(json.dumps({'finished':True,'cases':len(result['cases']),'health_after':result.get('health_after'),'error':result.get('error'),'restore_error':result.get('restore_error')}),flush=True)

raise SystemExit(0 if len(result['cases'])==1 and all(r['mechanical_pass'] for r in result['cases']) and not result.get('error') and not result.get('restore_error') else 1)
