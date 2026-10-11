"""Validate native JSON output, not socket/SSE delivery or model quality."""
import copy,hashlib,json,pathlib
root=pathlib.Path(__file__).parent
p=root/'mimo-native-serialization-result.json'
expected='{"path":{"name":"alpha.txt","flags":[true,false]},"offset":17}'
def verify(rows):
 assert len(rows)==8
 assert {(r['reversed'],r.get('chunk_bytes',r.get('mode'))) for r in rows}=={(rev,k) for rev in [False,True] for k in [1,7,194,'nonstream']}
 summaries=[]
 for row in rows:
  rev=row['reversed']
  if 'events' not in row:
   if rev:assert 'peg-native format' in row['error'] and 'json' not in row
   else:
    assert not row['error'];assert row['json']['model']=='offline-fixture';c=row['json']['choices'][0];m=c['message'];assert c['finish_reason']=='tool_calls'
    assert m['content']=='' and m['reasoning_content']=='Inspect the requested file.'
    assert len(m['tool_calls'])==1;call=m['tool_calls'][0];assert call['id'] and call['function']=={'name':'lookup','arguments':expected}
   continue
  assert row['consumed']==194
  chunks=row['chunk_bytes'];positions=list(range(chunks,194,chunks))+[194]
  assert [e['bytes'] for e in row['events'] if not e['final']]==positions
  args=name=identity=reasoning=content='';finishes=[]
  for event in row['events']:
   for packet in event['json']:
    assert packet['object']=='chat.completion.chunk' and packet['id']=='fixture-completion' and packet['model']=='offline-fixture'
    c=packet['choices'][0];assert c['index']==0;d=c['delta']
    content+=d.get('content') or '';reasoning+=d.get('reasoning_content') or ''
    for t in d.get('tool_calls',[]):
     assert t['index']==0
     if 'id' in t:
      assert t['id'] and (not identity or identity==t['id']);identity=t['id']
     name+=t['function'].get('name','');args+=t['function'].get('arguments','')
    if c['finish_reason'] is not None:finishes.append(c['finish_reason'])
  assert not content
  if not rev:
   assert row['finalized'] and not row['error'] and finishes==['tool_calls']
   assert identity and name=='lookup' and args==expected and reasoning=='Inspect the requested file.'
   assert row['state']['calls'][0]['id']==identity and row['state']['calls'][0]['arguments']==args
  else:
   assert not row['finalized'] and 'peg-native format' in row['error'] and not finishes
   assert args==('{' if chunks in (1,7) else '')
   assert reasoning==('Inspect the requested file.' if chunks in (1,7) else '')
   assert row['state']['reasoning']==reasoning
  summaries.append({'reversed':rev,'chunk_bytes':chunks,'serialized_arguments':args,'finish_reasons':finishes})
 return summaries
rows=json.loads(p.read_text());summary=verify(rows)
mutations=[lambda x:x.pop(),lambda x:x[0]['events'].pop(),lambda x:x[4].update(error=''),lambda x:x[0].update(finalized=False)]
mutations.extend([lambda x:x[0]['events'][0]['json'][0].update(model='wrong'), lambda x:x[3]['json'].update(model='wrong'), lambda x:x[4]['events'][0]['json'][0]['choices'][0]['delta'].update(reasoning_content='corrupt')])
for fn in mutations:
 bad=copy.deepcopy(rows);fn(bad)
 try:verify(bad)
 except (AssertionError,KeyError):pass
 else:raise AssertionError('corrupted result accepted')
print(json.dumps({'passed':True,'negative_controls':len(mutations),'result_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'summary':summary},indent=2))
