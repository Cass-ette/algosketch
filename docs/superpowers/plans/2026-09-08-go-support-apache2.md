# Go Support + Apache-2.0 Relicense Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Go as a fourth source language (full tree-sitter parser, fixtures, cross-language parity) and relicense the project to Apache-2.0 only, releasing as v0.2.0.

**Architecture:** A new `GoParser` in `crates/algosketch-core/src/parser/go.rs` mirrors the existing three adapters (tree-sitter CST → IR, parse-time `RawDiagnostics` via the shared `record_raw_*` helpers). Renderers are untouched (IR is the interface). License/version/doc changes are mechanical edits.

**Tech Stack:** Rust 2021, tree-sitter 0.24 runtime + `tree-sitter-go` 0.25 (verified compatible: its only runtime dep is `tree-sitter-language = "0.1"`), assert_cmd for CLI tests.

**Spec:** `docs/superpowers/specs/2026-09-08-go-support-and-apache2-design.md` (decisions locked — do not re-litigate).

**Conventions:**
- Commit messages: conventional commits. **NEVER add `Co-Authored-By` lines.**
- Never push / create PRs / tag without the controller's go-ahead (the controller coordinates release; the final task handles it).
- Run everything from the worktree root. `cargo test -p algosketch-core` / `-p algosketch-cli` scopes runs.
- Grammar-driven work rule: **always verify node kinds/field names against the actual CST before writing mapping code** (Task 1 Step 4's dump is the source of truth; if reality differs from this plan's assumed kind names, reality wins — note the difference in your report).

**File structure:**

- `crates/algosketch-core/src/parser/go.rs` — NEW, the only substantial new code (~500 lines, mirrors java.rs/cpp.rs organization: trait impl + free `parse_*` functions).
- `crates/algosketch-core/src/lib.rs` — `SourceLang::Go` + extension map entry.
- `crates/algosketch-core/src/parser/mod.rs` — export `GoParser`.
- `crates/algosketch-core/tests/fixtures/*.go` — 5 new fixtures.
- `crates/algosketch-core/tests/{cross_language,diagnostics}.rs` — 4-language migration + Go diagnostics tests.
- `crates/algosketch-cli/src/main.rs` — `CliLang::Go` + dispatch arm.
- `crates/algosketch-cli/tests/cli.rs`, `crates/algosketch-cli/fixtures/binary_search.go` — CLI tests.
- `Cargo.toml`, `crates/*/Cargo.toml`, `Cargo.lock` — tree-sitter-go dep + version 0.2.0.
- `LICENSE-MIT` (delete), `README.md`, `docs/superpowers/specs/2026-05-20-algosketch-design.md` — license + doc sync.

---

## Chunk 1: Go parser (Tasks 1–5)

### Task 1: Dependency, wiring, skeleton parser + CST reality check

**Files:**
- Modify: `Cargo.toml` (workspace.dependencies), `crates/algosketch-core/Cargo.toml`
- Modify: `crates/algosketch-core/src/lib.rs`, `crates/algosketch-core/src/parser/mod.rs`, `crates/algosketch-core/src/parser/go.rs` (new)
- Modify: `crates/algosketch-cli/src/main.rs`
- Test: `crates/algosketch-core/src/lib.rs` (unit), `crates/algosketch-cli/tests/cli.rs`

- [ ] **Step 1: Write the failing tests**

In `lib.rs` tests module, add:

```rust
#[test]
fn go_extension_maps_to_go() {
    assert_eq!(SourceLang::from_extension("go"), Some(SourceLang::Go));
}
```

In `cli.rs`, add (uses a new fixture file created in Step 3):

```rust
#[test]
fn go_file_auto_detected_and_runs() {
    let fixture = format!("{}/fixtures/binary_search.go", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg(fixture).arg("--lang").arg("en");

    cmd.assert().success();
}
```

