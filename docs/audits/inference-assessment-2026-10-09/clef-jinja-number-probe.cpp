// Differential test only: execute the pinned native Jinja serializer.
#include <iostream>
#include "json.h"
#include "jinja/runtime.h"
#include "jinja/parser.h"
#include "jinja/lexer.h"
#include "jinja/utils.h"
int main() {
    const std::string tmpl = "{{ data | tojson(ensure_ascii=False) }}";
    const auto vars = common_json::parse("{\"data\":[1.0,-0.0,1.23456789,1e-7,1e20]}");
    jinja::lexer lexer;
    auto ast = jinja::parse_from_tokens(lexer.tokenize(tmpl));
    jinja::context ctx(tmpl);
    jinja::global_from_json(ctx, vars, true);
    jinja::runtime runtime(ctx);
    auto parts = runtime.gather_string_parts(runtime.execute(ast));
    std::string actual;
    for (const auto & part : parts->as_string().parts) actual += part.val;
    std::cout << actual << '\n';
    return common_json::parse(actual) == common_json::parse("[1.0,-0.0,1.23456789,1e-07,1e+20]") ? 0 : 1;
}
