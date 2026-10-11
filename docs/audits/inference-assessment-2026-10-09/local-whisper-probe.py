"""CPU-only synthetic local ASR qualification; no microphone/network."""
import os
os.environ['HF_HUB_OFFLINE']='1'
os.environ['OMP_NUM_THREADS']='4'
import json,time,pathlib,subprocess,wave,resource,importlib.metadata
import ctranslate2
from faster_whisper import WhisperModel
root=pathlib.Path('/var/home/mythos/.hermes/cache/scratch/whisper-turbo-0a363e91')
out=pathlib.Path(__file__).with_name('local-whisper-result.json')
phrase='Please research local inference, and review the code changes.'
subprocess.run(['espeak-ng','-w',str(root/'speech.wav'),phrase],check=True)
with wave.open(str(root/'silence.wav'),'wb') as f:
 f.setnchannels(1);f.setsampwidth(2);f.setframerate(16000);f.writeframes(b'\0'*16000*2*3)
assert 'int8' in ctranslate2.get_supported_compute_types('cpu')
t=time.monotonic();model=WhisperModel(str(root),device='cpu',compute_type='int8',cpu_threads=4,num_workers=1,local_files_only=True)
result={'scope':'local synthetic CPU ASR only; not operator microphone or Hermes integration','model_manifest':json.loads((root/'manifest.json').read_text()),'versions':{d.metadata['Name']:d.version for d in importlib.metadata.distributions()},'load_seconds':time.monotonic()-t,'compute_type':model.model.compute_type,'reference':phrase,'cases':[]}
for name,vad in [('speech',False),('silence',False),('silence',True)]:
 t=time.monotonic();segments,info=model.transcribe(str(root/(name+'.wav')),language='en',beam_size=1,temperature=0,condition_on_previous_text=False,vad_filter=vad)
 segments=list(segments)
 result['cases'].append({'audio':name,'vad':vad,'text':' '.join(s.text.strip() for s in segments),'seconds':time.monotonic()-t,'audio_seconds':info.duration})
 result['peak_rss_kib']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
 out.write_text(json.dumps(result,indent=2));print(json.dumps(result['cases'][-1]),flush=True)
