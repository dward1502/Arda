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
        common_chat_parser_params params;
        params.format = COMMON_CHAT_FORMAT_PEG_NATIVE;
        common_json outcomes = common_json::array();
        bool pass = true;
        for (bool partial : {false, true}) {
            try {
                auto msg = common_chat_peg_parse(parser, output, partial, params);
                common_json calls = common_json::array();
                for (const auto & call : msg.tool_calls) calls.push_back({{"name",call.name},{"arguments",call.arguments}});
                bool valid = msg.tool_calls.size() == 1 && msg.tool_calls[0].name == "lookup" && common_json::parse(msg.tool_calls[0].arguments) == common_json::parse(R"({"path":"alpha.txt","offset":17})");
                outcomes.push_back({{"partial",partial},{"calls",calls},{"pass",valid}});
                pass = pass && valid;
            } catch (const std::exception & e) {
                outcomes.push_back({{"partial",partial},{"error",e.what()},{"pass",false}});
                pass = false;
            }
        }
        rows.push_back({{"reversed",reversed},{"outcomes",outcomes},{"pass",pass}});
        all = all && pass;
    }
    std::cout << rows.dump(2) << '\n';
    return all ? 0 : 1;
}
