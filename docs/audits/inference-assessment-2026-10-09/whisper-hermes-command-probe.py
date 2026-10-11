"""Run installed Hermes STT dispatch with isolated config and synthetic WAVs."""
import hashlib,json,os,pathlib,subprocess,sys,tempfile,time
B=pathlib.Path(__file__).parent
ROOT=B.parents[2]
STAGE=pathlib.Path('/var/home/mythos/.local/share/arda/voice-whisper')
SCRIPT=ROOT/'adapters/voice-capture/whisper_command.py'

def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

if '--child' in sys.argv:
    from tools.transcription_tools import transcribe_audio
    baseline=json.loads((B/'local-whisper-reviewed-result.json').read_text())
    audio=pathlib.Path(baseline['audio_directory'])
    rows=[]
    for name in ['speech','silence','missing']:
        p=audio/(name+'.wav')
        if name!='missing':assert sha(p)==baseline['audio_sha256'][name]
        t=time.monotonic();result=transcribe_audio(str(p),source='voice_mode')
        rows.append({'case':name,'elapsed_s':time.monotonic()-t,'result':result})
    print(json.dumps(rows))
else:
    manifest=json.loads((STAGE/'model/manifest.json').read_text())
    for f in manifest['files']:assert sha(STAGE/'model'/f['name'])==f['sha256']
    release=STAGE/'releases'/sha(SCRIPT)/'whisper_command.py'
    assert sha(release)==sha(SCRIPT)
    command=f'/usr/bin/systemd-run --user --wait --pipe --collect --quiet -p RuntimeMaxSec=75 -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=400% -p IPAddressDeny=any --setenv=HF_HUB_OFFLINE=1 --setenv=CUDA_VISIBLE_DEVICES= {STAGE}/venv/bin/python -I {release} --input {{input_path}} --model {STAGE}/model --language {{language}}'
    with tempfile.TemporaryDirectory(prefix='whisper-hermes-', dir='/var/home/mythos/.hermes/cache/scratch') as home:
        config={'stt':{'enabled':True,'provider':'arda_whisper_trial','providers':{'arda_whisper_trial':{'type':'command','command':command,'timeout':90,'language':'en','output_format':'txt'}}}}
        pathlib.Path(home,'config.yaml').write_text(json.dumps(config))
        discovery=json.loads(subprocess.check_output(['hermes','--print-runtime-command'],text=True))
        discovery[-1]=discovery[-1].replace("runpy.run_module('hermes_cli.main', run_name='__main__', alter_sys=True)","import json; print(json.dumps({'executable':sys.executable,'path':sys.path}))")
        runtime=json.loads(subprocess.check_output(discovery,text=True).strip().splitlines()[-1])
        invocation=[runtime['executable'],'-I','-c',f"import sys,runpy; sys.path={runtime['path']!r}; sys.argv=[{str(__file__)!r}, '--child']; runpy.run_path({str(__file__)!r}, run_name='__main__')"]
        env=dict(os.environ,HERMES_HOME=home,HF_HUB_OFFLINE='1',CUDA_VISIBLE_DEVICES='')
        run=subprocess.run(invocation,env=env,text=True,capture_output=True,timeout=210)
        print(run.stdout);print(run.stderr,file=sys.stderr)
        assert run.returncode==0,run.returncode
        rows=json.loads(run.stdout.strip().splitlines()[-1])
        assert rows[0]['result']['success'] is True
        assert rows[0]['result']['transcript']=='Please research local inference and review the code changes.'
        assert rows[1]['result']['success'] is False
        assert rows[2]['result']['success'] is False
        result={'complete':True,'scope':'installed Hermes public STT dispatcher, isolated config, synthetic fixtures only','adapter_sha256':sha(SCRIPT),'model_manifest':manifest,'config':config,'exit_code':run.returncode,'cases':rows}
        (B/'whisper-hermes-command-result.json').write_text(json.dumps(result,indent=2))
