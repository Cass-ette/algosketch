# M3 Java/C++ + MVP Wrap-up Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add minimal Java and C++ parser support for common algorithm code, prove language-neutral IR with cross-language fixtures, and finish v0.1 MVP polish.

**Architecture:** Extend the existing tree-sitter parser layer so Python, Java, and C++ feed the same IR, then reuse the existing pseudocode and explanation renderers. Keep M3 deliberately narrow: common single-file algorithm syntax only, no semantic analysis, no full language coverage, and unknown syntax preserved as `Raw`. M5 hardens CLI behavior, fallback warnings, docs, and release validation without adding any LLM-backed behavior.

**Tech Stack:** Rust workspace, tree-sitter grammars, `assert_cmd` CLI tests, existing `algosketch-core` IR/renderers, existing `algosketch-cli` binary.

**Context:** Base worktree is `/Users/chenzilve/Projects/pseudocode/.worktrees/m4-explanation-renderer` on branch `m4-explanation-renderer`. Base branch is `main`; no remote `develop` exists. No PR exists yet for `m4-explanation-renderer`.

**Relevant skills:** Implement with @superpowers:subagent-driven-development if subagents are available; otherwise use @superpowers:executing-plans. Use @superpowers:test-driven-development for each feature/bugfix task and @superpowers:verification-before-completion before claiming completion.

**Cargo test command note:** Cargo accepts a single positional test-name filter per invocation. When this plan names multiple tests together, run either a shared substring filter, separate `cargo test` commands, or the broader module/crate test command shown nearby. Do not pass multiple unrelated filters in one `cargo test` command.

---

## Current State Summary

- M4 baseline currently passes `cargo test --workspace`: CLI unit 1, CLI integration 12, core unit 36.
- `SourceLang::{Python, Java, Cpp}` and extension inference already exist in `crates/algosketch-core/src/lib.rs`.
- CLI already recognizes `--source-lang java|cpp` through `CliLang`, but dispatch returns `UnsupportedLanguage` for Java/C++.
- `crates/algosketch-core/src/parser.rs` currently contains a single `PythonParser` implementation.
- `PseudoRenderer` already renders functions, assignments, while/if/return/break/continue/raw/expr statements, but still emits `// <unsupported stmt>` for `Stmt::For` and `Stmt::VarDecl`.
- `ExplainRenderer` already handles `VarDecl` and all `ForKind` variants.
- Existing fixtures only cover Python `binary_search.py`; M3 needs Java/C++ and four more algorithm fixture families.

---

## File Structure

### Create

- `crates/algosketch-core/src/parser/mod.rs` — parser trait, parser module exports, shared public parser types.
- `crates/algosketch-core/src/parser/common.rs` — shared tree-sitter helpers and operator parsing used by all parser adapters.
- `crates/algosketch-core/src/parser/python.rs` — existing Python parser moved from `parser.rs`, plus Python M3 gap fixes.
- `crates/algosketch-core/src/parser/java.rs` — Java tree-sitter adapter.
- `crates/algosketch-core/src/parser/cpp.rs` — C++ tree-sitter adapter.
- `crates/algosketch-core/src/diagnostics.rs` — Raw fallback counting for MVP warnings.
- `crates/algosketch-core/tests/cross_language.rs` — cross-language skeleton tests.
- `crates/algosketch-core/tests/fixtures/binary_search.py`
- `crates/algosketch-core/tests/fixtures/binary_search.java`
- `crates/algosketch-core/tests/fixtures/binary_search.cpp`
- `crates/algosketch-core/tests/fixtures/reverse_string.py`
- `crates/algosketch-core/tests/fixtures/reverse_string.java`
- `crates/algosketch-core/tests/fixtures/reverse_string.cpp`
- `crates/algosketch-core/tests/fixtures/reverse_linked_list.py`
- `crates/algosketch-core/tests/fixtures/reverse_linked_list.java`
- `crates/algosketch-core/tests/fixtures/reverse_linked_list.cpp`
- `crates/algosketch-core/tests/fixtures/quick_sort.py`
- `crates/algosketch-core/tests/fixtures/quick_sort.java`
- `crates/algosketch-core/tests/fixtures/quick_sort.cpp`
- `crates/algosketch-core/tests/fixtures/two_sum.py`
- `crates/algosketch-core/tests/fixtures/two_sum.java`
- `crates/algosketch-core/tests/fixtures/two_sum.cpp`
- `crates/algosketch-cli/fixtures/binary_search.java` — user-facing CLI fixture.
- `crates/algosketch-cli/fixtures/binary_search.cpp` — user-facing CLI fixture.

### Modify

- `Cargo.toml` — add workspace dependencies for Java/C++ tree-sitter grammars and likely tree-sitter version bump.
- `Cargo.lock` — updated by Cargo after dependency changes.
- `crates/algosketch-core/Cargo.toml` — add core grammar dependencies.
- `crates/algosketch-core/src/lib.rs` — export `diagnostics`; parser module path remains `pub mod parser`.
- `crates/algosketch-core/src/parser.rs` — remove after moving contents to `src/parser/` directory.
- `crates/algosketch-core/src/renderer/pseudo.rs` — render `Stmt::VarDecl` and `Stmt::For` instead of unsupported comments.
- `crates/algosketch-cli/src/main.rs` — dispatch Java/C++ parsers, emit Raw fallback warnings, add all-output-disabled handling if needed.
- `crates/algosketch-cli/tests/cli.rs` — add Java/C++ CLI tests, warnings tests, source-lang stdin tests, output file tests if missing.
- `README.md` — update status and usage from planned/pre-alpha wording to MVP-supported wording after validation.

---

## Acceptance Criteria

M3 is complete when:

1. `JavaParser` and `CppParser` exist and implement `LanguageParser`.
2. CLI accepts `.java`, `.cpp`, `.cc`, `.cxx`, `.hpp`, `.h` paths and dispatches to the correct parser.
3. `binary_search`, `reverse_string`, `reverse_linked_list`, `quick_sort`, and `two_sum` fixtures exist in Python/Java/C++.
4. Cross-language skeleton tests pass for all five fixture families.
5. Pseudocode output no longer emits `// <unsupported stmt>` for parsed `VarDecl` and `For` nodes.

M5/MVP wrap-up is complete when:

1. Raw fallback warnings are emitted to stderr unless `--quiet` is present or equivalent quiet behavior is implemented.
2. Error exit codes remain: user/IO/unsupported/unknown = 1, parse = 2, internal = 3.
3. CLI tests cover Java/C++, stdin language selection, output files, fallback warnings, and quiet behavior.
4. README accurately describes what v0.1 supports and does not imply unsupported full Java/C++ parsing.
5. `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, and manual CLI smoke tests for Python/Java/C++ all pass.

---

## Chunk 1: Parser Module Preparation

### Task 1: Add Java/C++ tree-sitter dependencies

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/algosketch-core/Cargo.toml`
- Temporarily modify for test: `crates/algosketch-core/src/parser.rs`

- [ ] **Step 1: Write failing grammar-load tests**

Add these temporary tests to `crates/algosketch-core/src/parser.rs` inside the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn java_grammar_loads() {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .expect("java grammar should load");
}

