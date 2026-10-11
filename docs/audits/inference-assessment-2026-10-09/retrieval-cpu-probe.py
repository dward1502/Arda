"""Frozen development retrieval comparison; not production or held-out acceptance."""
import os
os.environ['HF_HUB_OFFLINE']='1'
import gc,hashlib,importlib.metadata,json,math,pathlib,resource,time
import torch
from transformers import AutoTokenizer,AutoModel,AutoModelForCausalLM

def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for c in iter(lambda:f.read(1048576),b''):h.update(c)
 return h.hexdigest()
def metrics(rank,expected):
 assert len(rank)==len(set(rank)) and expected
 rel=set(expected);g=[int(x in rel) for x in rank[:3]]
 return {'hit1':float(bool(g and g[0])),'recall3':sum(g)/len(rel),'mrr3':next((1/(i+1) for i,v in enumerate(g) if v),0),'ndcg3':sum(v/math.log2(i+2) for i,v in enumerate(g))/sum(1/math.log2(i+2) for i in range(min(3,len(rel))))}
assert metrics(['x','a'],['a'])['mrr3']==0.5
assert metrics(['a','b'],['a','b'])['ndcg3']==1
assert metrics(['x'],['a'])['recall3']==0
root=pathlib.Path('/var/home/mythos/Eregion/Arda');scratch=pathlib.Path('/var/home/mythos/.hermes/cache/scratch')
fixture=root/'crates/spine/executors/arda-varda/fixtures/reliability_questions_v1.json';baseline=root/'target/qualification/r07-20261008/after-reviewed.json'
f=json.loads(fixture.read_text());base=json.loads(baseline.read_text());assert sha(fixture)==base['fixture_sha256']
docs={d['id']:d['content'] for d in f['documents']}
for d in f['documents']:assert hashlib.sha256(d['content'].encode()).hexdigest()==d['content_sha256']
cases={c['query']:c for c in base['cases']};assert len(cases)==len(f['queries'])==24 and len(docs)==12
out=pathlib.Path(__file__).with_name('retrieval-cpu-result.json')
result={'completed':False,'scope':'agent-authored development labels, frozen corpus; historical BM25 ranking, no contemporaneous baseline timing','fixture_sha256':sha(fixture),'baseline_sha256':sha(baseline),'probe_sha256':sha(pathlib.Path(__file__)),'versions':{d.metadata['Name']:d.version for d in importlib.metadata.distributions()},'models':{},'cases':[]}
def save():out.write_text(json.dumps(result,indent=2))
torch.set_num_threads(4)
instruction='Given a web search query, retrieve relevant passages that answer the query'
def load(name,cls):
 path=scratch/name;m=json.loads((path/'manifest.json').read_text())
 for item in m['files']:assert sha(path/item['name'])==item['sha256']
 t=time.monotonic();tok=AutoTokenizer.from_pretrained(path,local_files_only=True,padding_side='left');model=cls.from_pretrained(path,local_files_only=True,torch_dtype=torch.float32).eval()
 result['models'][name]={'manifest':m,'load_seconds':time.monotonic()-t};return tok,model
prefix='<|im_start|>system\nJudge whether the Document meets the requirements based on the Query and the Instruct provided. Note that the answer can only be "yes" or "no".<|im_end|>\n<|im_start|>user\n'
suffix='<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n'
tok,model=load('Qwen3-Reranker-0.6B-e61197ed',AutoModelForCausalLM)
assert tok.convert_tokens_to_ids('yes')==9693 and tok.convert_tokens_to_ids('no')==2152
with torch.inference_mode():
 for q in f['queries']:
  candidates=cases[q['query']]['ranked_document_ids'];assert 0 < len(candidates) <= 3 and len(set(candidates)) == len(candidates)
  scored=[]
  for id in candidates:
   t=time.monotonic();body=f"<Instruct>: {instruction}\n<Query>: {q['query']}\n<Document>: {docs[id]}"
   ids=tok.encode(prefix,add_special_tokens=False)+tok.encode(body,add_special_tokens=False)+tok.encode(suffix,add_special_tokens=False)
   assert len(ids)<=2048,'input exceeds declared ceiling'
   x=torch.tensor([ids]);logits=model(input_ids=x,attention_mask=torch.ones_like(x)).logits[0,-1]
   no,yes=logits[2152].item(),logits[9693].item();score=torch.sigmoid(logits[9693]-logits[2152]).item()
   assert math.isfinite(score)
   scored.append({'id':id,'no':no,'yes':yes,'score':score,'tokens':len(ids),'seconds':time.monotonic()-t})
  ranked=[x['id'] for x in sorted(scored,key=lambda x:-x['score'])]
  result['cases'].append({'query':q['query'],'expected':q['expected_document_ids'],'bm25':candidates,'reranked':ranked,'pair_scores':scored});save();print('reranked',len(result['cases']),flush=True)
del model;gc.collect()
tok,model=load('Qwen3-Embedding-0.6B-97b0c614',AutoModel)
def embed(text):
 x=tok(text,return_tensors='pt',truncation=False);assert x['input_ids'].shape[1]<=2048
 with torch.inference_mode():v=model(**x).last_hidden_state[:,-1,:]
 assert v.shape[1]==1024
 return torch.nn.functional.normalize(v,p=2,dim=1)[0]
t=time.monotonic();ids=list(docs);vectors=torch.stack([embed(docs[id]) for id in ids]);result['document_encoding_seconds']=time.monotonic()-t
for c in result['cases']:
 t=time.monotonic();v=embed(f"Instruct: {instruction}\nQuery:{c['query']}");scores=(vectors@v).tolist();c['embedding_seconds']=time.monotonic()-t;c['embedding']=[ids[i] for i in sorted(range(len(ids)),key=lambda i:-scores[i])[:3]];c['embedding_scores']=dict(zip(ids,scores));save()
result['metrics']={}
for method in ['bm25','reranked','embedding']:
 rows=[metrics(c[method],c['expected']) for c in result['cases']]
 result['metrics'][method]={key:sum(x[key] for x in rows)/len(rows) for key in rows[0]}
assert result['metrics']['bm25']['recall3']==result['metrics']['reranked']['recall3']
result['completed']=True;result['peak_rss_kib']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss;save();print(json.dumps(result['metrics']),flush=True)
