#!/usr/bin/env python3
"""Bounded whole-tree inventory and advisory hygiene. Never executes repository code."""
from __future__ import annotations
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import stat
import time
from datetime import datetime, timezone
from urllib.parse import unquote

from rumil_markdown_link_check import LINK_RE, is_external, link_target_exists

# Inventory these trees, but never interpret dependency/build/private content.
OPAQUE = {'.git', 'target', '.target-local', 'node_modules', 'vendor', 'dist', 'build',
          '.venv', 'venv', '__pycache__', '.cache', '.hermes', '.agents', '.claude', '.opencode'}
SECRET = {'.env', 'auth.json', 'credentials.json', 'id_rsa', 'id_ed25519'}
MAX_CONTENT = 1024 * 1024
SOURCE_SUFFIXES = {'.rs', '.ts', '.tsx', '.js', '.jsx', '.py', '.sh', '.toml', '.yaml', '.yml', '.json', '.md'}
POLICY = {'destructive_actions_performed': False, 'execution_performed': False,
          'queue_mutation_performed': False, 'repair_requires': 'operator approval and Engine/core governed execution'}


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def content_policy(rel):
    parts = Path(rel).parts
    if any(p in OPAQUE for p in parts): return 'generated_dependency_or_private'
    if any(p in SECRET or p.startswith('.env.') or p.endswith(('.pem', '.key', '.p12')) for p in parts):
        return 'sensitive_metadata_only'
    if parts[0] in {'data', 'logs', '.tmp', 'tmp', 'audit', 'metrics'} or rel.startswith('core/state/'):
        return 'runtime_metadata_only'
    if 'archive' in parts: return 'historical_metadata_only'
    return 'authored'


def read_bounded(path):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as f:
        if not stat.S_ISREG(os.fstat(f.fileno()).st_mode): raise OSError('not regular')
        raw = f.read(MAX_CONTENT + 1)
    if len(raw) > MAX_CONTENT: return None
    return raw.decode('utf-8')


