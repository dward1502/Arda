"""Retained NONE-mode fixture gate; not arbitrary schemas/server qualification."""
import copy,hashlib,json,pathlib
root=pathlib.Path(__file__).parent
p=root/'mimo-parser-incremental-result.json'
def verify(d):
 assert d['generation_prompt']=='<|im_start|>assistant\n'
 assert len(d['cases'])==2 and [c['reversed'] for c in d['cases']]==[False,True]
 for c in d['cases']:
  assert len(c['input'].encode())==120
  assert [r['bytes'] for r in c['prefixes']]==list(range(121))
  assert len(c['replay'])==121
  assert [r['bytes'] for r in c['replay']]==list(range(1,121))+[120]
  assert all(r['partial'] for r in c['replay'][:-1]) and not c['replay'][-1]['partial']
  for row in c['prefixes']:
   for mode in ['strict','lenient']:
    r=row[mode];assert r['type'] in (0,1,2)
    assert (r['fail'],r['success'],r['need_more'])==(r['type']==0,r['type']==1,r['type']==2)
 ordered,reverse=d['cases']
 expected='{"path":"alpha.txt","offset":17}'
 last=''
 for row in ordered['replay']:
  assert 'error' not in row
  m=row['message'];assert not m['content'] and not m['reasoning'] and not row['reconstructed_content']
  calls=m['calls'];assert len(calls)<=1
  args=calls[0]['arguments'] if calls else ''
  if calls:assert calls[0]['id']=='fixture-id' and calls[0]['name']=='lookup'
  assert args==row['reconstructed_arguments'] and args.startswith(last);last=args
 assert last==expected and ordered['complete_reconstruction_pass'] and d['ordered_pass']
 for mode in ['strict','lenient']:
  r=ordered['prefixes'][-1][mode];assert r['success'] and r['end']==142
 for row in reverse['replay'][:-1]:
  assert 'error' not in row
  m=row['message'];assert not m['content'] and not m['reasoning']
  if row['bytes']>=28:assert row['reconstructed_arguments']=='{' and m['calls'][0]['arguments']=='{'
 assert 'peg-native format' in reverse['replay'][-1]['error']
 assert not reverse['complete_reconstruction_pass']
 assert reverse['prefixes'][-1]['strict']['fail'] and reverse['prefixes'][-1]['lenient']['fail']
 assert reverse['prefixes'][-1]['wrapper_partial']['calls']==[]
d=json.loads(p.read_text());verify(d)
mutations=[lambda x:x['cases'][0]['prefixes'].pop(),lambda x:x['cases'][1]['replay'][-1].pop('error'),lambda x:x['cases'][0]['replay'][-1].update(reconstructed_arguments='{}'),lambda x:x['cases'][0]['prefixes'][-1]['strict'].update(end=141),lambda x:x['cases'][0]['replay'][-1]['message'].update(reasoning='leak')]
for mutate in mutations:
 x=copy.deepcopy(d);mutate(x)
 try:verify(x)
 except (AssertionError,KeyError):pass
 else:raise AssertionError('negative control accepted')
print(json.dumps({'pass':True,'negative_controls':len(mutations),'result_sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
