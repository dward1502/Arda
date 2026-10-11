import os
os.environ['HF_HUB_OFFLINE']='1'
os.environ['OMP_NUM_THREADS']='4'
import json,pathlib,hashlib,time,resource
from faster_whisper import WhisperModel
B=pathlib.Path('/var/home/mythos/Eregion/Arda/docs/audits/inference-assessment-2026-10-09')
prior=json.loads((B/'local-whisper-reviewed-result.json').read_text());audio=pathlib.Path(prior['audio_directory']);modelpath=pathlib.Path('/var/home/mythos/.hermes/cache/scratch/whisper-turbo-0a363e91')
manifest=json.loads((modelpath/'manifest.json').read_text())
for f in manifest['files']:
 with (modelpath/f['name']).open('rb') as stream:assert hashlib.file_digest(stream,'sha256').hexdigest()==f['sha256']
t=time.monotonic();m=WhisperModel(str(modelpath),device='cpu',compute_type='int8',cpu_threads=4,num_workers=1,local_files_only=True)
r={'scope':'fresh offline CPU control on retained synthetic fixtures; not operator acceptance','model_manifest':manifest,'load_seconds':time.monotonic()-t,'cases':[]}
for name in ['speech','silence']:
 p=audio/(name+'.wav')
 with p.open('rb') as f:sha=hashlib.file_digest(f,'sha256').hexdigest()
 assert sha==prior['audio_sha256'][name]
 t=time.monotonic();segments,info=m.transcribe(str(p),language='en',beam_size=1,temperature=0,condition_on_previous_text=False,vad_filter=True)
 text=' '.join(s.text.strip() for s in segments)
 r['cases'].append({'audio':name,'audio_sha256':sha,'text':text,'audio_seconds':info.duration,'elapsed_s':time.monotonic()-t,'vad':True})
r['peak_rss_kib']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
(B/'whisper-offline-control-result.json').write_text(json.dumps(r,indent=2));print(json.dumps(r['cases']))
