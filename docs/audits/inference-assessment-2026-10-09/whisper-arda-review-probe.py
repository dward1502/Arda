"""Exercise real Arda review envelopes on synthetic local WAVs; no microphone."""
import hashlib,json,pathlib,subprocess,sys,tempfile,time
B=pathlib.Path(__file__).parent
ROOT=B.parents[2]
STAGE=pathlib.Path('/var/home/mythos/.local/share/arda/voice-whisper')
config=ROOT/'config/adapters/voice-capture-whisper-trial.toml'
prior=json.loads((B/'local-whisper-reviewed-result.json').read_text())
source=pathlib.Path(prior['audio_directory'])
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
rows=[]
with tempfile.TemporaryDirectory(prefix='synthetic-review-',dir=STAGE/'audio-inbox') as d:
    for name in ['speech','silence','malformed']:
        # Metacharacters remain literal argv text; no shell expansion is permitted.
        p=pathlib.Path(d)/f"{name} ' $(touch SHOULD_NOT_EXIST).wav"
        data=(source/(name+'.wav')).read_bytes() if name!='malformed' else b'not a WAV'
        p.write_bytes(data)
        if name!='malformed':assert sha(p)==prior['audio_sha256'][name]
        request={'schema_version':'arda.voice-capture.request.v1','audio_path':str(p)}
        t=time.monotonic()
        run=subprocess.run([sys.executable,'-I',str(ROOT/'adapters/voice-capture/arda_adapter.py'),'--config',str(config)],input=json.dumps(request),capture_output=True,text=True,timeout=100,cwd=d)
        assert run.returncode==0
        result=json.loads(run.stdout)
        assert result['external_send_authorized'] is False
        assert result['governed_action_authorized'] is False
        assert p.exists(),'adapter unexpectedly deleted recording'
        assert not (pathlib.Path(d)/'SHOULD_NOT_EXIST').exists()
        if name=='speech':
            assert result['status']=='transcript_pending_review'
            assert result['review_state']=='operator_review_required'
            assert result['editable'] is True
            assert result['transcript']=='Please research local inference and review the code changes.'
        else:
            assert result['status']=='recoverable_inbox'
            assert result['transcript'] is None
        rows.append({'case':name,'audio_sha256':sha(p),'exit_code':run.returncode,'elapsed_s':time.monotonic()-t,'result':result})
        print(json.dumps(rows[-1]),flush=True)
r={'complete':True,'scope':'synthetic Arda adapter CLI review-envelope test, not native editable UI or operator acceptance','config_sha256':sha(config),'command_sha256':sha(ROOT/'adapters/voice-capture/whisper_command.py'),'cases':rows}
(B/'whisper-arda-review-result.json').write_text(json.dumps(r,indent=2))
