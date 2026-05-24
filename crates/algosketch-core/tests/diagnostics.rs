use algosketch_core::diagnostics::{collect_raw_stats, RawStats};
use algosketch_core::ir::*;
use algosketch_core::SourceLang;

#[test]
fn collect_raw_stats_counts_nested_items_statements_and_expressions() {
    let module = Module {
        source_language: SourceLang::Python,
        items: vec![
            Item::Raw("decorator-like top-level fallback".into()),
            Item::Function(Function {
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Block(vec![
                    Stmt::Raw("yield 1".into()),
                    Stmt::Assign {
                        target: Expr::Ident("x".into()),
                        value: Expr::Binary {
                            op: BinOp::Add,
                            lhs: Box::new(Expr::Raw("unsupported_left".into())),
                            rhs: Box::new(Expr::Literal(Literal::Int(1))),
                        },
                    },
                    Stmt::For {
                        kind: ForKind::CStyle {
                            init: Box::new(Stmt::Raw("int i = 0".into())),
                            cond: Expr::Raw("i < n".into()),
                            step: Expr::Raw("i++".into()),
                        },
                        body: Block(vec![Stmt::ExprStmt(Expr::Call {
                            callee: Box::new(Expr::Ident("visit".into())),
                            args: vec![Expr::Raw("unsupported_arg".into())],
                        })]),
                    },
                ]),
                span: Span::default(),
            }),
        ],
    };

    let stats = collect_raw_stats(&module);

    assert_eq!(
        stats,
        RawStats {
            items: 1,
            statements: 2,
            expressions: 4,
        }
    );
    assert_eq!(stats.total(), 7);
}
