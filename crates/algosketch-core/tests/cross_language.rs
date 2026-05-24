use algosketch_core::diagnostics::collect_raw_stats;
use algosketch_core::ir::*;
use algosketch_core::parser::{CppParser, JavaParser, LanguageParser, PythonParser};

fn parse_fixture(algorithm: &str, ext: &str) -> Module {
    let source = std::fs::read_to_string(format!("tests/fixtures/{algorithm}.{ext}"))
        .unwrap_or_else(|err| panic!("failed to read {algorithm}.{ext}: {err}"));

    match ext {
        "py" => PythonParser::new().parse(&source),
        "java" => JavaParser::new().parse(&source),
        "cpp" => CppParser::new().parse(&source),
        _ => panic!("unsupported fixture extension: {ext}"),
    }
    .unwrap_or_else(|err| panic!("failed to parse {algorithm}.{ext}: {err}"))
}

fn module_skeleton(module: &Module) -> Vec<String> {
    module
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Function(function) => Some(function_skeleton(function)),
            _ => None,
        })
        .flatten()
        .collect()
}

fn function_skeleton(function: &Function) -> Vec<String> {
    let mut skeleton = vec![format!("fn:{}", function.name)];
    skeleton.extend(block_skeleton(&function.body));
    skeleton
}

fn expected_raw_total(algorithm: &str, ext: &str) -> usize {
    match (algorithm, ext) {
        ("quick_sort", "java" | "cpp") => 1,
        ("reverse_linked_list", "cpp") => 2,
        ("reverse_string", "java" | "cpp") => 1,
        ("two_sum", "java" | "cpp") => 2,
        _ => 0,
    }
}

fn block_skeleton(block: &Block) -> Vec<String> {
    let mut skeleton = Vec::new();
    for stmt in &block.0 {
        match stmt {
            Stmt::Assign { .. } => skeleton.push("assign".into()),
            Stmt::VarDecl(_) => skeleton.push("decl".into()),
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                skeleton.push("if".into());
                skeleton.extend(block_skeleton(then_block));
                if let Some(else_block) = else_block {
                    skeleton.push("else".into());
                    skeleton.extend(block_skeleton(else_block));
                }
                skeleton.push("end-if".into());
            }
            Stmt::While { body, .. } => {
                skeleton.push("while".into());
                skeleton.extend(block_skeleton(body));
                skeleton.push("end-while".into());
            }
            Stmt::For { body, .. } => {
                skeleton.push("for".into());
                skeleton.extend(block_skeleton(body));
                skeleton.push("end-for".into());
            }
            Stmt::Return(_) => skeleton.push("return".into()),
            Stmt::Break => skeleton.push("break".into()),
            Stmt::Continue => skeleton.push("continue".into()),
            Stmt::ExprStmt(_) => skeleton.push("expr".into()),
            Stmt::Raw(_) => skeleton.push("raw".into()),
        }
    }
    skeleton
}

#[test]
fn cross_language_skeletons_match_for_mvp_fixtures() {
    let algorithms = [
        "binary_search",
        "reverse_string",
        "reverse_linked_list",
        "quick_sort",
        "two_sum",
    ];

    for algorithm in algorithms {
        let py_module = parse_fixture(algorithm, "py");
        let java_module = parse_fixture(algorithm, "java");
        let cpp_module = parse_fixture(algorithm, "cpp");

        assert_eq!(
            collect_raw_stats(&py_module).total(),
            expected_raw_total(algorithm, "py"),
            "Python fixture raw fallback budget changed for {algorithm}"
        );
        assert_eq!(
            collect_raw_stats(&java_module).total(),
            expected_raw_total(algorithm, "java"),
            "Java fixture raw fallback budget changed for {algorithm}"
        );
        assert_eq!(
            collect_raw_stats(&cpp_module).total(),
            expected_raw_total(algorithm, "cpp"),
            "C++ fixture raw fallback budget changed for {algorithm}"
        );

        let py = module_skeleton(&py_module);
        let java = module_skeleton(&java_module);
        let cpp = module_skeleton(&cpp_module);

        assert_eq!(py, java, "Python and Java skeletons differ for {algorithm}");
        assert_eq!(py, cpp, "Python and C++ skeletons differ for {algorithm}");
    }
}
