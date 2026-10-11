"""Bounded synthetic Qwen ASR control; no microphone or production writes."""
import os
for key in ('HF_HUB_OFFLINE','TRANSFORMERS_OFFLINE'):
    os.environ[key]='1'
os.environ['CUDA_VISIBLE_DEVICES']=''
for key in ('OMP_NUM_THREADS','MKL_NUM_THREADS','OPENBLAS_NUM_THREADS'):
    os.environ[key]='4'
import datetime,hashlib,json,pathlib,resource,time,wave
import torch
from qwen_asr import Qwen3ASRModel
B=pathlib.Path(__file__).parent
stage=pathlib.Path('/var/home/mythos/.hermes/cache/scratch/qwen-asr-assessment')
prior=json.loads((B/'local-whisper-reviewed-result.json').read_text())
audio=pathlib.Path(prior['audio_directory'])
def sha(p):
    with p.open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()
files=['chat_template.json','config.json','generation_config.json','merges.txt','model.safetensors','preprocessor_config.json','tokenizer_config.json','vocab.json']
manifest=[{'name':f,'bytes':(stage/'model'/f).stat().st_size,'sha256':sha(stage/'model'/f)} for f in files]
assert next(f['sha256'] for f in manifest if f['name']=='model.safetensors')=='79d6cbd4c98c7bbffe9db2edac07f56cd6637d0d5944b27f6c2b8353840323ea'
torch.set_num_threads(4)
torch.set_num_interop_threads(1)
r={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'synthetic fixtures only, not operator acceptance','model_revision':'5eb144179a02acc5e5ba31e748d22b0cf3e303b0','model_files':manifest,'runtime_lock_sha256':sha(stage/'runtime.lock'),'torch':torch.__version__,'device':'cpu','dtype':'float32','attention':'eager','max_new_tokens':128,'threads':4,'cases':[]}
out=B/'qwen-asr-offline-control-result.json'
def save():
    r['peak_rss_kib']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    out.write_text(json.dumps(r,indent=2))
save()
t=time.monotonic()
model=Qwen3ASRModel.from_pretrained(str(stage/'model'),dtype=torch.float32,device_map='cpu',attn_implementation='eager',max_inference_batch_size=1,max_new_tokens=128,forced_aligner=None,local_files_only=True)
r['load_seconds']=time.monotonic()-t
save()
for name in ['speech','silence']:
    p=audio/(name+'.wav')
    assert sha(p)==prior['audio_sha256'][name]
    with wave.open(str(p),'rb') as w:
        seconds=w.getnframes()/w.getframerate()
    assert 0 < seconds <= 10
    t=time.monotonic()
    result=model.transcribe(audio=str(p),language=None,return_time_stamps=False)
    r['cases'].append({'audio':name,'audio_sha256':sha(p),'audio_seconds':seconds,'elapsed_s':time.monotonic()-t,'results':[{'text':x.text,'language':x.language} for x in result]})
    save()
    print(json.dumps(r['cases'][-1]),flush=True)