#[test]
fn cpp_grammar_loads() {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .expect("cpp grammar should load");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --package algosketch-core grammar_loads
```

Expected: FAIL to compile with unresolved crates `tree_sitter_java` and `tree_sitter_cpp`.

- [ ] **Step 3: Add workspace dependencies**

Use the current grammar API shape observed on 2026-05-22:

- `tree-sitter-java = "0.23.5"` exposes `tree_sitter_java::LANGUAGE`.
- `tree-sitter-cpp = "0.23.4"` exposes `tree_sitter_cpp::LANGUAGE`.
- Existing `tree-sitter-python = "0.21"` exposes `tree_sitter_python::language()` and depends on `tree-sitter >=0.21.0`.

First try the smallest dependency change that adds Java/C++ while keeping existing Python grammar pinned:

```toml
[workspace.dependencies]
thiserror = "1"
clap = { version = "4", features = ["derive"] }
tree-sitter = "0.24"
tree-sitter-language = "0.1"
tree-sitter-python = "0.21"
tree-sitter-java = "0.23.5"
tree-sitter-cpp = "0.23.4"
assert_cmd = "2"
predicates = "3"
```

Update `crates/algosketch-core/Cargo.toml`:

```toml
[dependencies]
thiserror.workspace = true
tree-sitter.workspace = true
tree-sitter-language.workspace = true
tree-sitter-python.workspace = true
tree-sitter-java.workspace = true
tree-sitter-cpp.workspace = true
```

Keep Python initialization as `tree_sitter_python::language()` in this task. Use Java/C++ initialization as:

```rust
parser.set_language(&tree_sitter_java::LANGUAGE.into())?;
parser.set_language(&tree_sitter_cpp::LANGUAGE.into())?;
```

If the dependency check fails specifically because `tree-sitter-python 0.21` cannot compile or link with `tree-sitter 0.24`, then update `tree-sitter-python` to the current compatible release in this same dependency task and change Python initialization to that release's `LANGUAGE` API. Do not make that upgrade preemptively.

- [ ] **Step 4: Run dependency verification**

Run:

```bash
cargo check --workspace
cargo test --package algosketch-core grammar_loads
```

Expected: both commands PASS.

- [ ] **Step 5: Run full baseline after dependency changes**

Run:

```bash
cargo test --workspace
```

Expected: PASS with the same existing tests plus the two new grammar-load tests.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/algosketch-core/Cargo.toml crates/algosketch-core/src/parser.rs
git commit -m "build(core): add Java and C++ tree-sitter grammars"
```

---

### Task 2: Split parser.rs into parser modules without behavior changes

**Files:**
- Create: `crates/algosketch-core/src/parser/mod.rs`
- Create: `crates/algosketch-core/src/parser/common.rs`
- Create: `crates/algosketch-core/src/parser/python.rs`
- Remove: `crates/algosketch-core/src/parser.rs`

- [ ] **Step 1: Move existing Python parser**

Run:

```bash
mkdir -p crates/algosketch-core/src/parser
git mv crates/algosketch-core/src/parser.rs crates/algosketch-core/src/parser/python.rs
```

- [ ] **Step 2: Create `parser/mod.rs`**

Create `crates/algosketch-core/src/parser/mod.rs`:

```rust
use crate::error::Result;
use crate::ir::Module;
use crate::SourceLang;

pub mod common;
pub mod cpp;
pub mod java;
pub mod python;

pub use cpp::CppParser;
pub use java::JavaParser;
pub use python::PythonParser;

pub trait LanguageParser {
    fn language(&self) -> SourceLang;
    fn parse(&self, source: &str) -> Result<Module>;
}
```

- [ ] **Step 3: Create placeholder Java/C++ parser modules**

Create `crates/algosketch-core/src/parser/java.rs`:

```rust
use crate::error::{PseudoError, Result};
use crate::ir::Module;
use crate::parser::LanguageParser;
use crate::SourceLang;

pub struct JavaParser;

impl JavaParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JavaParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for JavaParser {
    fn language(&self) -> SourceLang {
        SourceLang::Java
    }

    fn parse(&self, _source: &str) -> Result<Module> {
        Err(PseudoError::UnsupportedLanguage("java".to_string()))
    }
}
```

Create `crates/algosketch-core/src/parser/cpp.rs` with the same shape and `SourceLang::Cpp` / `"cpp"`.

- [ ] **Step 4: Move shared helper candidates into `common.rs`**

Move these helper functions from `python.rs` to `common.rs` and make them `pub(crate)`:

```rust
use crate::error::{PseudoError, Result};
use crate::ir::{BinOp, UnOp};

pub(crate) fn node_text<'a>(source: &'a str, node: tree_sitter::Node<'_>) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

pub(crate) fn parse_err(msg: impl Into<String>) -> PseudoError {
    PseudoError::Parse {
        file: "input".into(),
        message: msg.into(),
    }
}

pub(crate) fn named_child_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.is_named() && c.kind() == kind)
}

pub(crate) fn find_anon_operator<'a>(
    source: &'a str,
    node: tree_sitter::Node<'_>,
) -> Option<&'a str> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if !child.is_named() {
                let text = node_text(source, child);
                if !["(", ")", "[", "]", "{", "}", ",", ".", ":", ";"].contains(&text) {
                    return Some(text);
                }
            }
        }
    }
    None
}

pub(crate) fn parse_bin_op(text: &str) -> Result<BinOp> {
    match text {
        "+" => Ok(BinOp::Add),
        "-" => Ok(BinOp::Sub),
        "*" => Ok(BinOp::Mul),
        "/" => Ok(BinOp::Div),
        "//" => Ok(BinOp::IntDiv),
        "%" => Ok(BinOp::Mod),
        "&&" | "and" => Ok(BinOp::And),
        "||" | "or" => Ok(BinOp::Or),
        "&" => Ok(BinOp::BitAnd),
        "|" => Ok(BinOp::BitOr),
        "^" => Ok(BinOp::BitXor),
        "<<" => Ok(BinOp::Shl),
        ">>" => Ok(BinOp::Shr),
        _ => Err(parse_err(format!("unknown binary operator: {text}"))),
    }
}

pub(crate) fn parse_comparison_op(text: &str) -> Result<BinOp> {
    match text {
        "==" => Ok(BinOp::Eq),
        "!=" => Ok(BinOp::Ne),
        "<" => Ok(BinOp::Lt),
        "<=" => Ok(BinOp::Le),
        ">" => Ok(BinOp::Gt),
        ">=" => Ok(BinOp::Ge),
        _ => Err(parse_err(format!("unknown comparison operator: {text}"))),
    }
}

pub(crate) fn parse_un_op(text: &str) -> Result<UnOp> {
    match text {
        "-" => Ok(UnOp::Neg),
        "!" | "not" => Ok(UnOp::Not),
        "~" => Ok(UnOp::BitNot),
        _ => Err(parse_err(format!("unknown unary operator: {text}"))),
    }
}
```

- [ ] **Step 5: Update imports in `python.rs`**

At the top of `crates/algosketch-core/src/parser/python.rs`, keep only Python-specific imports and pull helper functions from `common`:

```rust
use crate::error::{PseudoError, Result};
use crate::ir::*;
use crate::parser::common::{
    find_anon_operator, named_child_by_kind, node_text, parse_bin_op, parse_comparison_op,
    parse_err, parse_un_op,
};
use crate::parser::LanguageParser;
use crate::SourceLang;
```

Remove the local copies of helper functions moved to `common.rs`.

- [ ] **Step 6: Run tests**

Run:

```bash
cargo test --workspace
```

Expected: PASS. Java/C++ parser modules still return unsupported; this task is a refactor only.

- [ ] **Step 7: Commit**

```bash
git add crates/algosketch-core/src/parser crates/algosketch-core/src/parser.rs
git commit -m "refactor(core): split parser adapters into modules"
```

---

## Chunk 2: Python Gap Closure for Cross-language Fixtures

### Task 3: Add Python for-loop, break, and continue parsing

**Files:**
- Modify: `crates/algosketch-core/src/parser/python.rs`
- Modify: `crates/algosketch-core/src/renderer/pseudo.rs` only if tests expose renderer gaps in this task; otherwise defer renderer work to Chunk 5.

- [ ] **Step 1: Write failing Python for/range test**

Add to `python.rs` tests:

```rust
#[test]
fn parses_python_range_for_loop() {
    let source = r#"
def two_sum(nums, target):
    for i in range(0, len(nums)):
        if nums[i] == target:
            return i
    return -1
"#;

    let module = PythonParser::new().parse(source).unwrap();
    let Item::Function(function) = &module.items[0] else {
        panic!("expected function");
    };
    let Stmt::For { kind, body } = &function.body.0[0] else {
        panic!("expected for loop");
    };
    assert!(matches!(kind, ForKind::Range { var, .. } if var == "i"));
    assert!(matches!(body.0[0], Stmt::If { .. }));
}
```

- [ ] **Step 2: Write failing break/continue test**

```rust
#[test]
fn parses_python_break_and_continue() {
    let source = r#"
def scan(nums):
    while True:
        continue
        break
"#;

    let module = PythonParser::new().parse(source).unwrap();
    let Item::Function(function) = &module.items[0] else {
        panic!("expected function");
    };
    let Stmt::While { body, .. } = &function.body.0[0] else {
        panic!("expected while");
    };
    assert!(matches!(body.0[0], Stmt::Continue));
    assert!(matches!(body.0[1], Stmt::Break));
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test --package algosketch-core parses_python_
```

Expected: FAIL because `for_statement`, `break_statement`, and `continue_statement` are currently parsed as raw.

- [ ] **Step 4: Implement Python statement parsing**

Add arms to `parse_stmt`:

```rust
"for_statement" => parse_for_stmt(source, node),
"break_statement" => Ok(Stmt::Break),
"continue_statement" => Ok(Stmt::Continue),
```

Add helper:

```rust
fn parse_for_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let target = node
        .named_child(0)
        .ok_or_else(|| parse_err("for missing target"))?;
    let iter = node
        .named_child(1)
        .ok_or_else(|| parse_err("for missing iterable"))?;
    let body = node
        .named_child(2)
        .ok_or_else(|| parse_err("for missing body"))?;

    let var = node_text(source, target).to_string();
    let iter_expr = parse_expr(source, iter)?;
    let kind = python_for_kind(var, iter_expr);

    Ok(Stmt::For {
        kind,
        body: parse_block(source, body)?,
    })
}

fn python_for_kind(var: String, iter: Expr) -> ForKind {
    if let Expr::Call { callee, args } = &iter {
        if matches!(callee.as_ref(), Expr::Ident(name) if name == "range") {
            return match args.as_slice() {
                [end] => ForKind::Range {
                    var,
                    start: Expr::Literal(Literal::Int(0)),
                    end: end.clone(),
                    step: None,
                },
                [start, end] => ForKind::Range {
                    var,
                    start: start.clone(),
                    end: end.clone(),
                    step: None,
                },
                [start, end, step] => ForKind::Range {
                    var,
                    start: start.clone(),
                    end: end.clone(),
                    step: Some(step.clone()),
                },
                _ => ForKind::ForEach { var, iter },
            };
        }
    }
    ForKind::ForEach { var, iter }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test --package algosketch-core parses_python_
cargo test --package algosketch-core parser::python
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/algosketch-core/src/parser/python.rs
git commit -m "feat(core): parse Python for loops and loop controls"
```

---

## Chunk 3: Java Parser

### Task 4: Add JavaParser method extraction

**Files:**
- Modify: `crates/algosketch-core/src/parser/java.rs`

- [ ] **Step 1: Write failing Java method shape test**

Add tests to `java.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Item, Stmt};

    #[test]
    fn parses_java_method_shape() {
        let source = r#"
class Solution {
    int answer(int x) {
        return x;
    }
}
"#;

        let module = JavaParser::new().parse(source).unwrap();
        assert_eq!(module.source_language, SourceLang::Java);
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        assert_eq!(function.name, "answer");
        assert_eq!(function.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["x"]);
        assert!(matches!(function.body.0[0], Stmt::Return(_)));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --package algosketch-core parses_java_method_shape
```

Expected: FAIL because `JavaParser::parse` still returns `UnsupportedLanguage`.

- [ ] **Step 3: Implement Java tree-sitter initialization and method traversal**

Replace the placeholder parser with:

```rust
impl LanguageParser for JavaParser {
    fn language(&self) -> SourceLang {
        SourceLang::Java
    }

    fn parse(&self, source: &str) -> Result<Module> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .map_err(|e| PseudoError::Internal(format!("tree-sitter init: {e}")))?;
        let tree = parser.parse(source, None).ok_or_else(|| PseudoError::Parse {
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

        let mut items = Vec::new();
        collect_java_items(source, root, &mut items)?;
        Ok(Module {
            source_language: SourceLang::Java,
            items,
        })
    }
}
```

Add traversal helpers:

```rust
fn collect_java_items(source: &str, node: tree_sitter::Node, items: &mut Vec<Item>) -> Result<()> {
    match node.kind() {
        "method_declaration" => items.push(Item::Function(parse_method(source, node)?)),
        "constructor_declaration" => items.push(Item::Raw(node_text(source, node).to_string())),
        _ => {
            for i in 0..node.named_child_count() {
                collect_java_items(source, node.named_child(i).unwrap(), items)?;
            }
        }
    }
    Ok(())
}

fn parse_method(source: &str, node: tree_sitter::Node) -> Result<Function> {
    let name = node
        .child_by_field_name("name")
        .or_else(|| named_child_by_kind(node, "identifier"))
        .map(|n| node_text(source, n).to_string())
        .ok_or_else(|| parse_err("method missing name"))?;

    let params_node = node
        .child_by_field_name("parameters")
        .or_else(|| named_child_by_kind(node, "formal_parameters"))
        .ok_or_else(|| parse_err("method missing parameters"))?;
    let params = parse_java_params(source, params_node);

    let body_node = node
        .child_by_field_name("body")
        .or_else(|| named_child_by_kind(node, "block"))
        .ok_or_else(|| parse_err("method missing body"))?;

    Ok(Function {
        name,
        params,
        return_type: None,
        body: parse_block(source, body_node)?,
        span: Span::default(),
    })
}

fn parse_java_params(source: &str, node: tree_sitter::Node) -> Vec<Param> {
    let mut params = Vec::new();
    for i in 0..node.named_child_count() {
        let child = node.named_child(i).unwrap();
        if child.kind() == "formal_parameter" || child.kind() == "spread_parameter" {
            if let Some(name) = child.child_by_field_name("name") {
                params.push(Param {
                    name: node_text(source, name).to_string(),
                    type_hint: None,
                });
            }
        }
    }
    params
}
```

Stub `parse_block`/`parse_stmt` enough for return statements; unsupported statements should become `Stmt::Raw`.

- [ ] **Step 4: Run test**

```bash
cargo test --package algosketch-core parses_java_method_shape
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/java.rs
git commit -m "feat(core): extract Java methods into IR functions"
```

---

### Task 5: Parse Java binary-search statements and expressions

**Files:**
- Modify: `crates/algosketch-core/src/parser/java.rs`

- [ ] **Step 1: Write failing Java binary search test**

```rust
#[test]
fn parses_java_binary_search_control_flow() {
    let source = r#"
class Solution {
    int binary_search(int[] nums, int target) {
        int left = 0;
        int right = nums.length - 1;
        while (left <= right) {
            int mid = (left + right) / 2;
            if (nums[mid] == target) {
                return mid;
            } else if (nums[mid] < target) {
                left = mid + 1;
            } else {
                right = mid - 1;
            }
        }
        return -1;
    }
}
"#;

    let module = JavaParser::new().parse(source).unwrap();
    let Item::Function(function) = &module.items[0] else {
        panic!("expected function");
    };
    assert_eq!(function.name, "binary_search");
    assert!(matches!(function.body.0[0], Stmt::VarDecl(_)));
    assert!(matches!(function.body.0[2], Stmt::While { .. }));
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --package algosketch-core parses_java_binary_search_control_flow
```

Expected: FAIL because Java statement/expression parsing is incomplete.

- [ ] **Step 3: Implement Java statement subset**

`parse_stmt` should cover:

```rust
match node.kind() {
    "block" => Ok(Stmt::Raw(node_text(source, node).to_string())),
    "local_variable_declaration" => parse_local_var_decl(source, node),
    "expression_statement" => parse_java_expression_statement(source, node),
    "while_statement" => parse_while_stmt(source, node),
    "if_statement" => parse_if_stmt(source, node),
    "return_statement" => parse_return_stmt(source, node),
    "break_statement" => Ok(Stmt::Break),
    "continue_statement" => Ok(Stmt::Continue),
    _ => Ok(Stmt::Raw(node_text(source, node).to_string())),
}
```

Implementation rules:

- `local_variable_declaration` with one declarator becomes `Stmt::VarDecl`.
- Multiple declarators may become `Stmt::Raw` for MVP.
- `expression_statement` containing `assignment_expression` becomes `Stmt::Assign`.
- Other expression statements become `Stmt::ExprStmt`.
- `if_statement` should lower `else if` as `else_block = Some(Block(vec![Stmt::If { .. }]))`, matching Python's existing shape.

- [ ] **Step 4: Implement Java expression subset**

`parse_expr` should cover:

```rust
"identifier" => Expr::Ident(...)
"decimal_integer_literal" | "hex_integer_literal" => Expr::Literal(Literal::Int(...)) or Raw on parse failure
"true" => Expr::Literal(Literal::Bool(true))
"false" => Expr::Literal(Literal::Bool(false))
"null_literal" => Expr::Literal(Literal::None)
"binary_expression" => Expr::Binary { ... }
"parenthesized_expression" => inner expression
"method_invocation" => Expr::Call { ... }
"array_access" => Expr::Index { ... }
"field_access" => Expr::Field { ... }
"assignment_expression" => Expr::Raw(original) when used as expression
"update_expression" => Expr::Raw(original)
_ => Expr::Raw(original)
```

When parsing Java `/` inside integer algorithms, map it to `BinOp::IntDiv` for Java/C++ numeric code in MVP because common algorithm samples use integer variables and expect CLRS `DIV` output. Keep this local to Java/C++ parser expression parsing; do not change Python `/` semantics.

- [ ] **Step 5: Run Java parser tests**

```bash
cargo test --package algosketch-core parses_java_
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/algosketch-core/src/parser/java.rs
git commit -m "feat(core): parse Java algorithm control flow"
```

---

### Task 6: Parse Java for and enhanced-for loops

**Files:**
- Modify: `crates/algosketch-core/src/parser/java.rs`

- [ ] **Step 1: Write failing C-style for test**

```rust
#[test]
fn parses_java_cstyle_for_loop() {
    let source = r#"
class Solution {
    int scan(int[] nums, int target) {
        for (int i = 0; i < nums.length; i++) {
            if (nums[i] == target) {
                return i;
            }
        }
        return -1;
    }
}
"#;

    let module = JavaParser::new().parse(source).unwrap();
    let Item::Function(function) = &module.items[0] else {
        panic!("expected function");
    };
    assert!(matches!(function.body.0[0], Stmt::For { kind: ForKind::CStyle { .. }, .. }));
}
```

- [ ] **Step 2: Write failing enhanced-for test**

```rust
#[test]
fn parses_java_enhanced_for_loop() {
    let source = r#"
class Solution {
    int sum(int[] nums) {
        int total = 0;
        for (int value : nums) {
            total = total + value;
        }
        return total;
    }
}
"#;

    let module = JavaParser::new().parse(source).unwrap();
    let Item::Function(function) = &module.items[0] else {
        panic!("expected function");
    };
    assert!(matches!(function.body.0[1], Stmt::For { kind: ForKind::ForEach { .. }, .. }));
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test --package algosketch-core parses_java_
```

Expected: FAIL because Java for loops are raw.

- [ ] **Step 4: Implement `for_statement` and `enhanced_for_statement` parsing**

Add `parse_for_stmt` for Java `for_statement`:

- Extract initializer into `Stmt::VarDecl` or `Stmt::Assign`; raw fallback if shape is unfamiliar.
- Extract condition into `Expr`; raw fallback to `TRUE` is not acceptable, use `Expr::Raw` with original condition text.
- Extract update into `Expr::Raw` unless it is a simple assignment.
- Parse body block.

Add `parse_enhanced_for_stmt` for `enhanced_for_statement`:

- Extract loop variable name.
- Extract iterable expression.
- Emit `ForKind::ForEach { var, iter }`.

- [ ] **Step 5: Run tests**

```bash
cargo test --package algosketch-core parses_java_
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/algosketch-core/src/parser/java.rs
git commit -m "feat(core): parse Java for loops"
```

---

## Chunk 4: C++ Parser

### Task 7: Add CppParser function extraction

**Files:**
- Modify: `crates/algosketch-core/src/parser/cpp.rs`

- [ ] **Step 1: Write failing C++ function shape test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Item, Stmt};

    #[test]
    fn parses_cpp_function_shape() {
        let source = r#"
int answer(int x) {
    return x;
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        assert_eq!(module.source_language, SourceLang::Cpp);
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        assert_eq!(function.name, "answer");
        assert_eq!(function.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["x"]);
        assert!(matches!(function.body.0[0], Stmt::Return(_)));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --package algosketch-core parses_cpp_function_shape
```

Expected: FAIL because `CppParser::parse` still returns unsupported.

- [ ] **Step 3: Implement C++ tree-sitter initialization and function traversal**

Use the same structure as Java, but initialize with:

```rust
parser
    .set_language(&tree_sitter_cpp::LANGUAGE.into())
    .map_err(|e| PseudoError::Internal(format!("tree-sitter init: {e}")))?;
```

Traverse top-level `translation_unit` and collect `function_definition` nodes. Preserve `class_specifier`, `struct_specifier`, `preproc_include`, and unknown top-level nodes as `Item::Raw` only if they are important to output; otherwise skip class/struct/includes for MVP if they would clutter pseudocode output. Prefer skipping includes and preserving structs/classes as `Item::Raw` only when tests require it.

Parse `function_declarator` to get function name and parameters. If name extraction is unclear, fallback to `node_text` for the declarator as `Item::Raw` instead of returning parse error.

- [ ] **Step 4: Run test**

```bash
cargo test --package algosketch-core parses_cpp_function_shape
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/cpp.rs
git commit -m "feat(core): extract C++ functions into IR functions"
```

---

### Task 8: Parse C++ binary-search statements and expressions

**Files:**
- Modify: `crates/algosketch-core/src/parser/cpp.rs`

- [ ] **Step 1: Write failing C++ binary search test**

```rust
#[test]
fn parses_cpp_binary_search_control_flow() {
    let source = r#"
int binary_search(vector<int>& nums, int target) {
    int left = 0;
    int right = nums.size() - 1;
    while (left <= right) {
        int mid = (left + right) / 2;
        if (nums[mid] == target) {
            return mid;
        } else if (nums[mid] < target) {
            left = mid + 1;
        } else {
            right = mid - 1;
        }
    }
    return -1;
}
"#;

    let module = CppParser::new().parse(source).unwrap();
    let Item::Function(function) = &module.items[0] else {
        panic!("expected function");
    };
    assert_eq!(function.name, "binary_search");
    assert!(matches!(function.body.0[0], Stmt::VarDecl(_)));
    assert!(matches!(function.body.0[2], Stmt::While { .. }));
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --package algosketch-core parses_cpp_binary_search_control_flow
```

Expected: FAIL because C++ statement/expression parsing is incomplete.

- [ ] **Step 3: Implement C++ statement subset**

`parse_stmt` should cover:

```rust
match node.kind() {
    "compound_statement" => Ok(Stmt::Raw(node_text(source, node).to_string())),
    "declaration" => parse_cpp_declaration(source, node),
    "expression_statement" => parse_cpp_expression_statement(source, node),
    "while_statement" => parse_while_stmt(source, node),
    "if_statement" => parse_if_stmt(source, node),
    "return_statement" => parse_return_stmt(source, node),
    "break_statement" => Ok(Stmt::Break),
    "continue_statement" => Ok(Stmt::Continue),
    _ => Ok(Stmt::Raw(node_text(source, node).to_string())),
}
```

Implementation rules:

- `declaration` with one `init_declarator` becomes `Stmt::VarDecl`.
- Multi-declarator declarations may be raw for MVP.
- `expression_statement` containing `assignment_expression` becomes `Stmt::Assign`.
- Other expression statements become `Stmt::ExprStmt`.
- `if_statement` lowers `else if` the same way Python/Java do.

- [ ] **Step 4: Implement C++ expression subset**

`parse_expr` should cover:

```rust
"identifier" => Expr::Ident(...)
"number_literal" => Expr::Literal(Literal::Int(...)) or Raw on parse failure
"true" => Expr::Literal(Literal::Bool(true))
"false" => Expr::Literal(Literal::Bool(false))
"nullptr" | "null" => Expr::Literal(Literal::None)
"binary_expression" => Expr::Binary { ... }
"parenthesized_expression" => inner expression
"call_expression" => Expr::Call { ... }
"subscript_expression" => Expr::Index { ... }
"field_expression" => Expr::Field { ... }
"assignment_expression" => Expr::Raw(original) when used as expression
"update_expression" => Expr::Raw(original)
"qualified_identifier" => Expr::Raw(original)
_ => Expr::Raw(original)
```

As with Java, map C++ `/` to `BinOp::IntDiv` for integer algorithm samples.

- [ ] **Step 5: Run C++ parser tests**

```bash
cargo test --package algosketch-core parses_cpp_
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/algosketch-core/src/parser/cpp.rs
git commit -m "feat(core): parse C++ algorithm control flow"
```

---

### Task 9: Parse C++ for and range-based for loops

**Files:**
- Modify: `crates/algosketch-core/src/parser/cpp.rs`

- [ ] **Step 1: Write failing C-style for test**

```rust
#[test]
fn parses_cpp_cstyle_for_loop() {
    let source = r#"
int scan(vector<int>& nums, int target) {
    for (int i = 0; i < nums.size(); i++) {
        if (nums[i] == target) {
            return i;
        }
    }
    return -1;
}
"#;

    let module = CppParser::new().parse(source).unwrap();
    let Item::Function(function) = &module.items[0] else {
        panic!("expected function");
    };
    assert!(matches!(function.body.0[0], Stmt::For { kind: ForKind::CStyle { .. }, .. }));
}
```

- [ ] **Step 2: Write failing range-for test**

```rust
#[test]
fn parses_cpp_range_for_loop() {
    let source = r#"
int sum(vector<int>& nums) {
    int total = 0;
    for (int value : nums) {
        total = total + value;
    }
    return total;
}
"#;

    let module = CppParser::new().parse(source).unwrap();
    let Item::Function(function) = &module.items[0] else {
        panic!("expected function");
    };
    assert!(matches!(function.body.0[1], Stmt::For { kind: ForKind::ForEach { .. }, .. }));
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test --package algosketch-core parses_cpp_
```

Expected: FAIL because C++ for loops are raw.

- [ ] **Step 4: Implement C++ loop parsing**

Parse:

- `for_statement` into `ForKind::CStyle`.
- `for_range_loop` or the actual tree-sitter C++ range-loop node kind into `ForKind::ForEach`. If the node kind differs, inspect `tree_sitter_cpp::NODE_TYPES` or use a temporary debug print in a test; do not guess and leave it broken.

Use raw fallback for complex initializers/updates rather than returning errors.

- [ ] **Step 5: Run tests**

```bash
cargo test --package algosketch-core parses_cpp_
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/algosketch-core/src/parser/cpp.rs
git commit -m "feat(core): parse C++ for loops"
```

---

## Chunk 5: Pseudocode Renderer Support for M3 IR

### Task 10: Render VarDecl and ForKind in PseudoRenderer

**Files:**
- Modify: `crates/algosketch-core/src/renderer/pseudo.rs`

- [ ] **Step 1: Write failing VarDecl render test**

Add to `pseudo.rs` tests:

```rust
#[test]
fn renders_vardecl_with_initializer() {
    let module = Module {
        source_language: SourceLang::Java,
        items: vec![Item::Function(Function {
            name: "answer".to_string(),
            params: vec![],
            return_type: None,
            body: Block(vec![Stmt::VarDecl(VarDecl {
                name: "x".to_string(),
                type_hint: None,
                init: Some(Expr::Literal(Literal::Int(1))),
            })]),
            span: Span::default(),
        })],
    };

    let out = PseudoRenderer::new().render_module(&module);
    assert!(out.contains("x ← 1"), "unexpected output:\n{out}");
    assert!(!out.contains("<unsupported stmt>"), "unexpected output:\n{out}");
}
```

- [ ] **Step 2: Write failing ForKind render tests**

```rust
#[test]
fn renders_for_each_loop() {
    let function = Function {
        name: "sum".to_string(),
        params: vec![],
        return_type: None,
        body: Block(vec![Stmt::For {
            kind: ForKind::ForEach {
                var: "x".to_string(),
                iter: Expr::Ident("nums".to_string()),
            },
            body: Block(vec![Stmt::ExprStmt(Expr::Ident("visit".to_string()))]),
        }]),
        span: Span::default(),
    };
    let out = PseudoRenderer::new().render_function(&function);
    assert!(out.contains("FOR EACH x IN nums"), "unexpected output:\n{out}");
    assert!(out.contains("END FOR"), "unexpected output:\n{out}");
}

#[test]
fn renders_range_loop() {
    let function = Function {
        name: "scan".to_string(),
        params: vec![],
        return_type: None,
        body: Block(vec![Stmt::For {
            kind: ForKind::Range {
                var: "i".to_string(),
                start: Expr::Literal(Literal::Int(0)),
                end: Expr::Ident("n".to_string()),
                step: None,
            },
            body: Block(vec![Stmt::Continue]),
        }]),
        span: Span::default(),
    };
    let out = PseudoRenderer::new().render_function(&function);
    assert!(out.contains("FOR i ← 0 TO n - 1"), "unexpected output:\n{out}");
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test --package algosketch-core renderer::pseudo
```

Expected: FAIL because `Stmt::For` and `Stmt::VarDecl` render as unsupported.

- [ ] **Step 4: Implement renderer arms**

Replace the unsupported arm in `render_stmt`:

```rust
Stmt::VarDecl(var) => {
    if let Some(init) = &var.init {
        out.push_str(&format!("{pad}{} ← {}\n", var.name, render_expr(init)));
    } else {
        out.push_str(&format!("{pad}DECLARE {}\n", var.name));
    }
}
Stmt::For { kind, body } => {
    self.render_for(kind, body, depth, out);
}
```

Add helper methods:

```rust
fn render_for(&self, kind: &ForKind, body: &Block, depth: usize, out: &mut String) {
    let pad = self.pad(depth);
    match kind {
        ForKind::ForEach { var, iter } => {
            out.push_str(&format!("{pad}FOR EACH {var} IN {}\n", render_expr(iter)));
        }
        ForKind::Range { var, start, end, step } => {
            let end_text = format!("{} - 1", render_expr(end));
            match step {
                Some(step) => out.push_str(&format!(
                    "{pad}FOR {var} ← {} TO {end_text} STEP {}\n",
                    render_expr(start),
                    render_expr(step)
                )),
                None => out.push_str(&format!(
                    "{pad}FOR {var} ← {} TO {end_text}\n",
                    render_expr(start)
                )),
            }
        }
        ForKind::CStyle { init, cond, step } => {
            out.push_str(&format!(
                "{pad}FOR {}; {}; {}\n",
                render_stmt_inline(init),
                render_expr(cond),
                render_expr(step)
            ));
        }
    }
    self.render_block(body, depth + 1, out);
    out.push_str(&format!("{pad}END FOR\n"));
}

fn render_stmt_inline(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Assign { target, value } => format!("{} ← {}", render_expr(target), render_expr(value)),
        Stmt::VarDecl(var) => match &var.init {
            Some(init) => format!("{} ← {}", var.name, render_expr(init)),
            None => format!("DECLARE {}", var.name),
        },
        Stmt::ExprStmt(expr) => render_expr(expr),
        Stmt::Raw(text) => text.clone(),
        _ => "<stmt>".to_string(),
    }
}
```

- [ ] **Step 5: Run renderer tests**

```bash
cargo test --package algosketch-core renderer::pseudo
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/algosketch-core/src/renderer/pseudo.rs
git commit -m "feat(core): render declarations and for loops in pseudocode"
```

---

## Chunk 6: Cross-language Fixtures and Skeleton Tests

### Task 11: Add core cross-language fixtures

**Files:**
- Create all 15 files under `crates/algosketch-core/tests/fixtures/` listed in File Structure.

- [ ] **Step 1: Create fixture directory**

```bash
mkdir -p crates/algosketch-core/tests/fixtures
```

- [ ] **Step 2: Add `binary_search` fixtures**

Use while-loop forms in all three languages.

`crates/algosketch-core/tests/fixtures/binary_search.py`:

```python
def binary_search(nums, target):
    left = 0
    right = len(nums) - 1
    while left <= right:
        mid = (left + right) // 2
        if nums[mid] == target:
            return mid
        elif nums[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1
```

`crates/algosketch-core/tests/fixtures/binary_search.java`:

```java
class Solution {
    int binary_search(int[] nums, int target) {
        int left = 0;
        int right = nums.length - 1;
        while (left <= right) {
            int mid = (left + right) / 2;
            if (nums[mid] == target) {
                return mid;
            } else if (nums[mid] < target) {
                left = mid + 1;
            } else {
                right = mid - 1;
            }
        }
        return -1;
    }
}
```

`crates/algosketch-core/tests/fixtures/binary_search.cpp`:

```cpp
int binary_search(vector<int>& nums, int target) {
    int left = 0;
    int right = nums.size() - 1;
    while (left <= right) {
        int mid = (left + right) / 2;
        if (nums[mid] == target) {
            return mid;
        } else if (nums[mid] < target) {
            left = mid + 1;
        } else {
            right = mid - 1;
        }
    }
    return -1;
}
```

- [ ] **Step 3: Add remaining algorithm fixtures**

Keep the fixtures intentionally simple and parser-friendly:

- `reverse_string`: two-pointer while loop with `left`, `right`, `temp`, assignments, return.
- `reverse_linked_list`: `prev`, `current`, `next_node`, while loop, pointer rewiring, return `prev`.
- `quick_sort`: keep this parser-friendly. Prefer a recursive `quick_sort(nums, low, high)` using one simple `partition(nums, low, high)` helper, assignments, while/for, and returns. If the first fixture shape forces broad parser scope, simplify the fixture rather than expanding into full Java/C++ semantics.
- `two_sum`: nested for loops returning two indices, avoiding maps/dicts so all languages share the same control-flow skeleton.

Do not use language features outside M3 scope in fixtures: no Java generics-heavy collections, no C++ templates beyond simple `vector<int>&` parameters, no Python list comprehensions, no exceptions, no lambdas. For `reverse_linked_list`, use field access (`current.next`) only if Java/C++ parsers already cover member/field syntax; otherwise make it a minimal pointer-rewiring fixture and preserve extra type/class declarations outside the skeleton comparison.

- [ ] **Step 4: Commit fixtures**

```bash
git add crates/algosketch-core/tests/fixtures
git commit -m "test(core): add cross-language algorithm fixtures"
```

---

### Task 12: Add cross-language skeleton tests

**Files:**
- Create: `crates/algosketch-core/tests/cross_language.rs`

- [ ] **Step 1: Write failing skeleton test harness**

Create `crates/algosketch-core/tests/cross_language.rs`:

```rust
use algosketch_core::ir::*;
use algosketch_core::parser::{CppParser, JavaParser, LanguageParser, PythonParser};

fn parse_fixture(algorithm: &str, ext: &str) -> Module {
    let path = format!("tests/fixtures/{algorithm}.{ext}");
    let source = std::fs::read_to_string(&path).expect("fixture should exist");
    match ext {
        "py" => PythonParser::new().parse(&source).unwrap(),
        "java" => JavaParser::new().parse(&source).unwrap(),
        "cpp" => CppParser::new().parse(&source).unwrap(),
        _ => panic!("unsupported fixture extension: {ext}"),
    }
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
    let mut out = vec![format!("fn:{}", function.name)];
    block_skeleton(&function.body, &mut out);
    out
}

fn block_skeleton(block: &Block, out: &mut Vec<String>) {
    for stmt in &block.0 {
        match stmt {
            Stmt::Assign { .. } => out.push("assign".to_string()),
            Stmt::VarDecl(_) => out.push("decl".to_string()),
            Stmt::If { then_block, else_block, .. } => {
                out.push("if".to_string());
                block_skeleton(then_block, out);
                if let Some(else_block) = else_block {
                    out.push("else".to_string());
                    block_skeleton(else_block, out);
                }
                out.push("end-if".to_string());
            }
            Stmt::While { body, .. } => {
                out.push("while".to_string());
                block_skeleton(body, out);
                out.push("end-while".to_string());
            }
            Stmt::For { body, .. } => {
                out.push("for".to_string());
                block_skeleton(body, out);
                out.push("end-for".to_string());
            }
            Stmt::Return(_) => out.push("return".to_string()),
            Stmt::Break => out.push("break".to_string()),
            Stmt::Continue => out.push("continue".to_string()),
            Stmt::ExprStmt(_) => out.push("expr".to_string()),
            Stmt::Raw(_) => out.push("raw".to_string()),
        }
    }
}

#[test]
fn cross_language_skeletons_match_for_mvp_fixtures() {
    for algorithm in [
        "binary_search",
        "reverse_string",
        "reverse_linked_list",
        "quick_sort",
        "two_sum",
    ] {
        let py = module_skeleton(&parse_fixture(algorithm, "py"));
        let java = module_skeleton(&parse_fixture(algorithm, "java"));
        let cpp = module_skeleton(&parse_fixture(algorithm, "cpp"));

        assert_eq!(py, java, "Python and Java skeleton differ for {algorithm}");
        assert_eq!(py, cpp, "Python and C++ skeleton differ for {algorithm}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails until all parsers/fixtures align**

```bash
cargo test --package algosketch-core --test cross_language cross_language_skeletons_match_for_mvp_fixtures
```

Expected initially: FAIL for any fixture whose parser coverage or fixture control flow does not align.

- [ ] **Step 3: Fix parser gaps or fixture syntax, not the test**

Allowed fixes:

- Add missing parser support for already-in-scope syntax.
- Simplify fixtures to stay within M3's intended subset.
- Adjust skeleton helper only if it is comparing irrelevant language noise, not to hide true control-flow mismatches.

Not allowed:

- Treat `Raw` as matching a structured loop/if/return.
- Delete an algorithm fixture from the acceptance set.
- Add LLM or heuristic guessing.

- [ ] **Step 4: Run test until it passes**

```bash
cargo test --package algosketch-core --test cross_language
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/tests/cross_language.rs crates/algosketch-core/tests/fixtures crates/algosketch-core/src/parser crates/algosketch-core/src/renderer/pseudo.rs
git commit -m "test(core): verify cross-language algorithm skeletons"
```

---

## Chunk 7: CLI Java/C++ Integration

### Task 13: Dispatch JavaParser and CppParser in CLI

**Files:**
- Modify: `crates/algosketch-cli/src/main.rs`
- Modify: `crates/algosketch-cli/tests/cli.rs`
- Create: `crates/algosketch-cli/fixtures/binary_search.java`
- Create: `crates/algosketch-cli/fixtures/binary_search.cpp`

- [ ] **Step 1: Add user-facing Java/C++ binary_search fixtures**

Copy the core `binary_search.java` and `binary_search.cpp` fixtures into `crates/algosketch-cli/fixtures/`.

- [ ] **Step 2: Write failing Java CLI test**

Add to `crates/algosketch-cli/tests/cli.rs`:

```rust
#[test]
fn java_file_outputs_pseudocode_and_explanation() {
    let fixture = format!("{}/fixtures/binary_search.java", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(fixture)
        .arg("--lang")
        .arg("en")
        .assert()
        .success()
        .stdout(contains("## binary_search"))
        .stdout(contains("FUNCTION binary_search"))
        .stdout(contains("WHILE left ≤ right"))
        .stdout(contains("### Explanation"));
}
```

- [ ] **Step 3: Write failing C++ CLI test**

```rust
#[test]
fn cpp_file_outputs_pseudocode_and_explanation() {
    let fixture = format!("{}/fixtures/binary_search.cpp", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(fixture)
        .arg("--lang")
        .arg("en")
        .assert()
        .success()
        .stdout(contains("## binary_search"))
        .stdout(contains("FUNCTION binary_search"))
        .stdout(contains("WHILE left ≤ right"))
        .stdout(contains("### Explanation"));
}
```

- [ ] **Step 4: Run tests to verify they fail**

```bash
cargo test --package algosketch-cli file_outputs_pseudocode_and_explanation
```

Expected: FAIL because CLI still returns unsupported language.

- [ ] **Step 5: Update CLI parser dispatch**

Update imports:

```rust
use algosketch_core::parser::{CppParser, JavaParser, LanguageParser, PythonParser};
```

Replace parser dispatch in `run`:

```rust
let module = match source_lang {
    SourceLang::Python => PythonParser::new().parse(&source)?,
    SourceLang::Java => JavaParser::new().parse(&source)?,
    SourceLang::Cpp => CppParser::new().parse(&source)?,
};
```

- [ ] **Step 6: Run CLI tests**

```bash
cargo test --package algosketch-cli file_outputs_pseudocode_and_explanation
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/algosketch-cli/src/main.rs crates/algosketch-cli/tests/cli.rs crates/algosketch-cli/fixtures/binary_search.java crates/algosketch-cli/fixtures/binary_search.cpp
git commit -m "feat(cli): support Java and C++ inputs"
```

---

### Task 14: Add stdin source-language tests for Java/C++

**Files:**
- Modify: `crates/algosketch-cli/tests/cli.rs`

- [ ] **Step 1: Add Java stdin test**

```rust
#[test]
fn stdin_accepts_explicit_java_source_lang() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("-")
        .arg("--source-lang")
        .arg("java")
        .arg("--no-explain")
        .write_stdin(r#"
class Solution {
    int answer(int x) {
        return x;
    }
}
"#)
        .assert()
        .success()
        .stdout(contains("FUNCTION answer(x)"))
        .stdout(contains("RETURN x"));
}
```

- [ ] **Step 2: Add C++ stdin test**

```rust
#[test]
fn stdin_accepts_explicit_cpp_source_lang() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("-")
        .arg("--source-lang")
        .arg("cpp")
        .arg("--no-explain")
        .write_stdin(r#"
int answer(int x) {
    return x;
}
"#)
        .assert()
        .success()
        .stdout(contains("FUNCTION answer(x)"))
        .stdout(contains("RETURN x"));
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test --package algosketch-cli stdin_accepts_explicit_
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/algosketch-cli/tests/cli.rs
git commit -m "test(cli): cover Java and C++ stdin parsing"
```

---

## Chunk 8: MVP Raw Fallback Warnings and CLI Polish

### Task 15: Add Raw fallback diagnostics in core

**Files:**
- Create: `crates/algosketch-core/src/diagnostics.rs`
- Modify: `crates/algosketch-core/src/lib.rs`

- [ ] **Step 1: Write failing diagnostics tests**

Create `crates/algosketch-core/src/diagnostics.rs`:

```rust
use crate::ir::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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

pub fn collect_raw_stats(_module: &Module) -> RawStats {
    RawStats::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceLang;

    #[test]
    fn counts_raw_nodes() {
        let module = Module {
            source_language: SourceLang::Python,
            items: vec![Item::Function(Function {
                name: "f".to_string(),
                params: vec![],
                return_type: None,
                body: Block(vec![
                    Stmt::Raw("yield x".to_string()),
                    Stmt::Assign {
                        target: Expr::Ident("x".to_string()),
                        value: Expr::Raw("a if cond else b".to_string()),
                    },
                ]),
                span: Span::default(),
            })],
        };

        let stats = collect_raw_stats(&module);
        assert_eq!(stats.statements, 1);
        assert_eq!(stats.expressions, 1);
        assert_eq!(stats.total(), 2);
    }
}
```

- [ ] **Step 2: Export diagnostics and run failing test**

Add to `crates/algosketch-core/src/lib.rs`:

```rust
pub mod diagnostics;
```

Run:

```bash
cargo test --package algosketch-core counts_raw_nodes
```

Expected: FAIL because `collect_raw_stats` returns zero.

- [ ] **Step 3: Implement traversal**

Implement recursive traversal for all `Item`, `Stmt`, `ForKind`, and `Expr` variants. Count:

- `Item::Raw(_)` as `items`.
- `Stmt::Raw(_)` as `statements`.
- `Expr::Raw(_)` as `expressions`.

Do not add line numbers in M5 unless spans are added to `Stmt` and `Expr`; current IR does not carry enough span information for accurate line reporting. Count-only warnings satisfy MVP fallback visibility without expanding IR scope.

- [ ] **Step 4: Run tests**

```bash
cargo test --package algosketch-core counts_raw_nodes
cargo test --package algosketch-core
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/diagnostics.rs crates/algosketch-core/src/lib.rs
git commit -m "feat(core): count raw fallback nodes"
```

---

### Task 16: Emit Raw fallback warnings from CLI

**Files:**
- Modify: `crates/algosketch-cli/src/main.rs`
- Modify: `crates/algosketch-cli/tests/cli.rs`

- [ ] **Step 1: Add `--quiet` CLI flag test**

```rust
#[test]
fn raw_fallback_emits_warning_by_default() {
    let fixture = write_temp_python_file(
        "raw-warning",
        r#"
def values(nums):
    yield nums
"#,
    );
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(fixture.path())
        .assert()
        .success()
        .stderr(contains("warning:"))
        .stderr(contains("unparsed"));
}

#[test]
fn quiet_suppresses_raw_fallback_warning() {
    let fixture = write_temp_python_file(
        "quiet-raw-warning",
        r#"
def values(nums):
    yield nums
"#,
    );
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(fixture.path())
        .arg("--quiet")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --package algosketch-cli raw_fallback
```

Expected: FAIL because `--quiet` and warnings do not exist yet.

- [ ] **Step 3: Add CLI flag**

Add to `Cli`:

```rust
/// Suppress non-fatal warnings.
#[arg(short = 'q', long = "quiet")]
quiet: bool,
```

- [ ] **Step 4: Emit warning after parsing**

Import:

```rust
use algosketch_core::diagnostics::collect_raw_stats;
```

After `module` is parsed and before rendering:

```rust
let raw_stats = collect_raw_stats(&module);
if !cli.quiet && raw_stats.total() > 0 {
    eprintln!(
        "warning: {} unparsed nodes preserved as raw fallback (items: {}, statements: {}, expressions: {})",
        raw_stats.total(),
        raw_stats.items,
        raw_stats.statements,
        raw_stats.expressions
    );
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test --package algosketch-cli raw_fallback
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/algosketch-cli/src/main.rs crates/algosketch-cli/tests/cli.rs
git commit -m "feat(cli): warn about raw fallback nodes"
```

---

### Task 17: Handle all-output-disabled CLI case

**Scope:** Optional M5 polish. Do this after M3 and Raw fallback warnings are working; it is not required to prove Java/C++ parser support.

**Files:**
- Modify: `crates/algosketch-cli/src/main.rs`
- Modify: `crates/algosketch-cli/tests/cli.rs`

- [ ] **Step 1: Write failing CLI test**

```rust
#[test]
fn disabling_both_outputs_returns_user_error() {
    let fixture = format!("{}/fixtures/binary_search.py", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(fixture)
        .arg("--no-pseudo")
        .arg("--no-explain")
        .assert()
        .code(1)
        .stderr(contains("at least one output"));
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --package algosketch-cli disabling_both_outputs_returns_user_error
```

Expected: FAIL because command currently succeeds with empty output.

- [ ] **Step 3: Add user error variant**

Modify `PseudoError` in `crates/algosketch-core/src/error.rs`:

```rust
#[error("invalid options: {0}")]
InvalidOptions(String),
```

Update `exit_code_for` in CLI:

```rust
PseudoError::InvalidOptions(_) => 1,
```

In `run`, after calculating output toggles:

```rust
if !show_pseudo && !show_explain {
    return Err(PseudoError::InvalidOptions(
        "at least one output must be enabled".to_string(),
    ));
}
```

- [ ] **Step 4: Run test**

```bash
cargo test --package algosketch-cli disabling_both_outputs_returns_user_error
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/error.rs crates/algosketch-cli/src/main.rs crates/algosketch-cli/tests/cli.rs
git commit -m "fix(cli): reject disabling all outputs"
```

---

## Chunk 9: Documentation and MVP Validation

### Task 18: Update README for v0.1 MVP reality

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update status line**

Change:

```markdown
**Status / 状态**: Pre-alpha — scaffolding only.
```

To:

```markdown
**Status / 状态**: v0.1 MVP — rule-based single-file algorithm pseudocode and explanation for a focused Python/Java/C++ subset.
```

- [ ] **Step 2: Update usage wording**

Change `### Planned usage` / `### 计划中的用法` to `### Usage` / `### 用法`.

- [ ] **Step 3: Add scope caveat**

Add after usage examples in English:

```markdown
v0.1 intentionally supports a focused algorithm-code subset: functions/methods, variable declarations, assignments, if/else, while loops, common for-loop forms, returns, break/continue, calls, indexing, fields, and simple expressions. Unsupported syntax is preserved as raw fallback text and reported as a warning.
```

Add Chinese equivalent:

```markdown
v0.1 有意只覆盖算法代码中的常见子集：函数/方法、变量声明、赋值、if/else、while、常见 for 循环、return、break/continue、调用、下标、字段访问和简单表达式。不支持的语法会以 raw fallback 原文保留，并通过 warning 提示。
```

- [ ] **Step 4: Update examples if output changed**

Run:

```bash
cargo run -- crates/algosketch-cli/fixtures/binary_search.py --lang en > /tmp/algosketch-readme-en.md
cargo run -- crates/algosketch-cli/fixtures/binary_search.py --lang zh > /tmp/algosketch-readme-zh.md
```

Use these outputs to keep README examples exact. Do not claim Java/C++ full-language support; say supported fixtures/subset.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: update README for v0.1 MVP scope"
```

---

### Task 19: Final verification suite

**Files:**
- No intended file changes unless verification finds issues.

- [ ] **Step 1: Format check**

```bash
cargo fmt --check
```

Expected: PASS.

If it fails:

```bash
cargo fmt
git add Cargo.toml Cargo.lock crates/ README.md
git commit -m "style: apply cargo fmt"
```

- [ ] **Step 2: Clippy**

```bash
cargo clippy --workspace -- -D warnings
```

Expected: PASS.

If it fails, fix the exact warnings and commit:

```bash
git add <changed-files>
git commit -m "fix: address clippy warnings"
```

- [ ] **Step 3: Full tests**

```bash
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 4: Manual CLI smoke tests**

```bash
cargo run -- crates/algosketch-cli/fixtures/binary_search.py --lang zh --no-explain
cargo run -- crates/algosketch-cli/fixtures/binary_search.java --lang en --no-explain
cargo run -- crates/algosketch-cli/fixtures/binary_search.cpp --lang en --no-explain
cargo run -- crates/algosketch-cli/fixtures/binary_search.py --lang zh --no-pseudo
cargo run -- crates/algosketch-cli/fixtures/binary_search.java --lang en
cargo run -- crates/algosketch-cli/fixtures/binary_search.cpp --lang en
```

Expected:

- Python, Java, and C++ commands exit 0.
- Pseudocode commands contain `FUNCTION binary_search` and `WHILE left ≤ right`.
- Explanation commands contain `Purpose:` or `目的：` and `Steps:` or `步骤：`.
- No command emits `unsupported language` for Java/C++.

- [ ] **Step 5: Optional real-world smoke test if file still exists**

```bash
cargo run -- "$HOME/Desktop/astar_maze_demo.py" --lang zh >/tmp/algosketch-astar.md
```

Expected: exit 0. It may emit Raw fallback warnings for unsupported Python syntax; that is acceptable for MVP if output is still produced. This is a local confidence check only, not a release gate, because the file is not part of the repository.

- [ ] **Step 6: Git status**

```bash
git status --short
```

Expected: clean working tree, except intentionally uncommitted planning/session files if the user has not asked to commit them.

- [ ] **Step 7: Commit final fixes if any**

```bash
git add <changed-files>
git commit -m "fix: finalize M3 MVP validation"
```

---

## Implementation Notes

- Keep Java/C++ parser support syntax-level only. Do not add type inference, name resolution, imports/includes handling, semantic analysis, or LLM fallback.
- Favor `Expr::Raw` / `Stmt::Raw` over parse failure for well-formed syntax outside scope.
- Do return `PseudoError::Parse` for unrecoverable tree-sitter syntax errors (`root.has_error()`).
- Keep fixtures parser-friendly rather than expanding parser scope to arbitrary language constructs.
- Do not refactor renderers beyond the `VarDecl`/`For` support needed for M3 output.
- Avoid adding comments unless a tree-sitter node-shape workaround would surprise a future reader.
- Commit messages must not include `Co-Authored-By` lines.

## Completion Handoff

After Task 19 passes:

1. Run @superpowers:requesting-code-review before declaring the branch complete.
2. If review passes, use @superpowers:finishing-a-development-branch to decide whether to open a PR, merge locally, or continue polish.
3. Final user report must include exactly which validation commands ran, which passed/failed, and any unverified paths.
