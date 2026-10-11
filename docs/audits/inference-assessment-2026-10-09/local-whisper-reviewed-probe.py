"""Bounded synthetic regression, not human speech acceptance."""
import os
os.environ['HF_HUB_OFFLINE']='1'
os.environ['OMP_NUM_THREADS']='4'
import datetime,hashlib,importlib.metadata,json,pathlib,re,resource,subprocess,tempfile,time,wave
import ctranslate2
from faster_whisper import WhisperModel

def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  for chunk in iter(lambda:f.read(1048576),b''):h.update(chunk)
 return h.hexdigest()

def words(s):return re.findall(r"[a-z0-9]+",s.lower())
def distance(a,b):
 row=list(range(len(b)+1))
 for i,x in enumerate(a,1):
  nxt=[i]
  for j,y in enumerate(b,1):nxt.append(min(nxt[-1]+1,row[j]+1,row[j-1]+(x!=y)))
  row=nxt
 return row[-1]
assert distance(['a','b'],['a','c'])==1
assert distance([],['x'])==1 and distance(['a'],[])==1
root=pathlib.Path('/var/home/mythos/.hermes/cache/scratch/whisper-turbo-0a363e91')
manifest=json.loads((root/'manifest.json').read_text())
for f in manifest['files']:
 p=root/f['name'];assert digest(p)==f['sha256'] and p.stat().st_size==f['bytes']
work=pathlib.Path(tempfile.mkdtemp(prefix='whisper-reviewed-',dir='/var/home/mythos/.hermes/cache/scratch'))
phrase='Please research local inference, and review the code changes.'
subprocess.run(['espeak-ng','-w',str(work/'speech.wav'),phrase],check=True)
with wave.open(str(work/'silence.wav'),'wb') as f:
 f.setnchannels(1);f.setsampwidth(2);f.setframerate(16000);f.writeframes(b'\0'*96000)
t=time.monotonic();model=WhisperModel(str(root),device='cpu',compute_type='int8',cpu_threads=4,num_workers=1,local_files_only=True)
result={'completed':False,'scope':'synthetic speech/VAD regression only; no human corpus, microphone or Hermes acceptance','utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'probe_sha256':digest(pathlib.Path(__file__)),'model_manifest':manifest,'audio_directory':str(work),'audio_sha256':{n:digest(work/(n+'.wav')) for n in ['speech','silence']},'espeak_version':subprocess.check_output(['espeak-ng','--version'],text=True).strip(),'versions':{d.metadata['Name']:d.version for d in importlib.metadata.distributions()},'load_seconds':time.monotonic()-t,'compute_type':model.model.compute_type,'normalization':'lowercase ASCII alphanumeric words; Levenshtein word error rate','cases':[]}
out=pathlib.Path(__file__).with_name('local-whisper-reviewed-result.json')
for name,vad in [('speech',False),('speech',True),('silence',False),('silence',True)]:
 t=time.monotonic();segments,info=model.transcribe(str(work/(name+'.wav')),language='en',beam_size=1,temperature=0,condition_on_previous_text=False,vad_filter=vad)
 text=' '.join(s.text.strip() for s in segments);elapsed=time.monotonic()-t
 ref=words(phrase) if name=='speech' else []
 errors=distance(ref,words(text));row={'audio':name,'vad':vad,'text':text,'seconds':elapsed,'audio_seconds':info.duration,'word_errors':errors,'wer':errors/len(ref) if ref else None,'false_activation':bool(text) if not ref else None}
 result['cases'].append(row);out.write_text(json.dumps(result,indent=2));print(json.dumps(row),flush=True)
result['completed']=True
result['synthetic_vad_gate']=all(c['word_errors']==0 for c in result['cases'] if c['vad'])
result['process_peak_rss_kib']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
out.write_text(json.dumps(result,indent=2))
raise SystemExit(0 if result['synthetic_vad_gate'] else 1)