def scan(root, max_files, seconds):
    started = time.monotonic()
    coverage = {'files': 0, 'bytes': 0, 'directories': 0, 'symlinks': 0, 'special_files': 0,
                'roots': {}, 'content_policy_counts': {}, 'errors': [], 'complete': True,
                'self_output_excluded': 'data/rumil/hygiene', 'checks': {'markdown': 0, 'python': 0, 'json': 0}}
    findings = {}; names = {}; pending_links = []
    def add(kind, rel, detail='', **extra):
        identity = digest([kind, rel, detail])
        findings[identity] = {'id': identity, 'kind': kind, 'path': rel, 'detail': detail, **extra}
    def error(err):
        coverage['complete'] = False
        coverage['errors'].append({'kind': type(err).__name__, 'path': str(getattr(err, 'filename', '') or '')})
    stop = False
    for current, dirs, files in os.walk(root, followlinks=False, onerror=error):
        coverage['directories'] += 1
        dirs.sort(); files.sort()
        for name in list(dirs):
            p = Path(current)/name; rel=p.relative_to(root).as_posix()
            if rel == 'data/rumil/hygiene': dirs.remove(name); continue
            if p.is_symlink():
                coverage['symlinks'] += 1; dirs.remove(name)
                coverage['roots'].setdefault(rel.split('/')[0], {'files':0,'bytes':0})
        for name in files:
            if coverage['files'] >= max_files or time.monotonic()-started > seconds:
                coverage['complete']=False; coverage['errors'].append({'kind':'budget_exhausted','path':''}); stop=True; break
            p=Path(current)/name; rel=p.relative_to(root).as_posix()
            try:
                info=p.lstat()
                if stat.S_ISLNK(info.st_mode): coverage['symlinks']+=1; continue
                if not stat.S_ISREG(info.st_mode): coverage['special_files']+=1; continue
                coverage['files']+=1; coverage['bytes']+=info.st_size
                group=coverage['roots'].setdefault(rel.split('/')[0], {'files':0,'bytes':0})
                group['files']+=1; group['bytes']+=info.st_size
                policy=content_policy(rel)
                counts=coverage['content_policy_counts']; counts[policy]=counts.get(policy,0)+1
                if policy in {'authored','historical_metadata_only'}: names.setdefault(name,[]).append(rel)
                if policy == 'authored' and (name.endswith(('.bak','.orig','~'))):
                    add('backup_review',rel,approval_required=True)
                if rel.startswith(('.tmp/','tmp/')):
                    # One stable directory-level recommendation, not one alert per file.
                    add('temporary_storage_review', rel.split('/')[0], approval_required=True)
                if policy != 'authored' or p.suffix not in SOURCE_SUFFIXES: continue
                if info.st_size > MAX_CONTENT:
                    counts['oversize_content_skipped']=counts.get('oversize_content_skipped',0)+1; continue
                text=read_bounded(p)
                if text is None: continue
                if any(line.startswith(('<<<<<<< ', '>>>>>>> ')) for line in text.splitlines()):
                    add('merge_conflict',rel)
                if p.suffix=='.py':
                    coverage['checks']['python']+=1
                    try: ast.parse(text, filename=rel)
                    except SyntaxError: add('python_syntax',rel)
                elif p.suffix=='.json':
                    if name.startswith(('tsconfig', 'jsconfig')) or '.vscode' in p.parts:
                        counts['jsonc_syntax_not_checked']=counts.get('jsonc_syntax_not_checked',0)+1
                        continue
                    coverage['checks']['json']+=1
                    try: json.loads(text)
                    except json.JSONDecodeError: add('json_syntax',rel)
                elif p.suffix=='.md':
                    coverage['checks']['markdown']+=1
                    fenced=False
                    for line in text.splitlines():
                        if line.lstrip().startswith(('```','~~~')): fenced=not fenced; continue
                        if fenced: continue
                        for match in LINK_RE.finditer(line):
                            target=match.group(1).strip()
                            if is_external(target): continue
                            target=unquote(target.split(' "',1)[0].strip('<>'))
                            if not link_target_exists(p,target,root): pending_links.append((rel,target))
            except (OSError, UnicodeError) as err: error(err)
        if stop: break
    for rel,target in pending_links:
        hints=names.get(Path(target.split('#',1)[0]).name,[])
        add('broken_link',rel,target,suggested_targets=hints[:10],ambiguous=len(hints)!=1)
    return coverage, findings


