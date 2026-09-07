use algosketch_core::parser::{CppParser, JavaParser, LanguageParser, PythonParser};

#[test]
fn python_reports_raw_statement_line() {
    let source = "\ndef f():\n    yield 42\n    return 1\n";
    let (_, diag) = PythonParser::new().parse_with_diag(source).unwrap();
    assert_eq!(diag.total(), 1);
    assert_eq!(diag.statements, 1);
    assert_eq!(diag.sorted_unique_lines(), vec![3]);
}

#[test]
fn java_reports_raw_statement_line() {
    let source = "class C {\n    int sum(int[] values) {\n        int total = 0;\n        total += values[0];\n        return total;\n    }\n}\n";
    let (_, diag) = JavaParser::new().parse_with_diag(source).unwrap();
    assert_eq!(diag.total(), 1);
    assert_eq!(diag.statements, 1);
    assert_eq!(diag.sorted_unique_lines(), vec![4]);
}

#[test]
fn cpp_reports_raw_statement_line() {
    let source = "int probe(int x) {\n    int *p;\n    return x;\n}\n";
    let (_, diag) = CppParser::new().parse_with_diag(source).unwrap();
    assert_eq!(diag.total(), 1);
    assert_eq!(diag.statements, 1);
    assert_eq!(diag.sorted_unique_lines(), vec![2]);
}
