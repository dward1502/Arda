// Offline qualification test, not a production parser modification.
#include "chat-auto-parser.h"
#include "chat-peg-parser.h"
#include "chat.h"
#include <fstream>
#include <iostream>
#include <iterator>
int main(int argc, char ** argv) {
    if (argc != 2) return 2;
    std::ifstream file(argv[1]);
    std::string text((std::istreambuf_iterator<char>(file)), {});
    if (text.empty()) return 2;
    common_chat_template tmpl(text, "", "");
    autoparser::autoparser analysis;
    analysis.analyze_template(tmpl);
    autoparser::generation_params inputs;
    inputs.tools = common_json::parse(R"([{"type":"function","function":{"name":"lookup","parameters":{"type":"object","properties":{"path":{"type":"string"},"offset":{"type":"integer"}},"required":["path","offset"]}}}])");
    inputs.reasoning_format = COMMON_REASONING_FORMAT_NONE;
    auto parser = analysis.build_parser(inputs, "");
    const std::string path = "<parameter=path>alpha.txt</parameter>";
    const std::string offset = "<parameter=offset>17</parameter>";
    common_json rows = common_json::array();
    bool all = true;
    for (bool reversed : {false, true}) {
        std::string output = "<tool_call><function=lookup>" + (reversed ? offset+path : path+offset) + "</function></tool_call>";
        common_peg_parse_context ctx(output);
        auto result = parser.parse(ctx);
        common_chat_msg msg;
        if (result.success()) common_chat_peg_mapper(msg).from_ast(ctx.ast, result);
        bool pass = msg.tool_calls.size() == 1 && msg.tool_calls[0].name == "lookup";
        common_json args;
        if (pass) {
            args = common_json::parse(msg.tool_calls[0].arguments);
            pass = args == common_json::parse(R"({"path":"alpha.txt","offset":17})");
        }
        rows.push_back({{"reversed",reversed},{"parse_success",result.success()},{"native_calls",msg.tool_calls.size()},{"arguments",args},{"pass",pass}});
        all = all && pass;
    }
    std::cout << rows.dump(2) << '\n';
    return all ? 0 : 1;
}
