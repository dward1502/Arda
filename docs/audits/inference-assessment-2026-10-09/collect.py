import subprocess,concurrent.futures,json,pathlib,datetime
probe=r'''hostname; uname -srmo; lscpu; free -b; df -h / /home; lspci -nn; nvidia-smi --query-gpu=name,memory.total,memory.used,driver_version,pci.bus_id,power.limit --format=csv; systemctl --user list-units --type=service --state=running --no-pager; ps -eo pid,args | python3 -c 'import sys; print("".join(l for l in sys.stdin if any(s in l for s in ["llama-server", "vllm", "llmster", "sglang"]) and "python3 -c" not in l))'; ss -ltn; for s in mesh-llm llama-server-light llama-server llama-server-bonsai-q1 llama-server-nanbeige; do systemctl --user show "$s.service" -p ExecStart -p FragmentPath -p ActiveState; done
python3 - <<'PY'
import pathlib,subprocess,json,urllib.request
h=pathlib.Path.home()
for d in [h/'llama.cpp',h/'llama.cpp-prism']:
 if (d/'.git').exists():
  for args in [['rev-parse','HEAD'],['log','-1','--format=%cI %s'],['remote','-v']]:
   r=subprocess.run(['git','-C',str(d),*args],capture_output=True,text=True);print(str(d),r.stdout)
for p in [9337,1234,8097,8098]:
 try: print('MODELS',p,urllib.request.urlopen(f'http://127.0.0.1:{p}/v1/models',timeout=3).read().decode()[:5000])
 except Exception as e:print('MODELS',p,str(e))
for base in [h/'models',h/'.local/bin']:
 if base.exists():
  for p in base.rglob('*'):
   if p.is_file() and (p.suffix=='.gguf' or p.name.startswith('llama-server')):print('ARTIFACT',p,p.stat().st_size)
PY
'''
targets={'core':None,'server':'annunimas-server@100.102.250.115','beelink':'citadel@100.103.125.88','laptop':'mythos@100.97.15.88','warden':'numenor@100.110.85.37'}
def run(item):
 name,host=item
 cmd=['bash','--noprofile','--norc','-s'] if host is None else ['ssh','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=6',host,'bash --noprofile --norc -s']
 try:
  r=subprocess.run(cmd,input=probe,text=True,capture_output=True,timeout=65);return name,{'stdout':r.stdout,'stderr':r.stderr,'returncode':r.returncode}
 except Exception as e:return name,{'error':str(e)}
with concurrent.futures.ThreadPoolExecutor(max_workers=5) as ex: results=dict(ex.map(run,targets.items()))
p=pathlib.Path('/var/home/mythos/.hermes/cache/scratch/inference-fleet-live.json');p.write_text(json.dumps(results,indent=2));print(p)
for name,r in results.items():print(name,json.dumps(r)[:22000])
