# algosketch M1-M2 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first runnable algosketch demo: CLI skeleton plus Python `binary_search.py` → IR → CLRS-style pseudocode output.

**Architecture:** `algosketch-core` owns source language detection, IR types, Python parsing, and rendering. `algosketch-cli` owns command-line parsing, file/stdin IO, and writing stdout/files. M2 deliberately supports Python + pseudocode only; Java/C++, explanations, Markdown polish, and provider support remain future milestones.

**Tech Stack:** Rust 2021, Cargo workspace, `clap`, `thiserror`, `tree-sitter`, `tree-sitter-python`, integration tests with `assert_cmd` later.

---

## Chunk 1: M1 CLI Skeleton

### Task 1: Core error model and source language helper

**Files:**
- Modify: `crates/algosketch-core/src/lib.rs`
- Create: `crates/algosketch-core/src/error.rs`
- Test: `crates/algosketch-core/src/lib.rs` unit tests

- [ ] **Step 1: Write failing tests**

Add tests for extension detection:

```rust
#[cfg(test)]
mod tests {
    use super::SourceLang;

    #[test]
    fn detects_supported_extensions() {
        assert_eq!(SourceLang::from_extension("py"), Some(SourceLang::Python));
        assert_eq!(SourceLang::from_extension("java"), Some(SourceLang::Java));
        assert_eq!(SourceLang::from_extension("cpp"), Some(SourceLang::Cpp));
        assert_eq!(SourceLang::from_extension("hpp"), Some(SourceLang::Cpp));
    }

    #[test]
    fn rejects_unknown_extensions() {
        assert_eq!(SourceLang::from_extension("rs"), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p algosketch-core detects_supported_extensions rejects_unknown_extensions`

Expected: compile failure or failed tests because `SourceLang::from_extension` is missing.

- [ ] **Step 3: Implement minimal code**

Add `PseudoError`, `Result<T>`, and `SourceLang::from_extension` / `as_str`.

- [ ] **Step 4: Verify green**

Run: `cargo test -p algosketch-core`

Expected: PASS.

### Task 2: CLI parses documented flags

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/algosketch-cli/Cargo.toml`
- Modify: `crates/algosketch-cli/src/main.rs`
- Test: `cargo run -q -p algosketch-cli -- --help`

- [ ] **Step 1: Write failing behavior check**

Run: `cargo run -q -p algosketch-cli -- --help`

Expected before implementation: help text does not include `--source-lang`, `--no-pseudo`, `--no-explain`, `--lang`, `--format`, `--output`.

- [ ] **Step 2: Implement minimal CLI skeleton**

Add `clap` derive parser with:

- positional `<INPUT>`
- `--source-lang python|java|cpp`
- `--no-pseudo`, `--no-explain`, `--pseudo-only`, `--explain-only`
- `--lang zh|en|auto`
- `--format md|text`
- `--output`, `--indent`, `--quiet`

`run()` can return `PseudoError::Internal("M1 skeleton only...")` for now.

- [ ] **Step 3: Verify green**

Run:

```bash
cargo build
cargo run -q -p algosketch-cli -- --help
cargo run -q -p algosketch-cli -- --version
cargo test --workspace
```

Expected: build/test pass; help includes documented flags.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/algosketch-core crates/algosketch-cli docs/superpowers/plans/2026-05-20-m1-m2-implementation.md
git commit -m "feat: add M1 CLI skeleton"
```

---

## Chunk 2: M2a IR Types

### Task 3: Define IR data structures

**Files:**
- Create: `crates/algosketch-core/src/ir.rs`
- Modify: `crates/algosketch-core/src/lib.rs`
- Test: `crates/algosketch-core/src/ir.rs` unit tests

- [ ] **Step 1: Write failing test**

Add a test that constructs a `Module` containing `binary_search` with a `While` statement and asserts the shape.

```rust
#[test]
fn can_model_binary_search_shape() {
    let module = Module {
        source_language: SourceLang::Python,
        items: vec![Item::Function(Function {
            name: "binary_search".into(),
            params: vec![],
            return_type: None,
            body: Block(vec![Stmt::While {
                cond: Expr::Binary {
                    op: BinOp::Le,
                    lhs: Box::new(Expr::Ident("left".into())),
                    rhs: Box::new(Expr::Ident("right".into())),
                },
                body: Block(vec![]),
            }]),
            span: Span::default(),
        })],
    };

    assert_eq!(module.items.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p algosketch-core can_model_binary_search_shape`

Expected: compile failure because IR types do not exist.

- [ ] **Step 3: Implement minimal IR**

Add the IR structs/enums from the spec with `Debug`, `Clone`, `PartialEq`, `Eq` where appropriate.

- [ ] **Step 4: Verify green**

