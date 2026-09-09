use algosketch_core::parser::{CppParser, GoParser, JavaParser, LanguageParser, PythonParser};

#[test]
fn python_reports_raw_statement_line() {
    let source = "\ndef f():\n    yield 42\n    return 1\n";
    let (_, diag) = PythonParser::new().parse(source).unwrap();
    assert_eq!(diag.total(), 1);
    assert_eq!(diag.statements, 1);
    assert_eq!(diag.sorted_unique_lines(), vec![3]);
}

#[test]
fn java_reports_raw_statement_line() {
    let source = "class C {\n    int sum(int[] values) {\n        int total = 0;\n        total += values[0];\n        return total;\n    }\n}\n";
    let (_, diag) = JavaParser::new().parse(source).unwrap();
    assert_eq!(diag.total(), 1);
    assert_eq!(diag.statements, 1);
    assert_eq!(diag.sorted_unique_lines(), vec![4]);
}

#[test]
fn cpp_reports_raw_statement_line() {
    let source = "int probe(int x) {\n    int *p;\n    return x;\n}\n";
    let (_, diag) = CppParser::new().parse(source).unwrap();
    assert_eq!(diag.total(), 1);
    assert_eq!(diag.statements, 1);
    assert_eq!(diag.sorted_unique_lines(), vec![2]);
}

#[test]
fn go_reports_raw_statement_and_expression_lines() {
    let source = "package main\n\nfunc f(x int) {\n\tdefer close(c)\n\tfor i := 0; i < x; i++ {\n\t\tg(i)\n\t}\n}\n";
    let (_, diag) = GoParser::new().parse(source).unwrap();
    assert_eq!(diag.statements, 1); // defer
    assert_eq!(diag.expressions, 1); // i++ update clause
    assert_eq!(diag.sorted_unique_lines(), vec![4, 5]);
}