def audit(root, out, max_files=500000, seconds=90, strict=False, force_report=False):
    if not root.is_dir(): raise ValueError('audit root is not a directory')
    if out.resolve() != root/'data/rumil/hygiene':
        raise ValueError('output must be an in-root nonsymlink directory')
    out.mkdir(parents=True,exist_ok=True,mode=0o700)
    if any((out/name).is_symlink() for name in ['state.sqlite3','latest.json','latest.json.tmp']):
        raise ValueError('symlinked output refused')
    db=sqlite3.connect(out/'state.sqlite3', timeout=1)
    db.execute('CREATE TABLE IF NOT EXISTS state (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL)')
    # Hold one writer lock through the bounded scan and commit to prevent overlapping cycles.
    db.execute('BEGIN IMMEDIATE')
    row=db.execute('SELECT payload FROM state WHERE id=1').fetchone()
    old=json.loads(row[0]) if row else {}
    now=datetime.now(timezone.utc); stamp=now.isoformat()
    coverage, current=scan(root,max_files,seconds)
    previous={x['id']:x for x in old.get('findings',[])}
    history={x['id']:x for x in old.get('resolved_findings',[])}
    changes={'new':[],'unchanged':[],'resolved':[],'reopened':[]}
    for fid,f in current.items():
        before=previous.get(fid) or history.get(fid)
        if fid in history:
            changes['reopened'].append(fid)
            del history[fid]
        f.update(first_seen=before['first_seen'] if before else stamp,last_seen=stamp,
                 observations=(before.get('observations',0)+1) if before else 1)
        changes['unchanged' if before else 'new'].append(fid)
    if coverage['complete']:
        changes['resolved']=sorted(set(previous)-set(current))
        for fid in changes['resolved']: history[fid]={**previous[fid],'resolved_at':stamp}
    else:
        for fid,f in previous.items():
            if fid not in current: current[fid]={**f,'observation_state':'not_rechecked'}
    growth=[]
    if coverage['complete'] and old.get('coverage',{}).get('complete'):
        for name, group in coverage['roots'].items():
            base=old['coverage']['roots'].get(name,{}).get('bytes',0); delta=group['bytes']-base
            if delta >= max(64*1024*1024, base//10): growth.append({'root':name,'growth_bytes':delta})
    proposals=[]
    for f in current.values():
        if f.get('observation_state')=='not_rechecked': continue
        # Missing links must persist; file motion is common. Never auto-retarget by basename.
        if f['kind']=='broken_link' and f['observations']<2: continue
        proposals.append({'finding_id':f['id'],'evidence_digest':digest({k:v for k,v in f.items() if k not in {'first_seen','last_seen','observations'}}),
                          'disposition':'review_required','execution_allowed':False,
                          'acceptance':'Re-audit named finding after an independently approved remedy; preserve source and runtime behavior.'})
    old_proposals={x['finding_id'] for x in old.get('proposals',[])}
    newly_actionable=sorted({x['finding_id'] for x in proposals}-old_proposals)
    last_report=old.get('last_report_at')
    weekly=not last_report or (now-datetime.fromisoformat(last_report)).total_seconds()>=7*86400
    notify=bool(force_report or weekly or changes['new'] or changes['resolved'] or changes['reopened'] or growth or newly_actionable or not coverage['complete'])
    result={'schema_version':'arda.rumil.nightly-hygiene.v1','generated_at_utc':stamp,
            'outcome':('partial' if not coverage['complete'] else 'findings' if current else 'clean'),
            'coverage':coverage,'findings':sorted(current.values(),key=lambda f:f['id']),
            'resolved_findings':list(history.values()),'changes':changes,'growth':growth,'proposals':proposals,'newly_actionable':newly_actionable,
            'policy':POLICY,'last_report_at':stamp if notify else last_report,
            'handoff':{'owner':'Engine/core','state':'awaiting_governed_review','execution_performed':False}}
    raw=json.dumps(result,sort_keys=True,indent=2)+'\n'
    db.execute('INSERT OR REPLACE INTO state VALUES (1,?)',(raw,))
    # SQLite is authority; replaceable JSON is a consumer projection.
    temp=out/'latest.json.tmp'; temp.write_text(raw); temp.replace(out/'latest.json')
    db.commit(); db.close()
    if notify:
        print(f"Rúmil nightly hygiene: {result['outcome']}; files={coverage['files']} bytes={coverage['bytes']}; "
              f"new={len(changes['new'])} resolved={len(changes['resolved'])} unchanged={len(changes['unchanged'])}; "
              f"review_candidates={len(proposals)}; growth_alerts={len(growth)}")
        print(f"Evidence: {out/'latest.json'}; no repair/deletion/execution performed.")
        for finding in result['findings'][:12]:
            print(f"- {finding['kind']}: {finding['path']} [review required]")
        if len(result['findings']) > 12:
            print(f"Remaining findings in local receipt: {len(result['findings'])-12}")
    return 2 if not coverage['complete'] else 1 if strict and current else 0


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[1])
    parser.add_argument('--max-files',type=int,default=500000)
    parser.add_argument('--seconds',type=float,default=90)
    parser.add_argument('--strict',action='store_true')
    parser.add_argument('--report',action='store_true')
    args=parser.parse_args()
    try:
        root=args.root.resolve()
        return audit(root,root/'data/rumil/hygiene',args.max_files,args.seconds,args.strict,args.report)
    except Exception as err:
        print(f'Rúmil hygiene execution error: {type(err).__name__}',file=__import__('sys').stderr)
        return 2

if __name__=='__main__': raise SystemExit(main())
