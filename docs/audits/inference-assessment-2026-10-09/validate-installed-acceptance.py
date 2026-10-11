"""Offline strict event validation; semantic acceptance remains separate."""
import copy,json
from collections import defaultdict,deque
from pathlib import Path

def validate(d, sources):
    root=Path(d['root']); pending=defaultdict(deque); reads={};missing=None;written=None
    for e in d['events']:
        if e['type']=='tool_use':pending[e['name']].append(e)
        elif e['type']=='tool_result':
            assert pending[e['name']], 'unlinked result'
            call=pending[e['name']].popleft();args=call['input']
            assert e['timestamp']>=call['timestamp']
            if call['name'] not in ('read_file','write_file'):
                assert not e.get('is_error')
                continue
            out=json.loads(e['output']) if isinstance(e['output'],str) else e['output']
            if call['name']=='read_file':
                path=Path(args['path'])
                if path==root/'latest-monitoring.json':
                    assert e.get('is_error') and out.get('not_found') and str(path) in out.get('error','')
                    missing=e['timestamp']
                elif path.parent==root and path.name in sources:
                    assert not e.get('is_error') and not out.get('not_found') and not out.get('truncated')
                    actual='\n'.join(x.split('|',1)[1] for x in out['content'].splitlines())
                    assert actual.strip()==sources[path.name].strip(), 'read content mismatch'
                    reads[path.name]=e['timestamp']
            elif call['name']=='write_file':
                assert Path(args['path'])==root/'allocation-report.txt'
                assert not e.get('is_error') and out.get('verified') is True
                assert args['content']==d['report'] and len(d['report'].strip())>0
                assert missing is not None and set(sources)<=set(reads)
                assert max(reads.values())<=call['timestamp']
                written=e['timestamp']
    assert not any(pending.values()), 'unanswered calls'
    assert missing is not None and written is not None
    assert missing<=reads['index.txt']<=reads['endpoint-packet.json']<=written
    assert d['exit_code']==0 and d['final']['exit_code']==0 and not d['final'].get('error')
    assert d['final']['timestamp']>=written
    assert d['checks']['sources_preserved'] and d['checks']['missing_not_created']
    return True

if __name__=='__main__':
    base=Path(__file__).parent; aggregate=json.loads((base/'installed-operating-acceptance.json').read_text());d=aggregate['hermes'];sources=aggregate['sources']
    assert validate(d,sources)
    rejected=[]
    for name in ('missing-recovery','failed-read','failed-write','wrong-artifact','missing-final'):
        bad=copy.deepcopy(d)
        if name=='missing-recovery':bad['events']=[e for e in bad['events'] if not(e['type']=='tool_result' and e.get('is_error'))]
        if name=='failed-read':next(e for e in bad['events'] if e['type']=='tool_result' and not e.get('is_error'))['is_error']=True
        if name=='failed-write':next(e for e in bad['events'] if e['type']=='tool_result' and e['name']=='write_file')['is_error']=True
        if name=='wrong-artifact':bad['report']='invented report'
        if name=='missing-final':bad['final']['exit_code']=1
        try:validate(bad,sources)
        except (AssertionError,KeyError,ValueError):rejected.append(name)
        else:raise AssertionError('negative control accepted: '+name)
    print(json.dumps({'actual_pass':True,'negative_controls_rejected':rejected,'semantic_review_required':True}))
