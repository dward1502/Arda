import os, json, subprocess, collections, pathlib, datetime, csv
root=pathlib.Path(__file__).resolve().parents[2]
out=root/'docs/architecture-audit-2026-10-01'
out.mkdir(exist_ok=True)
paths=set(subprocess.check_output(['git','ls-files','-z'],cwd=root).decode().split('\0'))
tracked=set(paths)
paths.update(subprocess.check_output(['git','ls-files','--others','--exclude-standard','-z'],cwd=root).decode().split('\0'))
paths.discard('')
source_ext={'.rs','.py','.ts','.tsx','.js','.jsx','.sh','.bash','.go','.c','.h','.cpp','.svelte'}
rows=[]
for p in sorted(paths):
 if p.startswith('docs/architecture-audit-2026-10-01/'): continue
 f=root/p
 sensitive=any(x in p.lower() for x in ('.env','credential','secret','private_key')) or f.suffix in {'.pem','.key'}
 try: size=f.lstat().st_size
 except FileNotFoundError: size=None
 ext=f.suffix
 lines=None
 if not sensitive and not f.is_symlink() and size is not None and ext in source_ext:
  try: lines=sum(1 for _ in f.open('rb'))
  except OSError: pass
 parts=p.split('/')
 role=('vendor' if parts[0]=='vendor' else 'runtime_or_personal_data' if parts[0] in {'data','core','human','logs','metrics'} else 'documentation' if ext=='.md' or parts[0]=='docs' else 'source' if ext in source_ext else 'config_asset_or_other')
 rows.append(dict(path=p,tracked=p in tracked,bytes=size,lines=lines,kind=role,sensitive_content_not_read=sensitive,symlink=f.is_symlink()))
with (out/'file-inventory.csv').open('w') as f:
 w=csv.DictWriter(f,fieldnames=rows[0].keys());w.writeheader();w.writerows(rows)
meta=json.loads(subprocess.check_output(['cargo','metadata','--no-deps','--format-version','1','--locked','--offline'],cwd=root,text=True))
names={p['name'] for p in meta['packages'] if p['id'] in meta['workspace_members']}
packages=[]
for p in meta['packages']:
 if p['id'] not in meta['workspace_members']:continue
 base=pathlib.Path(p['manifest_path']).parent
 prefix=str(base.relative_to(root))
 own=[r for r in rows if (r['path'].startswith(prefix+'/') if prefix!='.' else r['path'].startswith(('src/','tests/')))]
 packages.append(dict(name=p['name'],path=prefix,targets=[dict(name=t['name'],kind=t['kind'],path=str(pathlib.Path(t['src_path']).relative_to(root)),required_features=t.get('required-features',[])) for t in p['targets']],features=p['features'],internal_dependencies=[dict(name=d['name'],kind=d['kind'] or 'normal',optional=d['optional'],features=d['features'],uses_default_features=d['uses_default_features']) for d in p['dependencies'] if d['name'] in names],source_files=sum(r['lines'] is not None for r in own),source_lines=sum(r['lines'] or 0 for r in own)))
(out/'workspace-map.json').write_text(json.dumps(packages,indent=2)+'\n')
top=[]
for p in sorted(root.iterdir()):
 if p.name=='.git':continue
 rr=[r for r in rows if r['path']==p.name or r['path'].startswith(p.name+'/')]
 top.append(dict(path=p.name,type='symlink' if p.is_symlink() else 'directory' if p.is_dir() else 'file',inventoried_files=len(rr),tracked_files=sum(r['tracked'] for r in rr),source_lines=sum(r['lines'] or 0 for r in rr),inventoried_bytes=sum(r['bytes'] or 0 for r in rr)))
stats=dict(captured=datetime.datetime.now().astimezone().isoformat(),head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),files=len(rows),tracked=sum(r['tracked'] for r in rows),source_files=sum(r['lines'] is not None and r['kind']!='vendor' for r in rows),source_lines=sum(r['lines'] or 0 for r in rows if r['kind']!='vendor'),packages=len(packages),top_level=top,largest_source_files=sorted([r for r in rows if r['lines'] is not None and r['kind']!='vendor'],key=lambda r:r['lines'],reverse=True)[:60])
(out/'inventory-summary.json').write_text(json.dumps(stats,indent=2)+'\n')
print(json.dumps(stats,indent=2))
print('PACKAGES',json.dumps([{k:p[k] for k in ('name','path','source_files','source_lines','internal_dependencies')} for p in packages],indent=2))
