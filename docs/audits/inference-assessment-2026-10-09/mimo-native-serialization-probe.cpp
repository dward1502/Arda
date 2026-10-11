// Offline native parser/diff qualification, not server or model execution.
#include "chat-auto-parser.h"
#include "chat-peg-parser.h"
#include "chat.h"
#include "server-task.h"
#include <fstream>
#include <iostream>
#include <iterator>
using J=common_json;
J pack(const common_chat_msg &m){J a=J::array();for(auto &c:m.tool_calls)a.push_back({{"name",c.name},{"arguments",c.arguments},{"id",c.id}});return {{"content",m.content},{"reasoning",m.reasoning_content},{"calls",a}};}
int main(int argc,char**argv){
 if(argc!=3)return 2;std::ifstream f(argv[1]);std::string text((std::istreambuf_iterator<char>(f)),{});if(text.empty())return 2;
 common_chat_template tmpl(text,"","");autoparser::generation_params in;
 in.tools=J::parse(R"([{"type":"function","function":{"name":"lookup","parameters":{"type":"object","properties":{"path":{"type":"object","properties":{"name":{"type":"string"},"flags":{"type":"array","items":{"type":"boolean"}}},"required":["name","flags"]},"offset":{"type":"integer"}},"required":["path","offset"]}}}])");
 in.messages=J::parse(R"([{"role":"user","content":"Look up alpha.txt at offset 17."}])");
 in.reasoning_format=COMMON_REASONING_FORMAT_AUTO;in.stream=true;in.add_generation_prompt=true;in.enable_thinking=true;
 auto cp=autoparser::peg_generator::generate_parser(tmpl,in);common_chat_parser_params params(cp);params.reasoning_format=in.reasoning_format;params.parser.load(cp.parser);

 std::ifstream jf(argv[2]);std::string fixture_text((std::istreambuf_iterator<char>(jf)),{});J fixture=J::parse(fixture_text); J out=J::array();
 for(auto &c:fixture["cases"]){std::string input=c["input"];
  for(size_t chunk:{size_t(1),size_t(7),input.size()}){
   task_result_state state(params);J events=J::array();std::string error;size_t consumed=0;bool finalized=false;
   try{
    int count=0;
    for(size_t pos=0;pos<input.size();pos+=chunk){
     server_task_result_cmpl_partial part{};
     part.content=input.substr(pos,chunk);part.n_decoded=++count;
     part.n_prompt_tokens=1;part.n_prompt_tokens_cache=0;part.post_sampling_probs=false;
     part.res_type=TASK_RESPONSE_TYPE_OAI_CHAT;part.oaicompat_model="offline-fixture";part.oaicompat_cmpl_id="fixture-completion";
     part.update(state);consumed+=part.content.size();events.push_back({{"bytes",consumed},{"final",false},{"json",part.to_json()}});
    }
    server_task_result_cmpl_final final{};final.stream=true;final.include_usage=false;final.stop=STOP_TYPE_EOS;
    final.res_type=TASK_RESPONSE_TYPE_OAI_CHAT;final.oaicompat_model="offline-fixture";final.oaicompat_cmpl_id="fixture-completion";
    final.update(state);events.push_back({{"bytes",consumed},{"final",true},{"json",final.to_json()}});finalized=true;
   }catch(const std::exception&e){error=e.what();}
   out.push_back({{"reversed",c["reversed"]},{"chunk_bytes",chunk},{"consumed",consumed},{"finalized",finalized},{"error",error},{"events",events},{"state",pack(state.chat_msg)}});
  }
  task_result_state state(params);J row={{"reversed",c["reversed"]},{"mode","nonstream"}};
  try{server_task_result_cmpl_final final{};final.content=input;final.stream=false;final.include_usage=false;final.stop=STOP_TYPE_EOS;final.res_type=TASK_RESPONSE_TYPE_OAI_CHAT;final.oaicompat_model="offline-fixture";final.oaicompat_cmpl_id="fixture-completion";final.update(state);row["json"]=final.to_json();row["error"]="";}catch(const std::exception&e){row["error"]=e.what();}out.push_back(row);
 }
 std::cout<<out.dump(2)<<'\n';return 0;
}
