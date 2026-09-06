use algosketch_core::parser::{LanguageParser, PythonParser};

#[test]
fn python_reports_raw_statement_line() {
    let source = "\ndef f():\n    yield 42\n    return 1\n";
    let (_, diag) = PythonParser::new().parse_with_diag(source).unwrap();
    assert_eq!(diag.total(), 1);
    assert_eq!(diag.statements, 1);
    assert_eq!(diag.sorted_unique_lines(), vec![3]);
}
