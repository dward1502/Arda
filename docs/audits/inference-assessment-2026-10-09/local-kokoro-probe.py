"""Local CPU synthesis only; no playback or intelligibility acceptance."""
import os
os.environ['HF_HUB_OFFLINE']='1'
import hashlib,importlib.metadata,json,pathlib,resource,time,wave
import numpy as np
import torch
from kokoro import KModel,KPipeline
root=pathlib.Path('/var/home/mythos/.hermes/cache/scratch/kokoro-f3ff3571');out=pathlib.Path(__file__).parent
manifest=json.loads((root/'manifest.json').read_text())
for f in manifest['files']:assert hashlib.sha256((root/f['name']).read_bytes()).hexdigest()==f['sha256']
torch.set_num_threads(4);torch.manual_seed(42)
t=time.monotonic();model=KModel(config=str(root/'config.json'),model=str(root/'kokoro-v1_0.pth')).to('cpu').eval()
pipeline=KPipeline(lang_code='a',model=model,device='cpu',trf=False)
assert pipeline.g2p.fallback is not None
load=time.monotonic()-t
phrase='Please research local inference, and review the code changes.'
t=time.monotonic()
with torch.inference_mode():parts=list(pipeline(phrase,voice=str(root/'voices/af_heart.pt'),speed=1.0))
elapsed=time.monotonic()-t;audio=np.concatenate([p.audio.cpu().numpy() for p in parts]);assert audio.size and np.isfinite(audio).all() and np.max(np.abs(audio))>0
wav=out/'kokoro-af-heart-synthetic.wav'
with wave.open(str(wav),'wb') as f:
 f.setnchannels(1);f.setsampwidth(2);f.setframerate(24000);f.writeframes((np.clip(audio,-1,1)*32767).astype('<i2').tobytes())
result={'scope':'single synthetic CPU synthesis; not listening quality or Hermes acceptance','completed':True,'manifest':manifest,'probe_sha256':hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),'versions':{d.metadata['Name']:d.version for d in importlib.metadata.distributions()},'text':phrase,'phonemes':[p.phonemes for p in parts],'fallback_enabled':True,'load_seconds':load,'synthesis_seconds':elapsed,'audio_seconds':len(audio)/24000,'peak_amplitude':float(np.max(np.abs(audio))),'clipped_samples':int(np.sum(np.abs(audio)>1)),'wav_sha256':hashlib.sha256(wav.read_bytes()).hexdigest(),'process_peak_rss_kib':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}
(out/'local-kokoro-result.json').write_text(json.dumps(result,indent=2));print(json.dumps({k:v for k,v in result.items() if k not in ['manifest','versions']}))
