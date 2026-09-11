# M7 Coverage + Distribution Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Raise parser coverage for the highest-frequency real-world gaps (Python tuple-for/enumerate/classes, Go if-init/two-var-range/switch) and make the crate crates.io-publishable, releasing as v0.2.1.

**Architecture:** Pure parser-layer work inside the existing adapters (python.rs/go.rs) mapping into existing IR variants — no IR, renderer, or trait changes. Distribution is manifest metadata + a package rename (`algosketch-cli` package → `algosketch`).

**Tech Stack:** Rust 2021, tree-sitter 0.25 runtime, assert_cmd, cargo publish dry-run.

**Spec:** `docs/superpowers/specs/2026-09-10-m7-coverage-distribution-design.md` (decisions locked).

**Conventions:**
- Conventional commits; **NEVER `Co-Authored-By`**.
- No push/PR/tag by task executors — the controller handles release (final task).
- Grammar rule: verify node kinds against the actual CST before mapping code (temporary dump test, delete after; see the Go plan's errata for how).
- Cross-language fixtures must NOT change (canary budgets frozen).

**File structure:**

- `crates/algosketch-core/src/parser/python.rs` — tuple-for/enumerate target handling; class method extraction in the top-level walk.
- `crates/algosketch-core/src/parser/go.rs` — if-init structured emission; two-var range ForEach; switch → if/else-if chains.
- `crates/algosketch-cli/tests/cli.rs` — 2 pinning tests.
- `Cargo.toml` + `crates/algosketch-cli/Cargo.toml` + `crates/algosketch-core/Cargo.toml` + `Cargo.lock` — rename + metadata + 0.2.1.
- `README.md`, `docs/superpowers/specs/2026-05-20-algosketch-design.md` — install section, version sync.

---

## Chunk 1: Parser coverage (Tasks 1–5)

### Task 1: Python tuple-for and enumerate loops

**Files:**
- Modify: `crates/algosketch-core/src/parser/python.rs` (`parse_for_stmt` ~line 231; `python_for_kind` ~line 257)
- Test: same file (`mod tests`)

- [ ] **Step 1: Write the failing tests** (append to `mod tests`)

```rust
    #[test]
    fn parses_tuple_unpacking_for() {
        let source = "def f(pairs):\n    for k, v in pairs:\n        g(k)\n";
        let (module, diag) = PythonParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        let Stmt::For { kind: ForKind::ForEach { var, iter }, .. } = &f.body.0[0] else {
            panic!("expected foreach");
        };
        assert_eq!(var, "k, v");
        assert_eq!(iter, &Expr::Ident("pairs".into()));
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn parses_enumerate_for() {
        let source = "def f(xs):\n    for i, x in enumerate(xs):\n        g(x)\n";
        let (module, diag) = PythonParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        let Stmt::For { kind: ForKind::ForEach { var, iter }, .. } = &f.body.0[0] else {
            panic!("expected foreach");
        };
        assert_eq!(var, "i, x");
        assert_eq!(iter, &Expr::Ident("xs".into()));
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn single_var_enumerate_for() {
        let source = "def f(xs):\n    for x in enumerate(xs):\n        g(x)\n";
        let (module, _) = PythonParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        let Stmt::For { kind: ForKind::ForEach { var, iter }, .. } = &f.body.0[0] else {
            panic!("expected foreach");
        };
        assert_eq!(var, "x");
        assert_eq!(iter, &Expr::Ident("xs".into()));
    }

    #[test]
    fn starred_or_nested_tuple_target_stays_raw() {
        let source = "def f(xs):\n    for a, *rest in xs:\n        g(a)\n";
        let (_, diag) = PythonParser::new().parse(source).unwrap();
        assert_eq!(diag.statements, 1);
    }
```

(Adjust `use` items in the tests module as needed — `ForKind`, `Expr` are already available via the file's imports pattern; check sibling tests.)

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p algosketch-core parses_tuple_unpacking_for parses_enumerate_for` (and each other new test by filter)
Expected: the first three FAIL (whole-loop Raw today); the starred one PASSES (pinning).

- [ ] **Step 3: Implement**

In `parse_for_stmt` (~line 243-247, the `let var = match target.kind()` block):

```rust
    let var = match target.kind() {
        "identifier" => node_text(source, target).to_string(),
        "pattern_list" | "tuple" => {
            // flat unpack like `for k, v in pairs` → display string "k, v";
            // starred/nested patterns stay Raw (see spec §2)
            let names: Vec<&str> = (0..target.named_child_count())
                .filter_map(|i| target.named_child(i))
                .filter(|c| c.kind() == "identifier")
                .map(|c| node_text(source, c))
                .collect();
            let total = target.named_child_count();
            if names.len() == total && total >= 2 {
                names.join(", ")
            } else {
                return Ok(record_raw_stmt(source, node, diag));
            }
        }
        _ => return Ok(record_raw_stmt(source, node, diag)),
    };
```

(Verify with a CST dump whether tree-sitter-python 0.21 gives `pattern_list` or `tuple` for `for k, v in …` left sides — reality wins; handle the actual kind(s).)

In `python_for_kind` (~line 262-264, the `enumerate`-unaware `if iter.kind() == "call"` block): before the `range` check, add an `enumerate` check with the same shape:

```rust
        if callee.kind() == "identifier" && node_text(source, callee) == "enumerate" {
            // `for i, x in enumerate(e)` → FOR EACH i, x IN e
            // (index-from-0 semantics intentionally dropped; see spec §2.2)
            let arg = iter
                .named_child(1)
                .and_then(|args| args.named_child(0))
                .ok_or_else(|| parse_err("enumerate missing argument"))?;
            return Ok(ForKind::ForEach {
                var,
                iter: parse_expr(source, arg, diag)?,
            });
        }
```

(Confirm the arguments node shape via the dump — the `range` path above uses `named_child(1)` as the args container; mirror it. `enumerate(e, start)` → take the first arg only.)

Add a one-line comment on `parse_for_stmt` noting a `for … else:` clause is intentionally dropped (pre-existing, out of scope).

- [ ] **Step 4: Run tests**

Run: `cargo test -p algosketch-core` → PASS. `cargo clippy --workspace -- -D warnings`; `cargo fmt --all`.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/python.rs
git commit -m "feat(core): parse Python tuple-for and enumerate loops"
```

### Task 2: Python class method extraction

**Files:**
- Modify: `crates/algosketch-core/src/parser/python.rs` (top-level walk ~lines 50-58; new `collect_class_methods` free fn)
- Test: same file

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn extracts_python_class_methods() {
        let source = "class Solution:\n    def two_sum(self, nums, target):\n        return [1, 2]\n\n    def other(self):\n        pass\n";
        let (module, diag) = PythonParser::new().parse(source).unwrap();
        assert_eq!(module.items.len(), 2);
        let Item::Function(first) = &module.items[0] else { panic!() };
        assert_eq!(first.name, "two_sum");
        assert_eq!(first.params[0].name, "self");
        let Item::Function(second) = &module.items[1] else { panic!() };
        assert_eq!(second.name, "other");
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn class_fields_and_docstrings_skip_silently() {
        let source = "class C:\n    \"\"\"doc\"\"\"\n    x = 1\n\n    def m(self):\n        return self.x\n";
        let (module, diag) = PythonParser::new().parse(source).unwrap();
        assert_eq!(module.items.len(), 1);
        assert_eq!(diag.total(), 0);
    }
```

- [ ] **Step 2: Run to verify they fail** — today `class_definition` → `record_raw_item` (1 Raw item + warning).

- [ ] **Step 3: Implement**

Top-level walk gains an arm before the `_` fallback:

```rust
            if child.kind() == "class_definition" {
                collect_class_methods(source, child, &mut items, &mut diag)?;
            } else if child.kind() == "function_definition" {
                items.push(parse_function(source, child, &mut diag)?);
            } else {
                items.push(record_raw_item(source, child, &mut diag));
            }
```

New free fn (mirrors java.rs's `collect_methods` contract — flat `Item::Function`s, non-method class body silently skipped):

```rust
fn collect_class_methods(
    source: &str,
    node: tree_sitter::Node,
    items: &mut Vec<Item>,
    diag: &mut RawDiagnostics,
) -> Result<()> {
    let body = node
        .child_by_field_name("body")
        .ok_or_else(|| parse_err("class missing body"))?;
    for i in 0..body.named_child_count() {
        let child = body.named_child(i).unwrap();
        if child.kind() == "function_definition" {
            items.push(parse_function(source, child, diag)?);
        }
        // field assignments / docstrings / nested classes: skipped silently
        // (matches Java's class-field handling; see spec §2.3)
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests** — full crate suite green (existing tests unchanged: none cover class Raw for python). clippy + fmt.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/python.rs
git commit -m "feat(core): extract Python class methods as functions"
```

### Task 3: Go if-with-initializer structured emission

**Files:**
- Modify: `crates/algosketch-core/src/parser/go.rs` (`parse_if_stmt` guard ~lines 202-208; `parse_block`/`parse_stmt` plumbing for multi-statement emission)
- Test: same file (REWRITE `if_with_initializer_falls_back_to_raw` ~line 961)

- [ ] **Step 1: Rewrite the test for structured output**

Replace `if_with_initializer_falls_back_to_raw` with:

```rust
    #[test]
    fn if_with_initializer_emits_decl_then_if() {
        let source = "package main\n\nfunc f() int {\n\tif x := g(); x > 0 {\n\t\treturn x\n\t}\n\treturn 0\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        use crate::ir::Stmt;
        assert!(matches!(&f.body.0[0], Stmt::VarDecl(v) if v.name == "x"));
        assert!(matches!(&f.body.0[1], Stmt::If { .. }));
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn if_with_plain_assignment_init_emits_assign_then_if() {
        let source = "package main\n\nfunc f() {\n\tx := 1\n\tif x = g(); x > 0 {\n\t\tg(x)\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        use crate::ir::Stmt;
        assert!(matches!(&f.body.0[1], Stmt::Assign { .. })); // index 1: first is `x := 1`
        assert!(matches!(&f.body.0[2], Stmt::If { .. }));
        assert_eq!(diag.total(), 0);
    }
```

- [ ] **Step 2: Run to verify they fail** — current guard Raws the whole if.

- [ ] **Step 3: Implement**

The init must be emitted as a PRECEDING statement. Mechanically: change `parse_if_stmt` to return `Result<Vec<Stmt>>` (precedent: `parse_var_declaration` already returns `Vec<Stmt>` and `parse_block` appends — check how `parse_block` dispatches to it, ~line 153) and thread through `parse_stmt`? NO — cleaner: handle `if_statement` in `parse_block`'s dispatch where Vec-statement helpers already flow, OR make `parse_stmt` return the if WITH its init folded… IR has no multi-stmt node. Follow the existing Vec precedent: in `parse_block`'s statement loop, route `if_statement` to a `parse_if_stmt_vec(source, node, diag) -> Result<Vec<Stmt>>` that (a) parses the `initializer` field via the same short-var/assignment logic (`parse_stmt` on the init node works — it's a `short_var_declaration`/`assignment_statement` node), pushes it, then (b) parses the if itself (condition/consequence/alternative as today) and pushes it. `parse_stmt` keeps `if_statement` routed through the same helper by wrapping (or leave parse_stmt's if arm calling the vec fn and… pick the minimal plumbing; note the choice in your report).

Rewrite the guard comment at ~202: structured now; multi-name init (`if x, y := f(); c`) follows the multi-name assign rule via `parse_stmt` on the init node.

- [ ] **Step 4: Run tests** — full suite green. clippy + fmt.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/go.rs
git commit -m "feat(core): structure Go if-with-initializer"
```

### Task 4: Go two-var range

**Files:**
- Modify: `crates/algosketch-core/src/parser/go.rs` (range_clause arm ~lines 270-290)
- Test: same file (REWRITE `two_var_range_falls_back_to_raw` ~line 896)

- [ ] **Step 1: Rewrite the test**

```rust
    #[test]
    fn two_var_range_parses_as_foreach() {
        let source = "package main\n\nfunc f(xs []int) {\n\tfor i, v := range xs {\n\t\tg(i)\n\t\tg(v)\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        let Stmt::For { kind: ForKind::ForEach { var, iter }, .. } = &f.body.0[0] else {
            panic!("expected foreach");
        };
        assert_eq!(var, "i, v");
        assert_eq!(iter, &Expr::Ident("xs".into()));
        assert_eq!(diag.total(), 0);
    }
```

- [ ] **Step 2: Verify red** (current arm Raws two-name ranges).

- [ ] **Step 3: Implement** — in the `range_clause` arm, replace the two-name `record_raw_stmt` branch with: collect the left side's identifier names (the `left` field is an `expression_list`); if 1 name → today's path; if ≥2 flat identifiers → `ForEach { var: names.join(", "), iter: right }` (mirrors Python Task 1; non-identifier left elements → keep the Raw fallback).

- [ ] **Step 4: Tests green** + clippy + fmt.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/go.rs
git commit -m "feat(core): parse Go two-variable range loops"
```

### Task 5: Go switch → if/else-if chains

**Files:**
- Modify: `crates/algosketch-core/src/parser/go.rs` (new `parse_switch_stmt`; `parse_block`/`parse_stmt` routing; switch currently falls to the default Raw arm)
- Test: same file (REWRITE `switch_statement_falls_back_to_raw` ~line 981; add Raw cases)

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn tagless_switch_parses_as_if_chain() {
        let source = "package main\n\nfunc f(a int, b int) int {\n\tswitch {\n\tcase a > b:\n\t\treturn a\n\tcase b > a:\n\t\treturn b\n\tdefault:\n\t\treturn 0\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        let Stmt::If { cond, then_block, else_block: Some(els) } = &f.body.0[0] else {
            panic!("expected if chain");
        };
        assert!(matches!(cond, crate::ir::Expr::Binary { .. }));
        assert_eq!(then_block.0.len(), 1);
        assert!(matches!(els.0[0], Stmt::If { .. })); // else-if nesting
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn tagged_switch_parses_as_equality_chain() {
        let source = "package main\n\nfunc f(x int) string {\n\tswitch x {\n\tcase 1:\n\t\treturn \"one\"\n\tcase 2:\n\t\treturn \"two\"\n\t}\n\treturn \"\"\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        let Stmt::If { cond, else_block: Some(els), .. } = &f.body.0[0] else {
            panic!("expected if chain");
        };
        assert!(matches!(cond, crate::ir::Expr::Binary { op: crate::ir::BinOp::Eq, .. }));
        assert!(matches!(els.0[0], Stmt::If { .. }));
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn multi_expr_case_parses_as_or_chain() {
        let source = "package main\n\nfunc f(x int) int {\n\tswitch x {\n\tcase 1, 2, 3:\n\t\treturn 1\n\tdefault:\n\t\treturn 0\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else { panic!() };
        let Stmt::If { cond, .. } = &f.body.0[0] else { panic!() };
        // (x = 1 OR x = 2) OR x = 3 — any nested Binary{Or} shape accepted
        assert!(matches!(cond, crate::ir::Expr::Binary { op: crate::ir::BinOp::Or, .. }));
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn fallthrough_switch_stays_raw() {
        let source = "package main\n\nfunc f(x int) int {\n\tswitch x {\n\tcase 1:\n\t\tfallthrough\n\tcase 2:\n\t\treturn 2\n\t}\n\treturn 0\n}\n";
        let (_, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.statements, 1);
    }

    #[test]
    fn type_switch_stays_raw() {
        let source = "package main\n\nfunc f(v interface{}) int {\n\tswitch t := v.(type) {\n\tcase int:\n\t\treturn t\n\t}\n\treturn 0\n}\n";
        let (_, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.statements, 1);
    }
```

- [ ] **Step 2: Verify red** for the three structured tests (switch Raws today); the two Raw tests pass now and must STILL pass after.

- [ ] **Step 3: Implement**

`parse_switch_stmt(source, node, diag) -> Result<Stmt>` (grammar facts verified against tree-sitter-go 0.25 node-types.json: `expression_switch_statement` has optional `initializer` field (an init clause — if present → Raw, whole switch) and optional **`value`** field (the tag expression); case children are `expression_case` nodes with a required **`value`** field (an `expression_list`) and an optional `statement_list` child holding the body (`parse_block`'s find-statement_list lookup works on it directly); `default_case` has just the body; `fallthrough_statement` is a statement kind; type switches are `type_switch_statement` — stays Raw by simply not matching):

- If any case body contains a `fallthrough` statement → `record_raw_stmt` the whole switch.
- Build an else-if chain right-to-left: default block (if present) is the innermost else; each `expression_case` contributes `If { cond, then: body, else: accumulated }`.
- Tagged: cond per case = OR-chain of `Binary { Eq, tag, expr }` over the case's `value` expression_list; multi-expr → fold with `BinOp::Or` left-associative.
- Tagless: cond = OR-chain when multiple exprs, else the single expression (case exprs are full boolean expressions).
- Route `expression_switch_statement` in the statement dispatch; `type_switch_statement`, `select_statement`, and initializer-bearing switches fall to Raw.

- [ ] **Step 4: Full suite green** + clippy + fmt.

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/parser/go.rs
git commit -m "feat(core): parse Go switch as if chains"
```

---

## Chunk 2: CLI pinning, distribution, release (Tasks 6–9)

### Task 6: CLI pinning tests

**Files:**
- Test: `crates/algosketch-cli/tests/cli.rs` (append; reuse `write_temp_python_file`; Go source inline via `write_stdin`)

- [ ] **Step 1: Write the tests** (pinning — parser work from Tasks 1–5 must already make them pass; if not, fix upstream)

```rust
#[test]
fn python_class_file_renders_methods() {
    let fixture = write_temp_python_file(
        "class-methods",
        r#"class Solution:
    def two_sum(self, nums, target):
        for i, x in enumerate(nums):
            g(i)
        return []
"#,
    );
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg(fixture.path()).arg("--pseudo-only").arg("--lang").arg("en");

    cmd.assert()
        .success()
        .stdout(contains("FUNCTION two_sum"))
        .stdout(contains("FOR EACH i, x IN nums"))
        .stderr(contains("warning:").not());
}

#[test]
fn go_err_idiom_renders_structured() {
    let go_source = "package main\n\nfunc f() error {\n\tif err := g(); err != nil {\n\t\treturn err\n\t}\n\treturn nil\n}\n";
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("-")
        .arg("--source-lang")
        .arg("go")
        .arg("--pseudo-only")
        .arg("--lang")
        .arg("en")
        .write_stdin(go_source);

    cmd.assert()
        .success()
        .stdout(contains("FUNCTION f"))
        .stdout(contains("IF err ≠ NIL THEN"))
        .stderr(contains("warning:").not());
}
```

(Verify the exact rendered condition text — `err != nil` → `err ≠ NIL`? Check pseudo.rs's Ne/None rendering first (`Literal::None` renders as `NIL` per the Go Task 3 probe) and adjust the assertion to the real string. Asserting the IF exists with `≠` is the point; the decl presence is implied by FUNCTION + no warning.)

- [ ] **Step 2: Run** — both pass immediately (pinning). `cargo test --workspace` (report total); clippy; fmt.

- [ ] **Step 3: Commit**

```bash
git add crates/algosketch-cli/tests/cli.rs
git commit -m "test(cli): pin class and err-idiom coverage"
```

### Task 7: Package rename + crates.io metadata + README install

**Files:**
- Modify: `crates/algosketch-cli/Cargo.toml`, `crates/algosketch-core/Cargo.toml`, `README.md`
- NOT touched: directory names, workspace members paths, `[[bin]]`

- [ ] **Step 1: Rename + metadata**

`crates/algosketch-cli/Cargo.toml` `[package]`: `name = "algosketch-cli"` → `name = "algosketch"`. Add (workspace-inherit style where the file already uses it):

```toml
description = "Turn real source code into language-neutral pseudocode and human-readable explanations"
keywords = ["pseudocode", "cli", "tree-sitter", "algorithm", "code-review"]
categories = ["command-line-utilities", "development-tools"]
readme = "../../README.md"
```

`crates/algosketch-core/Cargo.toml` `[package]`: add matching `description` (the CURRENT description field there is stale at three languages — "Python / Java / C++"; refresh it to name all FOUR), `keywords` (["pseudocode", "tree-sitter", "parser", "ir", "rendering"]), `categories` (["development-tools"]), `readme = "../../README.md"`.

`homepage`/`repository`: workspace already carries `repository`; add nothing if inheritable — check whether `homepage` is worth adding (skip unless trivial: `homepage` is optional; leave it out, YAGNI).

- [ ] **Step 2: Verify**

Run: `cargo build --workspace` (rename ripples: lockfile package name change — run `cargo check` to refresh). `cargo test --workspace` green (`cargo_bin("algosketch")` unaffected — bin name unchanged). `cargo package -p algosketch --list --allow-dirty 2>&1 | head -20` sanity (readme included). Then the dry-run gate: `cargo publish --dry-run -p algosketch-core --allow-dirty`? NO — dry-run needs no allow-dirty on a clean tree; run `cargo publish --dry-run -p algosketch-core` (expect success; network check included). Do NOT dry-run the cli crate (its packaged core dep isn't on the registry yet — spec §6).

- [ ] **Step 3: README install section**

In README EN, after the intro/status and before usage (mirror placement in 中文 section):

```markdown
## Install / 安装

```bash
cargo install algosketch
```

Or build from source: `cargo install --git https://github.com/Cass-ette/algosketch --path crates/algosketch-cli`.
```

(One shared bilingual section like the status line; keep it minimal. 中文 section gets the same block or the shared one suffices — follow the README's existing bilingual structure.)

- [ ] **Step 4: Commit** — `chore: rename cli package and add crates.io metadata` (staging: two Cargo.tomls + Cargo.lock + README.md)

### Task 8: Version 0.2.1 + doc sync

**Files:**
- Modify: `Cargo.toml` (workspace version), `crates/algosketch-cli/Cargo.toml` (path-dep `version = "0.2.0"` → `"0.2.1"`), `Cargo.lock`, `docs/superpowers/specs/2026-05-20-algosketch-design.md`, `README.md`

- [ ] **Step 1: Version** — workspace `0.2.0` → `0.2.1`; cli path-dep string → `0.2.1`; `cargo check --workspace` refresh lock.

- [ ] **Step 2: Main spec** — §10 add row `| M7 | Coverage + distribution | Python classes/enumerate/tuple-for; Go if-init/switch/two-var range; crates.io-ready; v0.2.1. |`; completion line → "All milestones M1–M7 complete; v0.2.1 tagged 2026-09."

- [ ] **Step 3: README** — status line → v0.2.1 wording; neutralize the four version-stale sentences (lines ~20/42 EN, ~124/146 中文): "v0.1 is intentionally not…" → "algosketch is intentionally not…", "v0.1 is fully rule-based" → "algosketch is fully rule-based", 中文同理.

- [ ] **Step 4: Gates** — `cargo test --workspace` green; `cargo publish --dry-run -p algosketch-core` still clean (version bumped); fmt; clippy.

- [ ] **Step 5: Commit** — `chore: bump version to 0.2.1 and sync docs`

### Task 9: Final gate + release (controller-executed)

- [ ] Full gate: fmt --check / clippy -D warnings / test --workspace / build --release; `git status` clean.
- [ ] Smoke: python class file → methods rendered; go switch file → if-chain rendered; `--version` → 0.2.1.
- [ ] Push + PR + CI + merge + tag `v0.2.1` + cleanup + binary reinstall.
- [ ] Final report to the owner: the two publish commands (`cargo publish -p algosketch-core`, then `cargo publish -p algosketch`) — requires their `cargo login` once.
