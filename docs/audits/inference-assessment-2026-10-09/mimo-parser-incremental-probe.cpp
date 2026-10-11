// Offline native parser/diff qualification, not server or model execution.
#include "chat-auto-parser.h"
#include "chat-peg-parser.h"
#include "chat.h"
#include <fstream>
#include <iostream>
#include <iterator>
using J=common_json;
J pack(const common_chat_msg &m){J a=J::array();for(auto &c:m.tool_calls)a.push_back({{"name",c.name},{"arguments",c.arguments},{"id",c.id}});return {{"content",m.content},{"reasoning",m.reasoning_content},{"calls",a}};}
int main(int argc,char**argv){
 if(argc!=2)return 2;std::ifstream f(argv[1]);std::string text((std::istreambuf_iterator<char>(f)),{});if(text.empty())return 2;
 common_chat_template tmpl(text,"","");autoparser::generation_params in;
 in.tools=J::parse(R"([{"type":"function","function":{"name":"lookup","parameters":{"type":"object","properties":{"path":{"type":"string"},"offset":{"type":"integer"}},"required":["path","offset"]}}}])");
 in.messages=J::parse(R"([{"role":"user","content":"Look up alpha.txt at offset 17."}])");
 in.reasoning_format=COMMON_REASONING_FORMAT_NONE;in.stream=true;in.add_generation_prompt=true;in.enable_thinking=true;
 auto cp=autoparser::peg_generator::generate_parser(tmpl,in);common_chat_parser_params params(cp);params.parser.load(cp.parser);
 J out={{"generation_prompt",cp.generation_prompt},{"inputs",{{"messages",in.messages},{"tools",in.tools},{"reasoning_format","NONE"},{"stream",true},{"add_generation_prompt",true},{"enable_thinking",true}}},{"cases",J::array()}};
 bool ordered_ok=false;
 for(bool reverse:{false,true}){
  std::string path="<parameter=path>alpha.txt</parameter>",off="<parameter=offset>17</parameter>";
  std::string input="<tool_call><function=lookup>"+(reverse?off+path:path+off)+"</function></tool_call>";
  J rows=J::array();
  for(size_t n=0;n<=input.size();++n){J row={{"bytes",n}};std::string prefix=input.substr(0,n),effective=cp.generation_prompt+prefix;
   for(bool lenient:{false,true}){J r;try{common_peg_parse_context ctx(effective,lenient?COMMON_PEG_PARSE_FLAG_LENIENT:COMMON_PEG_PARSE_FLAG_NONE);auto v=params.parser.parse(ctx);r={{"type",int(v.type)},{"start",v.start},{"end",v.end},{"success",v.success()},{"fail",v.fail()},{"need_more",v.need_more_input()}};common_chat_msg m;common_chat_peg_mapper(m).from_ast(ctx.ast,v);r["message"]=pack(m);}catch(const std::exception&e){r["error"]=e.what();}row[lenient?"lenient":"strict"]=r;}
   for(bool partial:{false,true}){try{row[partial?"wrapper_partial":"wrapper_final"]=pack(common_chat_parse(prefix,partial,params));}catch(const std::exception&e){row[partial?"wrapper_partial":"wrapper_final"]={{"error",e.what()}};}}
   rows.push_back(row);
  }
  common_chat_msg previous;std::vector<std::string> ids;std::string accumulated,arguments,content;J replay=J::array();bool failed=false;
  for(size_t n=1;n<=input.size()+1;++n){bool partial=n<=input.size();if(partial)accumulated+=input[n-1];try{auto current=common_chat_parse(accumulated,partial,params);if(!current.empty()){current.set_tool_call_ids(ids,[]{return "fixture-id";});auto diffs=common_chat_msg_diff::compute_diffs(previous,current);for(auto &d:diffs){content+=d.content_delta;if(d.tool_call_index!=std::string::npos){if(d.tool_call_index!=0)throw std::runtime_error("unexpected call index");arguments+=d.tool_call_delta.arguments;}}previous=current;}replay.push_back({{"bytes",accumulated.size()},{"partial",partial},{"message",pack(previous)},{"reconstructed_arguments",arguments},{"reconstructed_content",content}});}catch(const std::exception&e){replay.push_back({{"bytes",accumulated.size()},{"partial",partial},{"error",e.what()}});failed=true;break;}}
  bool valid=false;try{valid=!failed && previous.tool_calls.size()==1 && previous.tool_calls[0].name=="lookup" && arguments==previous.tool_calls[0].arguments && J::parse(arguments)==J::parse(R"({"path":"alpha.txt","offset":17})") && content.empty() && previous.content.empty();}catch(...){}
  if(!reverse)ordered_ok=valid;
  out["cases"].push_back({{"reversed",reverse},{"input",input},{"prefixes",rows},{"replay",replay},{"complete_reconstruction_pass",valid}});
 }
 out["ordered_pass"]=ordered_ok;std::cout<<out.dump(2)<<'\n';return ordered_ok?0:1;
}