Run: `cargo test -p algosketch-core`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/ir.rs crates/algosketch-core/src/lib.rs
git commit -m "feat: add core IR types"
```

---

## Chunk 3: M2b Python Parser

### Task 4: Python parser extracts a function and while/if/return statements

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/algosketch-core/Cargo.toml`
- Create: `crates/algosketch-core/src/parser.rs`
- Modify: `crates/algosketch-core/src/lib.rs`
- Test: `crates/algosketch-core/src/parser.rs` unit tests

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn parses_python_binary_search_function_shape() {
    let source = r#"
def binary_search(nums, target):
    left, right = 0, len(nums) - 1
    while left <= right:
        mid = (left + right) // 2
        if nums[mid] == target:
            return mid
        elif nums[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1
"#;

    let module = PythonParser::new().parse(source).unwrap();
    let Item::Function(function) = &module.items[0] else { panic!("expected function"); };

    assert_eq!(function.name, "binary_search");
    assert_eq!(function.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["nums", "target"]);
    assert!(matches!(function.body.0[1], Stmt::While { .. }));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p algosketch-core parses_python_binary_search_function_shape`

Expected: compile failure because parser module / `PythonParser` missing.

- [ ] **Step 3: Implement minimal parser**

Add:

```rust
pub trait LanguageParser {
    fn language(&self) -> SourceLang;
    fn parse(&self, source: &str) -> Result<Module>;
}

pub struct PythonParser;
```

Use `tree_sitter::Parser`, `tree_sitter_python::language()`, and CST traversal. Implement only enough for `binary_search.py`:

- module → top-level `function_definition`
- function params → identifiers in `parameters`
- block → statements
- `assignment`
- `while_statement`
- `if_statement` including `elif_clause` / `else_clause`
- `return_statement`
- expressions: identifiers, integers, calls, attributes, subscripts, binary/unary expressions, tuples, raw fallback

- [ ] **Step 4: Verify green**

Run: `cargo test -p algosketch-core parses_python_binary_search_function_shape`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/algosketch-core
 git commit -m "feat: parse Python functions into IR"
```

---

## Chunk 4: M2c Pseudocode Renderer

### Task 5: Render binary_search IR as CLRS-style pseudocode

**Files:**
- Create: `crates/algosketch-core/src/renderer.rs`
- Modify: `crates/algosketch-core/src/lib.rs`
- Test: `crates/algosketch-core/src/renderer.rs` unit tests

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn renders_binary_search_pseudocode() {
    let source = r#"
def binary_search(nums, target):
    left, right = 0, len(nums) - 1
    while left <= right:
        mid = (left + right) // 2
        if nums[mid] == target:
            return mid
        elif nums[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1
"#;

    let module = PythonParser::new().parse(source).unwrap();
    let out = PseudoRenderer::default().render_module(&module);

    assert!(out.contains("FUNCTION binary_search(nums, target)"));
    assert!(out.contains("WHILE left ≤ right"));
    assert!(out.contains("mid ← (left + right) DIV 2"));
    assert!(out.contains("IF nums[mid] = target THEN"));
    assert!(out.contains("RETURN -1"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p algosketch-core renders_binary_search_pseudocode`

Expected: compile failure because renderer missing.

- [ ] **Step 3: Implement minimal renderer**

Render:

- `Function` as `FUNCTION name(params)` / `END FUNCTION`
- `Assign` as `target ← value`
- `While`, `If` / `ELSE IF` / `ELSE`
- `Return`
- `Expr` with operator normalization: `<=` → `≤`, `==` → `=`, `//` → `DIV`
- `len(x)` → `LENGTH(x)`

- [ ] **Step 4: Verify green**

Run: `cargo test -p algosketch-core renders_binary_search_pseudocode`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/renderer.rs crates/algosketch-core/src/lib.rs
git commit -m "feat: render Python IR as pseudocode"
```

---

## Chunk 5: M2d CLI End-to-End Demo

### Task 6: CLI reads `binary_search.py` and prints Markdown pseudocode

**Files:**
- Modify: `crates/algosketch-cli/src/main.rs`
- Create: `crates/algosketch-cli/tests/fixtures/binary_search.py`
- Create: `crates/algosketch-cli/tests/cli.rs`
- Modify: `crates/algosketch-cli/Cargo.toml`
- Test: `cargo test -p algosketch-cli binary_search_file_outputs_pseudocode`

- [ ] **Step 1: Write failing test**

```rust
use assert_cmd::Command;

#[test]
fn binary_search_file_outputs_pseudocode() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("tests/fixtures/binary_search.py")
        .arg("--no-explain")
        .assert()
        .success()
        .stdout(predicates::str::contains("FUNCTION binary_search(nums, target)"))
        .stdout(predicates::str::contains("WHILE left ≤ right"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p algosketch-cli binary_search_file_outputs_pseudocode`

Expected: failure because CLI still returns M1 internal error or test deps missing.

- [ ] **Step 3: Implement minimal CLI pipeline**

- Infer source language from extension.
- Read file or stdin.
- Support Python only for now; Java/C++ return `UnsupportedLanguage`.
- Call `PythonParser::new().parse(&source)`.
- Render `PseudoRenderer`.
- Wrap in Markdown when `--format md`.
- Write stdout or `--output` file.

- [ ] **Step 4: Verify green**

Run:

```bash
cargo test -p algosketch-cli binary_search_file_outputs_pseudocode
cargo run -q -p algosketch-cli -- crates/algosketch-cli/tests/fixtures/binary_search.py --no-explain
cargo test --workspace
```

Expected: all pass; manual command prints Markdown pseudocode.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-cli crates/algosketch-core Cargo.toml Cargo.lock
git commit -m "feat: add Python binary search CLI demo"
```

---

## Final verification for M1-M2

Run:

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo run -q -p algosketch-cli -- crates/algosketch-cli/tests/fixtures/binary_search.py --no-explain
```

Expected:

- fmt clean
- clippy clean
- all tests pass
- demo output contains CLRS-style pseudocode for binary search

Then request code review before claiming M2 complete.
