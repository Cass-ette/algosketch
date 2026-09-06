use crate::ir::{Block, Expr, ForKind, Item, Module, Stmt};

/// Raw-fallback diagnostics recorded while parsing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawDiagnostics {
    pub items: usize,
    pub statements: usize,
    pub expressions: usize,
    /// 1-based source line numbers, in encounter order, may contain duplicates.
    pub lines: Vec<usize>,
}

impl RawDiagnostics {
    pub fn total(&self) -> usize {
        self.items + self.statements + self.expressions
    }

    pub fn record_item(&mut self, line: usize) {
        self.items += 1;
        self.lines.push(line);
    }

    pub fn record_statement(&mut self, line: usize) {
        self.statements += 1;
        self.lines.push(line);
    }

    pub fn record_expression(&mut self, line: usize) {
        self.expressions += 1;
        self.lines.push(line);
    }

    pub fn sorted_unique_lines(&self) -> Vec<usize> {
        let mut lines = self.lines.clone();
        lines.sort_unstable();
        lines.dedup();
        lines
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RawStats {
    pub items: usize,
    pub statements: usize,
    pub expressions: usize,
}

impl RawStats {
    pub fn total(self) -> usize {
        self.items + self.statements + self.expressions
    }
}

pub fn collect_raw_stats(module: &Module) -> RawStats {
    let mut stats = RawStats::default();

    for item in &module.items {
        collect_item(item, &mut stats);
    }

    stats
}

fn collect_item(item: &Item, stats: &mut RawStats) {
    match item {
        Item::Function(function) => collect_block(&function.body, stats),
        Item::Class(class) => {
            for field in &class.fields {
                if let Some(init) = &field.init {
                    collect_expr(init, stats);
                }
            }
            for method in &class.methods {
                collect_block(&method.body, stats);
            }
        }
        Item::GlobalVar(var) => {
            if let Some(init) = &var.init {
                collect_expr(init, stats);
            }
        }
        Item::Import(_) => {}
        Item::Raw(_) => stats.items += 1,
    }
}

fn collect_block(block: &Block, stats: &mut RawStats) {
    for stmt in &block.0 {
        collect_stmt(stmt, stats);
    }
}

fn collect_stmt(stmt: &Stmt, stats: &mut RawStats) {
    match stmt {
        Stmt::Assign { target, value } => {
            collect_expr(target, stats);
            collect_expr(value, stats);
        }
        Stmt::VarDecl(var) => {
            if let Some(init) = &var.init {
                collect_expr(init, stats);
            }
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            collect_expr(cond, stats);
            collect_block(then_block, stats);
            if let Some(else_block) = else_block {
                collect_block(else_block, stats);
            }
        }
        Stmt::While { cond, body } => {
            collect_expr(cond, stats);
            collect_block(body, stats);
        }
        Stmt::For { kind, body } => {
            collect_for_kind(kind, stats);
            collect_block(body, stats);
        }
        Stmt::Return(expr) => {
            if let Some(expr) = expr {
                collect_expr(expr, stats);
            }
        }
        Stmt::Break | Stmt::Continue => {}
        Stmt::ExprStmt(expr) => collect_expr(expr, stats),
        Stmt::Raw(_) => stats.statements += 1,
    }
}

fn collect_for_kind(kind: &ForKind, stats: &mut RawStats) {
    match kind {
        ForKind::CStyle { init, cond, step } => {
            collect_stmt(init, stats);
            collect_expr(cond, stats);
            collect_expr(step, stats);
        }
        ForKind::ForEach { iter, .. } => collect_expr(iter, stats),
        ForKind::Range {
            start, end, step, ..
        } => {
            collect_expr(start, stats);
            collect_expr(end, stats);
            if let Some(step) = step {
                collect_expr(step, stats);
            }
        }
    }
}

fn collect_expr(expr: &Expr, stats: &mut RawStats) {
    match expr {
        Expr::Literal(_) | Expr::Ident(_) => {}
        Expr::Binary { lhs, rhs, .. } => {
            collect_expr(lhs, stats);
            collect_expr(rhs, stats);
        }
        Expr::Unary { expr, .. } => collect_expr(expr, stats),
        Expr::Call { callee, args } => {
            collect_expr(callee, stats);
            for arg in args {
                collect_expr(arg, stats);
            }
        }
        Expr::Index { obj, index } => {
            collect_expr(obj, stats);
            collect_expr(index, stats);
        }
        Expr::Field { obj, .. } => collect_expr(obj, stats),
        Expr::Tuple(exprs) => {
            for expr in exprs {
                collect_expr(expr, stats);
            }
        }
        Expr::Raw(_) => stats.expressions += 1,
    }
}

#[cfg(test)]
mod raw_diagnostics_tests {
    use super::RawDiagnostics;

    #[test]
    fn total_sums_all_kinds() {
        let mut diag = RawDiagnostics::default();
        diag.record_item(1);
        diag.record_statement(2);
        diag.record_expression(3);
        assert_eq!(diag.total(), 3);
    }

    #[test]
    fn recorders_increment_counts_and_append_lines() {
        let mut diag = RawDiagnostics::default();
        diag.record_statement(5);
        diag.record_statement(7);
        assert_eq!(diag.statements, 2);
        assert_eq!(diag.lines, vec![5, 7]);
    }

    #[test]
    fn sorted_unique_lines_sorts_and_dedups() {
        let mut diag = RawDiagnostics::default();
        diag.record_statement(9);
        diag.record_expression(3);
        diag.record_statement(9);
        assert_eq!(diag.sorted_unique_lines(), vec![3, 9]);
    }
}
