"""Offline synthetic Parakeet control; never captures operator audio."""
import pathlib,hashlib,json,subprocess,time,datetime,re,wave
B=pathlib.Path(__file__).parent
old=json.loads((B/'local-whisper-reviewed-result.json').read_text());audio=pathlib.Path(old['audio_directory'])
cli=pathlib.Path('/var/home/mythos/parakeet.cpp/build/examples/cli/parakeet-cli');model=pathlib.Path('/var/home/mythos/models/annunimas-staging/current/node-core-hub/parakeet/realtime_eou_120m-v1-q4_k.gguf')
def sha(p):return hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
r={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'synthetic speech and silence only, no microphone or production changes','cli_sha256':sha(cli),'model_sha256':sha(model),'model_path':str(model),'cases':[]}
for name in ['speech','silence']:
 p=audio/(name+'.wav');assert sha(p)==old['audio_sha256'][name]
 with wave.open(str(p),'rb') as w:duration=w.getnframes()/w.getframerate()
 cmd=['systemd-run','--user','--wait','--pipe','--collect','--unit=assessment-parakeet-'+name,'-p','RuntimeMaxSec=180','-p','MemoryMax=8G','-p','MemorySwapMax=0','-p','CPUQuota=400%','-p','IPAddressDeny=any','--setenv=CUDA_VISIBLE_DEVICES=','--setenv=GGML_CUDA_VISIBLE_DEVICES=','--setenv=HF_HUB_OFFLINE=1',str(cli),'transcribe','--model',str(model),'--input',str(p),'--threads','4','--json']
 t=time.monotonic()
 try:
  result=subprocess.run(cmd,capture_output=True,text=True,timeout=200);row={'exit':result.returncode,'stdout':result.stdout,'stderr':result.stderr}
 except subprocess.TimeoutExpired:row={'timeout':True}
 row.update(audio=name,audio_sha256=sha(p),audio_seconds=duration,elapsed_s=time.monotonic()-t,command=cmd)
 r['cases'].append(row)
 (B/'parakeet-offline-control-result.json').write_text(json.dumps(r,indent=2));print(json.dumps(row),flush=True)