And create `crates/algosketch-cli/fixtures/binary_search.go` (full content below — it stays for Task 7's richer assertions):

```go
package main

func binary_search(items []int, target int) int {
	low := 0
	high := len(items) - 1
	for low <= high {
		mid := (low + high) / 2
		if items[mid] == target {
			return mid
		} else if items[mid] < target {
			low = mid + 1
		} else {
			high = mid - 1
		}
	}
	return -1
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p algosketch-core go_extension` → FAIL (no `Go` variant).
Run: `cargo test -p algosketch-cli --test cli go_file` → FAIL (unknown language, exit 1).

- [ ] **Step 3: Implement the wiring**

1. Root `Cargo.toml` `[workspace.dependencies]`: add `tree-sitter-go = "0.25"`.
2. `crates/algosketch-core/Cargo.toml` `[dependencies]`: add `tree-sitter-go.workspace = true`.
3. `lib.rs` `SourceLang`: add `Go` variant; `from_extension`: `"go" => Some(Self::Go)` (insert before the C++ arm's catch-all is fine — the match is on exact strings, order irrelevant); `as_str`: `Self::Go => "go"`.
4. `parser/mod.rs`: `pub mod go;` + `pub use go::GoParser;`.
5. `crates/algosketch-cli/src/main.rs`: `CliLang::Go` variant (+ `From<CliLang>` arm → `SourceLang::Go`), import `GoParser`, and add the dispatch arm in `run()`'s parse match: `SourceLang::Go => GoParser::new().parse(&source)?,`.
6. New file `crates/algosketch-core/src/parser/go.rs` — skeleton that compiles and returns an empty module:

```rust
use crate::diagnostics::RawDiagnostics;
use crate::error::{PseudoError, Result};
use crate::ir::Module;
use crate::SourceLang;

pub struct GoParser;

impl GoParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for GoParser {
    fn language(&self) -> SourceLang {
        SourceLang::Go
    }

    fn parse(&self, source: &str) -> Result<(Module, RawDiagnostics)> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(|e| PseudoError::Internal(format!("tree-sitter init: {e}")))?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| PseudoError::Parse {
                file: "input".into(),
                message: "parse failed".into(),
            })?;
        let root = tree.root_node();
        if root.has_error() {
            return Err(PseudoError::Parse {
                file: "input".into(),
                message: "syntax error".into(),
            });
        }

        let diag = RawDiagnostics::default(); // no mut yet: nothing records until Task 2
        let items = Vec::new(); // statements land in Task 2–4
        Ok((
            Module {
                source_language: SourceLang::Go,
                items,
            },
            diag,
        ))
    }
}
```

**API note (verified against the crate):** tree-sitter-go 0.25 exports `pub const LANGUAGE: LanguageFn` — there is **no** `language()` function. Use `tree_sitter_go::LANGUAGE.into()` exactly as java.rs:33 / cpp.rs:33 do for their grammars. Import the trait as the siblings do (`crate::parser::LanguageParser`).

**Errata (from Task 1 execution, verified empirically):**
1. tree-sitter-go 0.25 is grammar ABI 15; runtime 0.24.7 supports ABI 14 only — `set_language` hard-fails. The workspace `tree-sitter` runtime was therefore bumped to `"0.25"` (resolves 0.25.10). All existing grammars (python 0.21 / java 0.23.5 / cpp 0.23.4, ABI 14) are re-verified green on 0.25.10 (full workspace suite). Do not "fix" this back to 0.24.
2. Go `block` nodes wrap their statements in a single `statement_list` child (unlike java/cpp blocks). `parse_block` must descend through the `statement_list` wrapper before iterating statements.
3. The CST-dump fixture path in the throwaway test is `../../crates/algosketch-cli/fixtures/binary_search.go` (test CWD is `crates/algosketch-core`).

- [ ] **Step 4: CST reality check (MANDATORY before Tasks 2–4)**

Temporarily add this test at the bottom of `go.rs`, run it, and **record the dumped S-expression in your report** (it is the authority for every node kind/field name used later — if the real grammar differs from this plan's assumed names, follow the dump):

```rust
#[cfg(test)]
mod cst_dump {
    #[test]
    fn dump_binary_search() {
        let source = std::fs::read_to_string("../../algosketch-cli/fixtures/binary_search.go").unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into()).unwrap();
        let tree = parser.parse(&source, None).unwrap();
        panic!("{}", tree.root_node().to_sexp());
    }
}
```

Run: `cargo test -p algosketch-core cst_dump -- --nocapture 2>&1 | head -5`
Expected: the panic message contains the full S-expression. **Delete this test afterward** (it exists only for grammar verification).

Grammar facts pre-extracted from tree-sitter-go 0.25's `node-types.json` (from the local registry cache — still confirm in the dump, they are the authority):
- `function_declaration` / `method_declaration`: fields `name`, `parameters`, `receiver` (method only), `result`, `body`. Parameters are `parameter_declaration` nodes (fields `name`, `type`).
- `if_statement`: fields `condition`, `consequence` (the then-block — singular), `alternative`. `else if` nests: `alternative` is directly an `if_statement` (no else_if_clause kind); plain `else` gives a `block`.
- `for_statement`: its ONLY field is `body`. The loop shape comes from named CHILDREN: a `for_clause` child (fields `initializer`, `condition`, `update`), a `range_clause` child (fields `left`, `right`), a bare condition CHILD (no field — e.g. a `binary_expression` directly under `for_statement`), or nothing (infinite). Do NOT use `child_by_field_name("condition")` on `for_statement`.
- `return_statement` has NO fields; `expression_list` is a named child KIND (iterate named children to find it).
- `i++` / `i--` parse as `inc_statement` / `dec_statement`.
- Statement kinds: `expression_statement`, `short_var_declaration`, `assignment_statement`, `var_declaration`/`const_declaration` (containing `var_spec`/`const_spec`), `return_statement`, `break_statement`, `continue_statement`, `defer_statement`, `go_statement`, `labeled_statement`.
- Expression kinds: `binary_expression` (field `operator`), `unary_expression` (field `operator`), `call_expression` (fields `function`, `arguments`), `selector_expression` (fields `operand`, `field`), `index_expression` (fields `operand`, `index`), `parenthesized_expression`, `identifier`, `int_literal`, `float_literal`, `interpreted_string_literal`, `raw_string_literal`.

- [ ] **Step 5: Run tests**

Run: `cargo test --workspace`
Expected: PASS (go_extension unit test + go_file CLI test — the CLI test only asserts `.success()`, which the empty-module parser satisfies).

Run: `cargo clippy --workspace -- -D warnings` → PASS. `cargo fmt --all`.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/algosketch-core crates/algosketch-cli
git commit -m "feat(core): scaffold Go parser wiring and language detection"
```

### Task 2: Function declarations, params, receivers

**Files:**
- Modify: `crates/algosketch-core/src/parser/go.rs`
- Test: same file (`#[cfg(test)] mod tests`, mirroring python.rs's test style)

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Item;

    #[test]
    fn parses_go_function_shape() {
        let source = "package main\n\nfunc add(a int, b int) int {\n\treturn a + b\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.total(), 0);
        assert_eq!(module.items.len(), 1);
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
    }

    #[test]
    fn parses_go_method_receiver_as_param() {
        let source = "package main\n\nfunc (n *Node) value() int {\n\treturn n.v\n}\n";
        let (module, _) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.name, "value");
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].name, "n");
    }

    #[test]
    fn skips_package_and_import_silently() {
        let source = "package main\n\nimport \"fmt\"\n\nfunc f() {\n\tfmt.Println(1)\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.total(), 0); // package/import: silent, no Raw
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn returns_parse_error_for_invalid_go() {
        let source = "func broken( {\n";
        let err = GoParser::new().parse(source).unwrap_err();
        assert!(matches!(err, crate::error::PseudoError::Parse { .. }));
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p algosketch-core parses_go`
Expected: FAIL — items empty (skeleton returns no items).

- [ ] **Step 3: Implement**

In `go.rs`, following the structure of `java.rs` (trait impl calls a `collect` free function; free `parse_*` functions below):

- Top-level walk: iterate `root`'s named children.
  - `package_clause` / `import_declaration` → skip silently (continue).
  - `function_declaration` / `method_declaration` → `parse_function(source, node, &mut diag)?` → `Item::Function`.
  - Anything else → `record_raw_item(source, child, &mut diag)` (spec §2 top-level handling).
- `parse_function`: name from the `name` field (`node.child_by_field_name("name")`); params from the `parameters` field — a `parameter_list` whose children are `parameter_declaration` nodes, each contributing its `name` field as a `Param { name, type_hint: None }`. For `method_declaration`, the `receiver` field is also a parameter_list — prepend its names first.
- Body: the `body` field is a `block`. In THIS task, `parse_block` is a stub returning `Block(vec![])` (the tests here don't inspect bodies); Task 3 makes it real.
- Import `record_raw_item` from `crate::parser::common` (match the siblings' import style). `Span` from `node.start_byte()..node.end_byte()` like the siblings.

Pinning-test note: `returns_parse_error_for_invalid_go` already passes against the skeleton's `has_error` check, and `skips_package_and_import_silently` asserts `items.len() == 1` which needs the top-level walk — run the whole file's tests rather than a red-first ceremony for those two.

- [ ] **Step 4: Run tests**

Run: `cargo test -p algosketch-core` → PASS. `cargo clippy --workspace -- -D warnings` → PASS. `cargo fmt --all`.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/go.rs
git commit -m "feat(core): parse Go function declarations and receivers"
```

### Task 3: Statements and expressions

**Files:**
- Modify: `crates/algosketch-core/src/parser/go.rs`
- Test: same file

- [ ] **Step 1: Write the failing tests** (append to `mod tests`)

```rust
    #[test]
    fn parses_go_declarations_and_assignments() {
        let source = "package main\n\nfunc f() {\n\tvar x int = 3\n\ty := 4\n\tx = y\n\ta, b := 1, 2\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        use crate::ir::Stmt;
        assert!(matches!(f.body.0[0], Stmt::VarDecl(_)));       // var x int = 3
        assert!(matches!(f.body.0[1], Stmt::VarDecl(_)));       // y := 4 (single-name :=)
        assert!(matches!(f.body.0[2], Stmt::Assign { .. }));    // x = y
        assert!(matches!(f.body.0[3], Stmt::Assign { .. }));    // a, b := 1, 2 (multi)
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn parses_go_literals_and_operators() {
        let source = "package main\n\nfunc f() {\n\tx := 1 + 2 * 3\n\ts := \"hi\"\n\tb := true\n\tn := nil\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        assert_eq!(f.body.0.len(), 4);
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn parses_go_return_break_continue() {
        let source = "package main\n\nfunc f() {\n\treturn 1\n}\n\nfunc g() {\n\treturn\n}\n";
        let (module, _) = GoParser::new().parse(source).unwrap();
        use crate::ir::Stmt;
        let Item::Function(f) = &module.items[0] else { panic!() };
        assert!(matches!(&f.body.0[0], Stmt::Return(Some(_))));
        let Item::Function(g) = &module.items[1] else { panic!() };
        assert!(matches!(&g.body.0[0], Stmt::Return(None)));
    }

    #[test]
    fn parses_go_calls_index_and_selectors() {
        let source = "package main\n\nfunc f(xs []int) {\n\tlen(xs)\n\tv := xs[0]\n\tw := xs[0].field\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        assert_eq!(f.body.0.len(), 3);
        assert_eq!(diag.total(), 0);
    }
```

- [ ] **Step 2: Run to verify they fail** — `cargo test -p algosketch-core parses_go` → FAIL.

- [ ] **Step 3: Implement**

Free functions (mirror java.rs/cpp.rs shapes — all take `source`, `node`, `diag`):

- `parse_block` → iterate named children → `parse_stmt`.
- `parse_stmt` — dispatch on `node.kind()` (kinds per Task 1's dump; expected):
  - `expression_statement` → `Stmt::ExprStmt(parse_expr(inner)?)`
  - `short_var_declaration` → if exactly one name: `Stmt::VarDecl` (name + `init` from the single expression); multiple names → `Stmt::Assign { target: Expr::Tuple(names), value: parse_expr or Tuple }` (spec §2)
  - `assignment_statement` → `Stmt::Assign`; `left`/`right` fields may each hold multiple expressions → single → direct, multiple → `Expr::Tuple`
  - `var_declaration` / `const_declaration` → for each inner spec (`var_spec`/`const_spec`): single name → `Stmt::VarDecl`; multiple → one `Stmt::VarDecl` per name if no init, else Assign-with-tuple (keep it simple: mirror short_var logic)
  - `return_statement` → `expression_list` field: empty → `Return(None)`, one → `Return(Some(e))`, many → `Return(Some(Expr::Tuple))`
  - `break_statement` / `continue_statement` → `Stmt::Break` / `Stmt::Continue`
  - `if_statement`, `for_statement` → **leave falling through to `record_raw_stmt` in this task** (Task 4 implements them; the tests above don't use them)
  - everything else → `record_raw_stmt`
- `parse_expr` — dispatch (kinds per dump; expected):
  - `identifier` → special-case text: `"true"` → `Literal::Bool(true)`, `"false"` → `Literal::Bool(false)`, `"nil"` → `Literal::None`; else `Expr::Ident`
  - `int_literal` → `text.parse::<i64>()` → `Literal::Int`, Err → `record_raw_expr` (keep `text` as scrutinee — same as java/cpp)
  - `interpreted_string_literal` / `raw_string_literal` → `Literal::Str(text.trim_matches('"'))`
  - `float_literal` → `Literal::Float(text.to_string())`
  - `parenthesized_expression` → recurse into inner
  - `binary_expression` → operator via `child_by_field_name("operator")`; map with `parse_c_family_bin_op` (covers `== != < <= > >= && || + - * / % << >> & | ^`; `/` → `IntDiv` is correct for Go ints) — non-matching ops → `record_raw_expr`
  - `unary_expression` → operator token (`!`, `-`) → `parse_un_op`; any operator `parse_un_op` does NOT recognize (e.g. `^`, `&`, `*` deref) → `record_raw_expr` — **never** a hard error (spec: unrecognized syntax always falls back)
  - `call_expression` → `Expr::Call { callee: function field, args: arguments field's named exprs }` (type conversions like `T(x)` also land here — fine)
  - `index_expression` → `Expr::Index { obj: operand, index }`
  - `selector_expression` → `Expr::Field { obj: operand, name: field }`
  - everything else → `record_raw_expr`

- [ ] **Step 4: Run tests** — `cargo test -p algosketch-core` → PASS; clippy; fmt.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/go.rs
git commit -m "feat(core): parse Go statements and expressions"
```

### Task 4: Control flow (if chains, all four for-shapes)

**Files:**
- Modify: `crates/algosketch-core/src/parser/go.rs`
- Test: same file

- [ ] **Step 1: Write the failing tests** (append)

```rust
    #[test]
    fn parses_go_if_else_chain() {
        let source = "package main\n\nfunc f(x int) int {\n\tif x == 1 {\n\t\treturn 1\n\t} else if x == 2 {\n\t\treturn 2\n\t} else {\n\t\treturn 3\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        use crate::ir::Stmt;
        let Stmt::If { then_block, else_block: Some(else_block), .. } = &f.body.0[0] else {
            panic!("expected if with else");
        };
        assert_eq!(then_block.0.len(), 1);
        // else-if must nest an If inside the else block (matches python/java shape)
        assert!(matches!(else_block.0[0], Stmt::If { .. }));
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn parses_go_cstyle_and_cond_fors() {
        let source = "package main\n\nfunc f(n int) {\n\tfor i := 0; i < n; i++ {\n\t\tg(i)\n\t}\n\tfor n > 0 {\n\t\tn = n - 1\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        use crate::ir::{ForKind, Stmt};
        let Stmt::For { kind: ForKind::CStyle { .. }, .. } = &f.body.0[0] else {
            panic!("expected c-style for");
        };
        let Stmt::While { cond, .. } = &f.body.0[1] else { panic!("expected while") };
        // pin the condition actually parsed (guards against silently dropping it)
        assert!(matches!(cond, crate::ir::Expr::Binary { .. }));
        // the i++ update clause records one Raw expression (cpp/java parity)
        assert_eq!(diag.expressions, 1);
        assert_eq!(diag.statements, 0);
    }

    #[test]
    fn parses_go_range_and_infinite_for() {
        let source = "package main\n\nfunc f(xs []int) {\n\tfor v := range xs {\n\t\tg(v)\n\t}\n\tfor {\n\t\tbreak\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        use crate::ir::{ForKind, Literal, Stmt};
        let Stmt::For { kind: ForKind::ForEach { var, .. }, .. } = &f.body.0[0] else {
            panic!("expected foreach");
        };
        assert_eq!(var, "v");
        let Stmt::While { cond, .. } = &f.body.0[1] else { panic!("expected while") };
        assert_eq!(cond, &crate::ir::Expr::Literal(Literal::Bool(true)));
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn two_var_range_falls_back_to_raw() {
        let source = "package main\n\nfunc f(xs []int) {\n\tfor i, v := range xs {\n\t\tg(i)\n\t\tg(v)\n\t}\n}\n";
        let (_, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.statements, 1);
        assert_eq!(diag.sorted_unique_lines(), vec![4]);
    }
```

- [ ] **Step 2: Run to verify they fail** — control-flow statements currently `record_raw_stmt`.

- [ ] **Step 3: Implement** (dispatch arms in `parse_stmt`; grammar shapes per Task 1's pre-extracted facts)

- `if_statement`: `condition` field → `parse_expr`; `consequence` (singular) → `parse_block`; `alternative`:
  - an `if_statement` (tree-sitter-go nests else-if directly) → `else_block = Block(vec![nested If])` (matches python/java nesting — REQUIRED for skeleton parity)
  - a `block` → `parse_block`
- `for_statement` (its only field is `body`; the clause shape decides the kind):
  - `for_clause` child (fields `initializer`, `condition`, `update`): initializer → `parse_stmt` (Box), condition → `parse_expr`, update → `record_raw_expr` (Raw — cpp/java parity, drives fixture budgets) → `ForKind::CStyle`
  - no `for_clause`/`range_clause` child → inspect the named children besides the `body` block: exactly one → `Stmt::While { cond: parse_expr(that child) }`; none → `Stmt::While { cond: Literal::Bool(true) }`. (The condition has no field — do not call `child_by_field_name("condition")`.)
  - `range_clause` child (fields `left`, `right`): single name on the left (`for v := range xs` / `for _ := range xs`) → `ForKind::ForEach { var, iter: right }`; two names on the left → `record_raw_stmt` on the whole for statement (test above)

Pinning-test note: `two_var_range_falls_back_to_raw` passes even before this task (unhandled `for` already hits `record_raw_stmt`) — it locks the behavior in.

- [ ] **Step 4: Run tests** — `cargo test -p algosketch-core` → PASS; clippy; fmt.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/go.rs
git commit -m "feat(core): parse Go control flow"
```

### Task 5: Go diagnostics integration tests

**Files:**
- Modify: `crates/algosketch-core/tests/diagnostics.rs`

- [ ] **Step 1: Write the failing test** (append; extend the import with `GoParser`)

```rust
#[test]
fn go_reports_raw_statement_and_expression_lines() {
    let source = "package main\n\nfunc f(x int) {\n\tdefer close(c)\n\tfor i := 0; i < x; i++ {\n\t\tg(i)\n\t}\n}\n";
    let (_, diag) = GoParser::new().parse(source).unwrap();
    assert_eq!(diag.statements, 1); // defer
    assert_eq!(diag.expressions, 1); // i++ update clause
    assert_eq!(diag.sorted_unique_lines(), vec![4, 5]);
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test -p algosketch-core --test diagnostics go_reports` → FAIL (no `go_reports…` test yet / import missing).

- [ ] **Step 3: Verify it passes** (no implementation needed — Task 4 already provides the behavior; this test pins it). If it fails, the parser is wrong — fix the parser, not the test.

Run: `cargo test -p algosketch-core --test diagnostics` → PASS. `cargo clippy --workspace -- -D warnings` → PASS. `cargo fmt --all`.

- [ ] **Step 4: Commit**

```bash
git add crates/algosketch-core/tests/diagnostics.rs
git commit -m "test(core): pin Go raw diagnostics lines"
```

---

## Chunk 2: Fixtures, CLI, license, release (Tasks 6–10)

### Task 6: Five Go fixtures + cross-language migration

**Files:**
- Create: `crates/algosketch-core/tests/fixtures/{binary_search,two_sum,quick_sort,reverse_string,reverse_linked_list}.go`
- Modify: `crates/algosketch-core/tests/cross_language.rs`

- [ ] **Step 1: Write the failing migration**

In `cross_language.rs`:

1. `parse_fixture` gains the arm:

```rust
"go" => GoParser::new().parse(&source),
```

(and add `GoParser` to the import.)

2. `expected_raw_total` gains Go entries:

```rust
fn expected_raw_total(algorithm: &str, ext: &str) -> usize {
    match (algorithm, ext) {
        ("quick_sort", "java" | "cpp" | "go") => 1,
        ("reverse_linked_list", "cpp") => 2,
        ("reverse_string", "java" | "cpp" | "go") => 1,
        ("two_sum", "java" | "cpp" | "go") => 2,
        _ => 0,
    }
}
```

3. In the test body, add the fourth fixture parse + budget assert + skeleton equality:

```rust
let (go_module, go_diag) = parse_fixture(algorithm, "go");
assert_eq!(
    go_diag.total(),
    expected_raw_total(algorithm, "go"),
    "Go fixture raw fallback budget changed for {algorithm}"
);
let go = module_skeleton(&go_module);
assert_eq!(py, go, "Python and Go skeletons differ for {algorithm}");
```

- [ ] **Step 2: Create the fixtures**

`binary_search.go` (identical to the CLI fixture from Task 1):

```go
package main

func binary_search(items []int, target int) int {
	low := 0
	high := len(items) - 1
	for low <= high {
		mid := (low + high) / 2
		if items[mid] == target {
			return mid
		} else if items[mid] < target {
			low = mid + 1
		} else {
			high = mid - 1
		}
	}
	return -1
}
```

`two_sum.go` (budget 2 — two `++` update clauses):

```go
package main

func two_sum(items []int, target int) int {
	for i := 0; i < len(items); i++ {
		for j := i + 1; j < len(items); j++ {
			if items[i]+items[j] == target {
				return i
			}
		}
	}
	return -1
}
```

`quick_sort.go` (budget 1 — one `j++` update):

```go
package main

func quick_sort(items []int, low int, high int) []int {
	if low < high {
		pivot := partition(items, low, high)
		quick_sort(items, low, pivot-1)
		quick_sort(items, pivot+1, high)
	}
	return items
}

func partition(items []int, low int, high int) int {
	pivot := items[high]
	i := low
	for j := low; j < high; j++ {
		if items[j] < pivot {
			temp := items[i]
			items[i] = items[j]
			items[j] = temp
			i = i + 1
		}
	}
	temp := items[i]
	items[i] = items[high]
	items[high] = temp
	return i
}
```

`reverse_string.go` (budget 1 — `i--` update):

```go
package main

func reverse_string(text string) string {
	result := ""
	for i := len(text) - 1; i >= 0; i-- {
		result = result + text[i]
	}
	return result
}
```

`reverse_linked_list.go` (budget 0 — no for loops; `var` for the None-initialized decl to match the other languages' `decl` tags):

```go
package main

func reverse_linked_list(head *Node) *Node {
	var previous *Node = nil
	current := head
	for current != nil {
		next_node := current.next
		current.next = previous
		previous = current
		current = next_node
	}
	return previous
}
```

(Skeleton-tag discipline per spec §4: every line that is `decl` in the other languages is `var`/single-name `:=` here; every `assign` line is `=`; `low := 0` etc. are single-name `:=` → VarDecl → `decl`, matching the Python typed decls.)

- [ ] **Step 3: Run the canary**

Run: `cargo test -p algosketch-core --test cross_language`
Expected: PASS. If a skeleton differs, debug which tag mismatches (`assert_eq!` diff shows it) and fix the **fixture spelling** (not the parser) unless the parser genuinely mis-parses valid Go.

Run: `cargo test --workspace` → PASS. clippy, fmt.

- [ ] **Step 4: Commit**

```bash
git add crates/algosketch-core/tests/fixtures crates/algosketch-core/tests/cross_language.rs
git commit -m "test(core): add Go fixtures and four-language skeleton parity"
```

### Task 7: CLI Go tests

**Files:**
- Modify: `crates/algosketch-cli/tests/cli.rs`

- [ ] **Step 1: Write the pinning tests** (two NEW tests — keep Task 1's `go_file_auto_detected_and_runs` as-is)

```rust
#[test]
fn go_file_outputs_pseudocode_and_explanation() {
    let fixture = format!("{}/fixtures/binary_search.go", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg(fixture).arg("--lang").arg("en");

    cmd.assert()
        .success()
        .stdout(contains("FUNCTION binary_search"))
        .stdout(contains("WHILE"))
        .stdout(contains("RETURN"))
        .stdout(contains("Purpose:"))
        .stdout(contains("Steps:"));
}

#[test]
fn go_stdin_with_source_lang() {
    let go_source = "package main\n\nfunc double(x int) int {\n\treturn x * 2\n}\n";
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("-")
        .arg("--source-lang")
        .arg("go")
        .arg("--lang")
        .arg("en")
        .write_stdin(go_source);

    cmd.assert()
        .success()
        .stdout(contains("FUNCTION double"))
        .stdout(contains("RETURN x * 2"));
}
```

- [ ] **Step 2: Run them** — by this point Tasks 2–6 already provide the behavior, so both pass immediately (they are pinning tests). If either fails, something upstream is wrong — fix upstream, not the tests.

Run: `cargo test -p algosketch-cli --test cli` → PASS (30 tests: 27 pre-existing + Task 1's + these two). clippy, fmt.

- [ ] **Step 3: Commit**

```bash
git add crates/algosketch-cli/tests/cli.rs
git commit -m "test(cli): cover Go auto-detection and output"
```

### Task 8: Relicense to Apache-2.0

**Files:**
- Delete: `LICENSE-MIT`
- Modify: `Cargo.toml`, `README.md`, `docs/superpowers/specs/2026-05-20-algosketch-design.md`

- [ ] **Step 1: Make the changes**

1. Root `Cargo.toml` `[workspace.package]`: `license = "MIT OR Apache-2.0"` → `license = "Apache-2.0"`.
2. `git rm LICENSE-MIT`.
3. README EN section (near line 109-115): replace the dual-license block with:

```markdown
### License

Apache-2.0 — see [`LICENSE-APACHE`](LICENSE-APACHE).
```

4. README 中文 section (near line 205-208): replace with:

```markdown
### 许可协议

Apache-2.0 — 见 [`LICENSE-APACHE`](LICENSE-APACHE)。
```

(Leave "MIT Press" bibliography citations untouched — they are not license references.)
5. Main design spec header line 6: `- License: MIT OR Apache-2.0` → `- License: Apache-2.0`.

- [ ] **Step 2: Verify**

```bash
grep -n "LICENSE-MIT\|MIT —\|MIT OR" README.md Cargo.toml docs/superpowers/specs/2026-05-20-algosketch-design.md || echo clean
```
Expected: `clean`. Also `cargo check --workspace` (license field change must not break anything; Cargo.lock untouched by this).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml README.md docs/superpowers/specs/2026-05-20-algosketch-design.md
git commit -m "chore: relicense to Apache-2.0 only"
```

### Task 9: Version 0.2.0 + spec/README language sync

**Files:**
- Modify: `Cargo.toml`, `crates/algosketch-cli/Cargo.toml`, `Cargo.lock`
- Modify: `docs/superpowers/specs/2026-05-20-algosketch-design.md`, `README.md`

- [ ] **Step 1: Version bump**

Root `Cargo.toml`: `version = "0.1.0"` → `"0.2.0"`. `crates/algosketch-cli/Cargo.toml` path dep: `version = "0.1.0"` → `"0.2.0"`. Run `cargo check --workspace` to refresh `Cargo.lock`.

- [ ] **Step 2: Main design spec updates**

In `docs/superpowers/specs/2026-05-20-algosketch-design.md`:

1. §1: "three source languages (Python, Java, C++)" → "four source languages (Python, Java, C++, Go)".
2. §3 diagram caption "Python / Java / C++ adapters" → "Python / Java / C++ / Go adapters".
3. §4 crate layout: add `│   │   │   │   └── go.rs` under parser/ (keep the tree aligned).
4. §7 CLI: `-l, --source-lang <LANG>      python | java | cpp (auto from extension)` → `python | java | cpp | go`; extension table add `| \`.go\` | Go |`.
5. §9: "15 samples (5 algorithms × 3 languages)" → "20 samples (5 algorithms × 4 languages)"; also the bullet "the same algorithm in three languages must produce equivalent control-flow IR skeletons" → "four languages" (~line 373).
6. §10: append milestone row `| M6 | Go + Apache-2.0 | Go parser + fixtures in; 4-language skeleton test green; relicensed. |`; update the completion line "All milestones M1–M5 complete; v0.1.0 tagged 2026-09." → "All milestones M1–M6 complete; v0.2.0 tagged 2026-09." Change the future-track lines to `- v0.3: LLM provider hooked in for \`Raw\` node fallback.` / `- v0.4: WASM build + minimal web UI.`

Note: the §3 diagram edit sits inside an ASCII box with a right `│` border — keep the border aligned (trim/adjust trailing spaces after lengthening the caption).

- [ ] **Step 3: README language-count updates**

Run `grep -n "Python, Java, or C++\|Python, Java, and C++\|Python / Java / C++\|Python、Java\|三种 tree-sitter\|三种源语言" README.md` and update every hit to the four-language wording (EN + 中文 sections both — "and C++" and "or C++" spellings both occur). Also update the README status line ("v0.1 MVP" → "v0.2 — supports Python, Java, C++, and Go" or equivalent) in both language sections. Sanity-check usage examples still match (`algosketch --help`).

- [ ] **Step 4: Full suite**

Run: `cargo test --workspace` → PASS. `cargo clippy --workspace -- -D warnings`, `cargo fmt --all`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/algosketch-cli/Cargo.toml docs/superpowers/specs/2026-05-20-algosketch-design.md README.md
git commit -m "chore: bump version to 0.2.0 and sync docs for Go support"
```

### Task 10: Final gate + release coordination (controller-executed)

- [ ] **Step 1: Full local gate**

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo build --release --workspace
git status --short   # clean
```

- [ ] **Step 2: Smoke tests**

```bash
cargo run --release -- crates/algosketch-core/tests/fixtures/quick_sort.go --lang zh
cargo run --release -- crates/algosketch-cli/fixtures/binary_search.go --no-pseudo --lang en
cat crates/algosketch-core/tests/fixtures/two_sum.go | cargo run --release -- - --source-lang go --lang en
cargo run --release -- --version   # 0.2.0
```

- [ ] **Step 3: Push + PR + CI + merge + tag v0.2.0 + cleanup** (controller coordinates; user pre-authorized)

**Done when:** spec §7 acceptance checklist is fully ticked and `v0.2.0` is tagged.
