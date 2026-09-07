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
        assert_eq!(diag.total(), diag.lines.len());
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
