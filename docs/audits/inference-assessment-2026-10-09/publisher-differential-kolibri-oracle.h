// Scratch instrumentation: inject router inputs, check native operations.
#include <algorithm>
#include <cstdlib>
#include <numeric>
#include <map>
static size_t oracle_checks=0;
static size_t oracle_mismatches=0;
static std::map<std::string,size_t> stages;
static void oracle_finish(){
 bool norm=std::getenv("KOLIBRI_NORM") && std::string(std::getenv("KOLIBRI_NORM"))=="true";
 for(int layer=0;layer<2;++layer){
  for(auto stage:{"ffn_moe_logits","ffn_moe_logits_biased","ffn_moe_topk","ffn_moe_weights"}) GGML_ASSERT(stages[std::string(stage)+"-"+std::to_string(layer)]>0);
  GGML_ASSERT((stages["ffn_moe_weights_norm-"+std::to_string(layer)]>0)==norm);
 }
 for(auto & kv:stages) fprintf(stderr,"STAGE %s %zu\n",kv.first.c_str(),kv.second);
 fprintf(stderr,"ORACLE_CHECKS=%zu PUBLISHER_MISMATCHES=%zu\n",oracle_checks,oracle_mismatches);
 if(!oracle_checks || oracle_mismatches) std::_Exit(9);
}
static double oracle_logit(int i) {
 const double first[]={10,9,8,7,6,2,0};
 if (std::string(std::getenv("KOLIBRI_CASE"))=="tiny") return -30.0-i*0.1;
 return i<7?first[i]:-20.0-i*0.1;
}
static double oracle_bias(int i) {
 return i==6 ? (std::string(std::getenv("KOLIBRI_CASE"))=="promote"?20.0:1.0):0.0;
}
static bool oracle_eval(ggml_tensor * t,bool ask,void *) {
 std::string name=t->name;
 bool wanted=name.find("ffn_moe_logits-")==0 || name.find("ffn_moe_logits_biased-")==0 || name.find("ffn_moe_topk-")==0 || name.find("ffn_moe_weights-")==0 || name.find("ffn_moe_weights_norm-")==0;
 if (ask) return wanted;
 if (!wanted) return true;
 ++stages[name];
 GGML_ASSERT(t->ne[3]==1);
 GGML_ASSERT(t->type==(name.find("ffn_moe_topk-")==0?GGML_TYPE_I32:GGML_TYPE_F32));
 if(name.find("ffn_moe_topk-")==0) GGML_ASSERT(t->ne[0]==6 && t->ne[2]==1);
 if(name.find("ffn_moe_logits_biased-")==0) GGML_ASSERT(t->ne[0]==384 && t->ne[2]==1);
 if(name.find("ffn_moe_weights-")==0) GGML_ASSERT((t->ne[0]==1 && t->ne[1]==6) || (t->ne[0]==6 && t->ne[2]==1));
 if(name.find("ffn_moe_weights_norm-")==0) GGML_ASSERT((t->ne[0]==1 && t->ne[1]==6) || (t->ne[0]==6 && t->ne[2]==1));
 if (name.find("ffn_moe_logits-")==0) {
  GGML_ASSERT(t->type==GGML_TYPE_F32 && t->ne[0]==384 && ggml_is_contiguous(t));
  std::vector<float> v(ggml_nelements(t));
  for(size_t i=0;i<v.size();++i)v[i]=oracle_logit(i%384);
  ggml_backend_tensor_set(t,v.data(),0,v.size()*sizeof(float));return true;
 }
 std::vector<unsigned char> data(ggml_nbytes(t));ggml_backend_tensor_get(t,data.data(),0,data.size());
 std::vector<int> order(384);std::iota(order.begin(),order.end(),0);
 std::sort(order.begin(),order.end(),[](int a,int b){return float(oracle_logit(a))+float(oracle_bias(a))>float(oracle_logit(b))+float(oracle_bias(b));});
 double sum=0;for(int i=0;i<6;++i)sum+=1.0/(1.0+std::exp(-double(float(oracle_logit(order[i])))));
 for(int64_t z=0;z<t->ne[2];++z)for(int64_t y=0;y<t->ne[1];++y)for(int64_t x=0;x<t->ne[0];++x){
  size_t offset=x*t->nb[0]+y*t->nb[1]+z*t->nb[2];
  if(name.find("ffn_moe_topk-")==0){int32_t actual;memcpy(&actual,data.data()+offset,4);GGML_ASSERT(actual==order[x]);}
  else {float actual;memcpy(&actual,data.data()+offset,4);double expected;
   if(name.find("ffn_moe_logits_biased-")==0)expected=float(oracle_logit(x))+float(oracle_bias(x));
   else {int k=t->ne[0]==1?y:x;expected=1.0/(1.0+std::exp(-double(float(oracle_logit(order[k])))));if(name.find("weights_norm-")!=std::string::npos)expected/=(sum+1e-20);}
   if (!(std::isfinite(actual) && std::abs(actual-expected)<=1e-18+2e-5*std::abs(expected))) ++oracle_mismatches;
  }
  ++oracle_checks;
 }
 return true;
}
