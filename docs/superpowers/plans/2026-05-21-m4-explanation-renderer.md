# M4 Explanation Renderer Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement natural-language explanation renderer with Chinese/English support, locale detection, and CLI integration

**Architecture:** ExplainRenderer walks IR and generates explanations using template-based approach. Pattern recognition for function-level summaries, statement-by-statement templates for steps. Locale auto-detection from environment variables.

**Tech Stack:** Rust, clap (CLI), tree-sitter (already integrated), assert_cmd (CLI testing)

**Spec:** `docs/superpowers/specs/2026-05-21-m4-explanation-renderer.md`

---

## File Structure

**New files:**
- `crates/algosketch-core/src/renderer/mod.rs` — module exports (split from renderer.rs)
- `crates/algosketch-core/src/renderer/pseudo.rs` — PseudoRenderer (moved from renderer.rs)
- `crates/algosketch-core/src/renderer/explain.rs` — ExplainRenderer (new, ~200 lines)
- `crates/algosketch-cli/fixtures/binary_search.py` — test fixture

**Modified files:**
- `crates/algosketch-core/src/lib.rs` — add NaturalLang enum
- `crates/algosketch-core/src/renderer.rs` — DELETE (split into renderer/mod.rs + renderer/pseudo.rs)
- `crates/algosketch-cli/src/main.rs` — CLI flags, locale detection, output assembly
- `crates/algosketch-cli/tests/cli.rs` — add explanation tests
- `README.md` — add output examples section

---

## Chunk 1: Core Types and Renderer Module Structure

### Task 1: Add NaturalLang enum to lib.rs

**Files:**
- Modify: `crates/algosketch-core/src/lib.rs`

- [ ] **Step 1: Write failing test for NaturalLang enum**

```rust
// Add to lib.rs after SourceLang tests (if any)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_lang_enum_exists() {
        let zh = NaturalLang::Zh;
        let en = NaturalLang::En;
        assert_ne!(zh, en);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package algosketch-core natural_lang_enum_exists`
Expected: FAIL with "NaturalLang not found"

- [ ] **Step 3: Add NaturalLang enum**

```rust
// Add after SourceLang definition in lib.rs
/// Natural language for explanation output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NaturalLang {
    Zh,
    En,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package algosketch-core natural_lang_enum_exists`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/lib.rs
git commit -m "feat(core): add NaturalLang enum for explanation language"
```

---

### Task 2: Split renderer.rs into module structure

**Files:**
- Create: `crates/algosketch-core/src/renderer/mod.rs`
- Create: `crates/algosketch-core/src/renderer/pseudo.rs`
- Delete: `crates/algosketch-core/src/renderer.rs`

- [ ] **Step 1: Create renderer/mod.rs with exports**

```rust
//! Renderers for pseudocode and natural-language explanations.

pub mod pseudo;

pub use pseudo::PseudoRenderer;
```

- [ ] **Step 2: Move renderer.rs content to renderer/pseudo.rs**

Run: `mkdir -p crates/algosketch-core/src/renderer && mv crates/algosketch-core/src/renderer.rs crates/algosketch-core/src/renderer/pseudo.rs`

- [ ] **Step 3: Update pseudo.rs module comment**

Change first line from `//! Pseudocode renderer.` to:
```rust
//! Pseudocode renderer (CLRS-style).
```

- [ ] **Step 4: Verify tests still pass**

Run: `cargo test --package algosketch-core`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/renderer/
git rm crates/algosketch-core/src/renderer.rs
git commit -m "refactor(core): split renderer into module structure"
```

---

## Chunk 2: ExplainRenderer Skeleton and Pattern Recognition

### Task 3: Create ExplainRenderer skeleton with render_module

**Files:**
- Create: `crates/algosketch-core/src/renderer/explain.rs`
- Modify: `crates/algosketch-core/src/renderer/mod.rs`

- [ ] **Step 1: Write failing test for ExplainRenderer**

Create `crates/algosketch-core/src/renderer/explain.rs`:
```rust
//! Natural-language explanation renderer.

use crate::ir::*;
use crate::NaturalLang;

#[derive(Debug)]
pub struct ExplainRenderer {
    pub lang: NaturalLang,
}

impl ExplainRenderer {
    pub fn new(lang: NaturalLang) -> Self {
        Self { lang }
    }
    
    pub fn render_module(&self, _module: &Module) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{LanguageParser, PythonParser};

    #[test]
    fn renders_empty_module() {
        let source = "";
        let module = PythonParser::new().parse(source).unwrap();
        let out = ExplainRenderer::new(NaturalLang::Zh).render_module(&module);
        assert_eq!(out, "");
    }
}
```

- [ ] **Step 2: Export ExplainRenderer from mod.rs**

Add to `crates/algosketch-core/src/renderer/mod.rs`:
```rust
pub mod explain;

pub use explain::ExplainRenderer;
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test --package algosketch-core renders_empty_module`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/algosketch-core/src/renderer/
git commit -m "feat(core): add ExplainRenderer skeleton"
```

---

### Task 4: Implement pattern recognition helpers

**Files:**
- Modify: `crates/algosketch-core/src/renderer/explain.rs`

- [ ] **Step 1: Write test for has_loop detection**

Add to tests in `explain.rs`:
```rust
#[test]
fn detects_loop_in_function() {
    let source = r#"
def foo():
    while True:
        pass
"#;
    let module = PythonParser::new().parse(source).unwrap();
    let renderer = ExplainRenderer::new(NaturalLang::Zh);
    if let Some(Item::Function(f)) = module.items.first() {
        assert!(renderer.has_loop(&f.body));
    } else {
        panic!("Expected function");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package algosketch-core detects_loop_in_function`
Expected: FAIL with "has_loop not found"

- [ ] **Step 3: Implement has_loop helper**

Add to `ExplainRenderer` impl:
```rust
fn has_loop(&self, block: &Block) -> bool {
    block.0.iter().any(|stmt| matches!(stmt, Stmt::While { .. } | Stmt::For { .. }))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package algosketch-core detects_loop_in_function`
Expected: PASS

- [ ] **Step 5: Write test for has_recursion detection**

Add to tests:
```rust
#[test]
fn detects_recursion_in_function() {
    let source = r#"
def factorial(n):
    return factorial(n - 1)
"#;
    let module = PythonParser::new().parse(source).unwrap();
    let renderer = ExplainRenderer::new(NaturalLang::Zh);
    if let Some(Item::Function(f)) = module.items.first() {
        assert!(renderer.has_recursion(f));
    } else {
        panic!("Expected function");
    }
}
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cargo test --package algosketch-core detects_recursion_in_function`
Expected: FAIL with "has_recursion not found"

- [ ] **Step 7: Implement has_recursion helper**

Add to `ExplainRenderer` impl:
```rust
fn has_recursion(&self, f: &Function) -> bool {
    self.block_calls_function(&f.body, &f.name)
}

fn block_calls_function(&self, block: &Block, fname: &str) -> bool {
    block.0.iter().any(|stmt| self.stmt_calls_function(stmt, fname))
}

fn stmt_calls_function(&self, stmt: &Stmt, fname: &str) -> bool {
    match stmt {
        Stmt::Return(Some(e)) | Stmt::ExprStmt(e) => self.expr_calls_function(e, fname),
        Stmt::Assign { value, .. } => self.expr_calls_function(value, fname),
        Stmt::If { then_block, else_block, .. } => {
            self.block_calls_function(then_block, fname)
                || else_block.as_ref().map_or(false, |b| self.block_calls_function(b, fname))
        }
        Stmt::While { body, .. } | Stmt::For { body, .. } => {
            self.block_calls_function(body, fname)
        }
        _ => false,
    }
}

fn expr_calls_function(&self, expr: &Expr, fname: &str) -> bool {
    match expr {
        Expr::Call { callee, .. } => {
            matches!(callee.as_ref(), Expr::Ident(name) if name == fname)
        }
        Expr::Binary { lhs, rhs, .. } => {
            self.expr_calls_function(lhs, fname) || self.expr_calls_function(rhs, fname)
        }
        _ => false,
    }
}
```

- [ ] **Step 8: Run test to verify it passes**

Run: `cargo test --package algosketch-core detects_recursion_in_function`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add crates/algosketch-core/src/renderer/explain.rs
git commit -m "feat(core): add pattern recognition helpers (has_loop, has_recursion)"
```

---

### Task 5: Implement detect_purpose with pattern matching

**Files:**
- Modify: `crates/algosketch-core/src/renderer/explain.rs`

- [ ] **Step 1: Write test for detect_purpose with search pattern**

Add to tests:
```rust
#[test]
fn detects_search_purpose_zh() {
    let source = r#"
def binary_search(nums, target):
    while True:
        pass
"#;
    let module = PythonParser::new().parse(source).unwrap();
    let renderer = ExplainRenderer::new(NaturalLang::Zh);
    if let Some(Item::Function(f)) = module.items.first() {
        let purpose = renderer.detect_purpose(f);
        assert!(purpose.contains("查找"));
        assert!(purpose.contains("迭代"));
    } else {
        panic!("Expected function");
    }
}

#[test]
fn detects_search_purpose_en() {
    let source = r#"
def binary_search(nums, target):
    while True:
        pass
"#;
    let module = PythonParser::new().parse(source).unwrap();
    let renderer = ExplainRenderer::new(NaturalLang::En);
    if let Some(Item::Function(f)) = module.items.first() {
        let purpose = renderer.detect_purpose(f);
        assert!(purpose.contains("search"));
        assert!(purpose.contains("iteratively"));
    } else {
        panic!("Expected function");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package algosketch-core detects_search_purpose`
Expected: FAIL with "detect_purpose not found"

- [ ] **Step 3: Implement detect_purpose**

Add to `ExplainRenderer` impl:
```rust
fn detect_purpose(&self, f: &Function) -> String {
    let name_lower = f.name.to_lowercase();
    let has_loop = self.has_loop(&f.body);
    let has_recursion = self.has_recursion(f);
    
    let action = if name_lower.contains("search") || name_lower.contains("find") {
        match self.lang {
            NaturalLang::Zh => "查找",
            NaturalLang::En => "search for",
        }
    } else if name_lower.contains("sort") {
        match self.lang {
            NaturalLang::Zh => "排序",
            NaturalLang::En => "sort",
        }
    } else if name_lower.contains("reverse") {
        match self.lang {
            NaturalLang::Zh => "反转",
            NaturalLang::En => "reverse",
        }
    } else {
        match self.lang {
            NaturalLang::Zh => "处理",
            NaturalLang::En => "process",
        }
    };
    
    let method = if has_recursion {
        match self.lang {
            NaturalLang::Zh => "（递归）",
            NaturalLang::En => " (recursively)",
        }
    } else if has_loop {
        match self.lang {
            NaturalLang::Zh => "（迭代）",
            NaturalLang::En => " (iteratively)",
        }
    } else {
        ""
    };
    
    match self.lang {
        NaturalLang::Zh => format!("{}输入数据{}", action, method),
        NaturalLang::En => format!("{} the input{}", action, method),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --package algosketch-core detects_search_purpose`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/renderer/explain.rs
git commit -m "feat(core): implement detect_purpose with pattern matching"
```

---

## Chunk 3: Statement-by-Statement Explanation

### Task 6: Implement expr_to_text helper

**Files:**
- Modify: `crates/algosketch-core/src/renderer/explain.rs`

- [ ] **Step 1: Copy render_expr logic from pseudo.rs**

Add to `explain.rs` (outside impl block):
```rust
// Helper: convert Expr to pseudocode text (reuses PseudoRenderer logic)
fn expr_to_text(expr: &Expr) -> String {
    match expr {
        Expr::Literal(lit) => render_literal(lit),
        Expr::Ident(name) => name.clone(),
        Expr::Binary { op, lhs, rhs } => {
            format!(
                "{} {} {}",
                render_binary_operand(lhs),
                render_binop(*op),
                render_binary_operand(rhs)
            )
        }
        Expr::Unary { op, expr } => {
            format!("{}{}", render_unop(*op), expr_to_text(expr))
        }
        Expr::Call { callee, args } => render_call(callee, args),
        Expr::Index { obj, index } => format!("{}[{}]", expr_to_text(obj), expr_to_text(index)),
        Expr::Field { obj, name } => format!("{}.{}", expr_to_text(obj), name),
        Expr::Tuple(items) => items.iter().map(expr_to_text).collect::<Vec<_>>().join(", "),
        Expr::Raw(text) => text.clone(),
    }
}

fn render_binary_operand(expr: &Expr) -> String {
    match expr {
        Expr::Binary { .. } => format!("({})", expr_to_text(expr)),
        _ => expr_to_text(expr),
    }
}

fn render_call(callee: &Expr, args: &[Expr]) -> String {
    if let Expr::Ident(name) = callee {
        if name == "len" && args.len() == 1 {
            return format!("LENGTH({})", expr_to_text(&args[0]));
        }
    }
    if let Expr::Field { obj, name } = callee {
        if (name == "size" || name == "length") && args.is_empty() {
            return format!("LENGTH({})", expr_to_text(obj));
        }
    }
    let args_str = args.iter().map(expr_to_text).collect::<Vec<_>>().join(", ");
    format!("{}({})", expr_to_text(callee), args_str)
}

fn render_literal(lit: &Literal) -> String {
    match lit {
        Literal::Int(n) => n.to_string(),
        Literal::Float(s) => s.clone(),
        Literal::Str(s) => format!("\"{s}\""),
        Literal::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Literal::None => "NIL".to_string(),
    }
}

fn render_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::IntDiv => "DIV",
        BinOp::Mod => "MOD",
        BinOp::Eq => "=",
        BinOp::Ne => "≠",
        BinOp::Lt => "<",
        BinOp::Le => "≤",
        BinOp::Gt => ">",
        BinOp::Ge => "≥",
        BinOp::And => "AND",
        BinOp::Or => "OR",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}

fn render_unop(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "NOT ",
        UnOp::BitNot => "~",
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --package algosketch-core`
Expected: SUCCESS

- [ ] **Step 3: Commit**

```bash
git add crates/algosketch-core/src/renderer/explain.rs
git commit -m "feat(core): add expr_to_text helper for pseudocode notation"
```

---

### Task 7: Implement render_stmt with statement templates

**Files:**
- Modify: `crates/algosketch-core/src/renderer/explain.rs`

- [ ] **Step 1: Write test for render_stmt with Assign**

Add to tests:
```rust
#[test]
fn renders_assign_stmt_zh() {
    let source = r#"
def foo():
    x = 5
"#;
    let module = PythonParser::new().parse(source).unwrap();
    let renderer = ExplainRenderer::new(NaturalLang::Zh);
    if let Some(Item::Function(f)) = module.items.first() {
        let mut out = String::new();
        renderer.render_stmt(&f.body.0[0], 0, &mut out);
        assert!(out.contains("将 x 赋值为 5"));
    } else {
        panic!("Expected function");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package algosketch-core renders_assign_stmt_zh`
Expected: FAIL with "render_stmt not found"

- [ ] **Step 3: Implement render_stmt for basic statements**

Add to `ExplainRenderer` impl:
```rust
fn render_stmt(&self, stmt: &Stmt, depth: usize, out: &mut String) {
    match stmt {
        Stmt::Assign { target, value } => {
            let line = match self.lang {
                NaturalLang::Zh => format!("将 {} 赋值为 {}", expr_to_text(target), expr_to_text(value)),
                NaturalLang::En => format!("Assign {} to {}", expr_to_text(target), expr_to_text(value)),
            };
            out.push_str(&format!("{}\n", line));
        }
        
        Stmt::Return(expr) => {
            let val = expr.as_ref().map(expr_to_text).unwrap_or_default();
            let line = match self.lang {
                NaturalLang::Zh => format!("返回 {}", val),
                NaturalLang::En => format!("Return {}", val),
            };
            out.push_str(&format!("{}\n", line));
        }
        
        Stmt::Break => {
            let tmpl = match self.lang {
                NaturalLang::Zh => "跳出循环",
                NaturalLang::En => "Break out of loop",
            };
            out.push_str(&format!("{}\n", tmpl));
        }
        
        Stmt::Continue => {
            let tmpl = match self.lang {
                NaturalLang::Zh => "继续下一次迭代",
                NaturalLang::En => "Continue to next iteration",
            };
            out.push_str(&format!("{}\n", tmpl));
        }
        
        Stmt::ExprStmt(e) => {
            out.push_str(&format!("{}\n", expr_to_text(e)));
        }
        
        Stmt::Raw(_) => {
            let tmpl = match self.lang {
                NaturalLang::Zh => "> 此处源代码未能结构化解析，已原样保留。",
                NaturalLang::En => "> Unparsed source preserved as-is.",
            };
            out.push_str(&format!("{}\n", tmpl));
        }

        Stmt::VarDecl(var) => {
            let line = match &var.init {
                Some(init) => match self.lang {
                    NaturalLang::Zh => format!("声明 {} 并赋值为 {}", var.name, expr_to_text(init)),
                    NaturalLang::En => format!("Declare {} and set it to {}", var.name, expr_to_text(init)),
                },
                None => match self.lang {
                    NaturalLang::Zh => format!("声明 {}", var.name),
                    NaturalLang::En => format!("Declare {}", var.name),
                },
            };
            out.push_str(&format!("{}\n", line));
        }
        
        _ => {
            // If/While/For handled separately
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package algosketch-core renders_assign_stmt_zh`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/renderer/explain.rs
git commit -m "feat(core): implement render_stmt for basic statements"
```

---

### Task 8: Implement render_stmt for control flow (If/While)

**Files:**
- Modify: `crates/algosketch-core/src/renderer/explain.rs`

- [ ] **Step 1: Write test for nested while/if output**

Add to tests:
```rust
#[test]
fn renders_while_and_if_steps_zh() {
    let source = r#"
def foo(x):
    while x > 0:
        if x == 1:
            return x
        x = x - 1
"#;
    let module = PythonParser::new().parse(source).unwrap();
    let out = ExplainRenderer::new(NaturalLang::Zh).render_module(&module);
    assert!(out.contains("当 x > 0 时重复以下步骤"));
    assert!(out.contains("如果 x = 1，则"));
    assert!(out.contains("返回 x"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package algosketch-core renders_while_and_if_steps_zh`
Expected: FAIL because render_module does not yet render function steps

- [ ] **Step 3: Extend render_stmt with control flow statements**

Extend the existing `render_stmt` match with these additional arms:
```rust
Stmt::If { cond, then_block, else_block } => {
    let prefix = match self.lang {
        NaturalLang::Zh => format!("如果 {}，则：", expr_to_text(cond)),
        NaturalLang::En => format!("If {}, then:", expr_to_text(cond)),
    };
    out.push_str(&format!("{}\n", prefix));
    out.push_str(&self.render_steps(then_block, depth + 1));
    if let Some(else_block) = else_block {
        self.render_else(else_block, depth, out);
    }
}
Stmt::While { cond, body } => {
    let line = match self.lang {
        NaturalLang::Zh => format!("当 {} 时重复以下步骤：", expr_to_text(cond)),
        NaturalLang::En => format!("While {}, repeat:", expr_to_text(cond)),
    };
    out.push_str(&format!("{}\n", line));
    out.push_str(&self.render_steps(body, depth + 1));
}
Stmt::For { kind, body } => {
    let line = self.render_for_header(kind);
    out.push_str(&format!("{}\n", line));
    out.push_str(&self.render_steps(body, depth + 1));
}
```

Add ForKind helper:
```rust
fn render_for_header(&self, kind: &ForKind) -> String {
    match kind {
        ForKind::ForEach { var, iter } => match self.lang {
            NaturalLang::Zh => format!("对 {} 中的每个 {}，重复以下步骤：", expr_to_text(iter), var),
            NaturalLang::En => format!("For each {} in {}, repeat:", var, expr_to_text(iter)),
        },
        ForKind::Range { var, start, end, step } => {
            let step_text = step.as_ref().map(|s| format!(", step {}", expr_to_text(s))).unwrap_or_default();
            match self.lang {
                NaturalLang::Zh => format!("令 {} 从 {} 到 {}{}，重复以下步骤：", var, expr_to_text(start), expr_to_text(end), step_text),
                NaturalLang::En => format!("For {} from {} to {}{}, repeat:", var, expr_to_text(start), expr_to_text(end), step_text),
            }
        }
        ForKind::CStyle { cond, step, .. } => match self.lang {
            NaturalLang::Zh => format!("按条件 {} 和步进 {} 循环执行：", expr_to_text(cond), expr_to_text(step)),
            NaturalLang::En => format!("Loop with condition {} and step {}, repeat:", expr_to_text(cond), expr_to_text(step)),
        },
    }
}
```

Also add the `render_steps` helper method:
```rust
fn render_steps(&self, block: &Block, depth: usize) -> String {
    let mut out = String::new();
    for (i, stmt) in block.0.iter().enumerate() {
        let indent = "  ".repeat(depth);
        out.push_str(&format!("{}{}. ", indent, i + 1));
        self.render_stmt(stmt, depth, &mut out);
    }
    out
}
```

Add helper:
```rust
fn render_else(&self, else_block: &Block, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    if let [Stmt::If { cond, then_block, else_block: nested_else }] = else_block.0.as_slice() {
        let line = match self.lang {
            NaturalLang::Zh => format!("否则如果 {}，则：", expr_to_text(cond)),
            NaturalLang::En => format!("Otherwise if {}, then:", expr_to_text(cond)),
        };
        out.push_str(&format!("{}{}\n", indent, line));
        out.push_str(&self.render_steps(then_block, depth + 1));
        if let Some(nested) = nested_else {
            self.render_else(nested, depth, out);
        }
    } else {
        let line = match self.lang { NaturalLang::Zh => "否则：", NaturalLang::En => "Otherwise:" };
        out.push_str(&format!("{}{}\n", indent, line));
        out.push_str(&self.render_steps(else_block, depth + 1));
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package algosketch-core renders_while_and_if_steps_zh`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/renderer/explain.rs
git commit -m "feat(core): render control-flow explanation steps"
```

---

### Task 9: Implement render_function and render_module output

**Files:**
- Modify: `crates/algosketch-core/src/renderer/explain.rs`

- [ ] **Step 1: Write full binary_search explanation tests**

Add to tests:
```rust
#[test]
fn renders_binary_search_explanation_zh() {
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
    let out = ExplainRenderer::new(NaturalLang::Zh).render_module(&module);
    assert!(out.contains("函数 binary_search(nums, target)"));
    assert!(out.contains("目的：查找输入数据（迭代）"));
    assert!(out.contains("步骤："));
    assert!(out.contains("当 left ≤ right 时重复以下步骤"));
    assert!(out.contains("否则如果 nums[mid] < target，则"));
}

#[test]
fn renders_binary_search_explanation_en() {
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
    let out = ExplainRenderer::new(NaturalLang::En).render_module(&module);
    assert!(out.contains("Function binary_search(nums, target)"));
    assert!(out.contains("Purpose: search for the input (iteratively)"));
    assert!(out.contains("Steps:"));
    assert!(out.contains("While left ≤ right, repeat"));
    assert!(out.contains("Otherwise if nums[mid] < target, then"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package algosketch-core renders_binary_search_explanation`
Expected: FAIL because render_module/render_function incomplete

- [ ] **Step 3: Implement render_function and complete render_module**

Replace render_module:
```rust
pub fn render_module(&self, module: &Module) -> String {
    let mut out = String::new();
    for item in &module.items {
        if let Item::Function(f) = item {
            out.push_str(&self.render_function(f));
            out.push('\n');
        }
    }
    out
}
```

Add render_function:
```rust
pub fn render_function(&self, f: &Function) -> String {
    let params = f.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ");
    let purpose = self.detect_purpose(f);
    let steps = self.render_steps(&f.body, 1);
    match self.lang {
        NaturalLang::Zh => format!(
            "函数 {}({})\n\n目的：{}\n\n步骤：\n{}",
            f.name, params, purpose, steps
        ),
        NaturalLang::En => format!(
            "Function {}({})\n\nPurpose: {}\n\nSteps:\n{}",
            f.name, params, purpose, steps
        ),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --package algosketch-core renders_binary_search_explanation`
Expected: PASS

- [ ] **Step 5: Run all core tests**

Run: `cargo test --package algosketch-core`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/algosketch-core/src/renderer/explain.rs
git commit -m "feat(core): render full function explanations"
```

---

## Chunk 4: CLI Flags and Locale Detection

### Task 10: Add CLI language enum and locale detection tests

**Files:**
- Modify: `crates/algosketch-cli/src/main.rs`

- [ ] **Step 1: Add NaturalLangArg enum and resolver tests**

Add to `main.rs`:
```rust
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum NaturalLangArg {
    Zh,
    En,
    Auto,
}

fn resolve_natural_lang(arg: NaturalLangArg) -> NaturalLang {
    match arg {
        NaturalLangArg::Zh => NaturalLang::Zh,
        NaturalLangArg::En => NaturalLang::En,
        NaturalLangArg::Auto => detect_locale(),
    }
}
```

Add tests at bottom of `main.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_explicit_natural_lang() {
        assert_eq!(resolve_natural_lang(NaturalLangArg::Zh), NaturalLang::Zh);
        assert_eq!(resolve_natural_lang(NaturalLangArg::En), NaturalLang::En);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package algosketch-cli resolves_explicit_natural_lang`
Expected: FAIL because NaturalLang not imported

- [ ] **Step 3: Import NaturalLang**

Update import in `main.rs`:
```rust
use algosketch_core::{NaturalLang, PseudoError, SourceLang};
```

- [ ] **Step 4: Add detect_locale implementation**

```rust
fn detect_locale() -> NaturalLang {
    if let Ok(val) = std::env::var("PSEUDOCODE_LANG") {
        if val.starts_with("zh") {
            return NaturalLang::Zh;
        }
        if val.starts_with("en") {
            return NaturalLang::En;
        }
    }

    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if val.starts_with("zh") || val.starts_with("zh_") {
                return NaturalLang::Zh;
            }
        }
    }

    NaturalLang::En
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --package algosketch-cli resolves_explicit_natural_lang`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/algosketch-cli/src/main.rs
git commit -m "feat(cli): add natural language resolution"
```

---

### Task 11: Add --lang, --no-pseudo, and --no-explain flags

**Files:**
- Modify: `crates/algosketch-cli/src/main.rs`

- [ ] **Step 1: Add failing help test in CLI tests**

Add to `crates/algosketch-cli/tests/cli.rs`:
```rust
#[test]
fn help_shows_explanation_flags() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--lang"))
        .stdout(predicate::str::contains("--no-pseudo"))
        .stdout(predicate::str::contains("--no-explain"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package algosketch-cli help_shows_explanation_flags`
Expected: FAIL because flags don't exist

- [ ] **Step 3: Add fields to Cli struct**

Add to `Cli` struct:
```rust
/// Disable pseudocode output.
#[arg(long = "no-pseudo")]
no_pseudo: bool,

/// Disable explanation output.
#[arg(long = "no-explain")]
no_explain: bool,

/// Natural language for explanations: zh | en | auto.
#[arg(long = "lang", value_enum, default_value_t = NaturalLangArg::Auto)]
lang: NaturalLangArg,
```

Note: positive `--pseudo`/`--explain` flags are intentionally skipped because pseudocode and explanation are enabled by default; the CLI only needs the negative toggles for M4.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package algosketch-cli help_shows_explanation_flags`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-cli/src/main.rs crates/algosketch-cli/tests/cli.rs
git commit -m "feat(cli): add explanation output flags"
```

---

### Task 12: Integrate ExplainRenderer into CLI output

**Files:**
- Modify: `crates/algosketch-cli/src/main.rs`

- [ ] **Step 1: Update imports**

```rust
use algosketch_core::ir::Item;
use algosketch_core::renderer::{ExplainRenderer, PseudoRenderer};
```

- [ ] **Step 2: Add public render_function helper to PseudoRenderer**

In `crates/algosketch-core/src/renderer/pseudo.rs`, add this public method:
```rust
pub fn render_function(&self, f: &Function) -> String {
    let mut out = String::new();
    self.render_function_into(f, &mut out);
    out
}
```

Then rename the existing private method:
```rust
fn render_function(&self, f: &Function, out: &mut String)
```
to:
```rust
fn render_function_into(&self, f: &Function, out: &mut String)
```

Update internal caller in `render_item`:
```rust
Item::Function(f) => self.render_function_into(f, out),
```

This keeps the existing module-level renderer behavior while allowing CLI code to render one function at a time.

- [ ] **Step 3: Replace run() output assembly**

Replace pseudocode-only section in `run` with:
```rust
let natural_lang = resolve_natural_lang(cli.lang);
let pseudo_renderer = PseudoRenderer { indent_width: cli.indent };
let explain_renderer = ExplainRenderer::new(natural_lang);

let show_pseudo = !cli.no_pseudo;
let show_explain = !cli.no_explain;

let mut sections = Vec::new();

for item in &module.items {
    if let Item::Function(f) = item {
        let mut func_output = String::new();

        if cli.format == OutFormat::Md {
            func_output.push_str(&format!("## {}\n\n", f.name));
        }

        if show_pseudo {
            let pseudo = pseudo_renderer.render_function(f);
            if cli.format == OutFormat::Md {
                if show_explain {
                    func_output.push_str("### Pseudocode\n\n");
                }
                func_output.push_str(&format!("```text\n{pseudo}```\n\n"));
            } else {
                func_output.push_str(&pseudo);
            }
        }

        if show_explain {
            let explain = explain_renderer.render_function(f);
            if cli.format == OutFormat::Md {
                let title = match natural_lang {
                    NaturalLang::Zh => "### 解释\n\n",
                    NaturalLang::En => "### Explanation\n\n",
                };
                func_output.push_str(title);
            }
            func_output.push_str(&explain);
            func_output.push('\n');
        }

        sections.push(func_output);
    }
}

let output = sections.join("\n");
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check --workspace`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-core/src/renderer/pseudo.rs crates/algosketch-cli/src/main.rs
git commit -m "feat(cli): integrate explanation renderer output"
```

---

## Chunk 5: CLI End-to-End Tests and Fixtures

### Task 13: Add binary_search fixture

**Files:**
- Create: `crates/algosketch-cli/fixtures/binary_search.py`

- [ ] **Step 1: Create fixtures directory and fixture file**

```bash
mkdir -p crates/algosketch-cli/fixtures
```

Create `crates/algosketch-cli/fixtures/binary_search.py`:
```python
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
```

- [ ] **Step 2: Commit**

```bash
git add crates/algosketch-cli/fixtures/binary_search.py
git commit -m "test(cli): add binary_search fixture"
```

---

### Task 14: Add CLI tests for explanation output

**Files:**
- Modify: `crates/algosketch-cli/tests/cli.rs`

- [ ] **Step 1: Add explain-only Chinese test**

```rust
#[test]
fn explains_only_in_chinese() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("crates/algosketch-cli/fixtures/binary_search.py")
        .arg("--no-pseudo")
        .arg("--lang")
        .arg("zh");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("函数 binary_search"))
        .stdout(predicate::str::contains("目的："))
        .stdout(predicate::str::contains("步骤："))
        .stdout(predicate::str::contains("FUNCTION binary_search").not());
}
```

- [ ] **Step 2: Add default pseudo+explain English test**

```rust
#[test]
fn outputs_pseudocode_and_explanation_by_default() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("crates/algosketch-cli/fixtures/binary_search.py")
        .arg("--lang")
        .arg("en");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("## binary_search"))
        .stdout(predicate::str::contains("### Pseudocode"))
        .stdout(predicate::str::contains("FUNCTION binary_search"))
        .stdout(predicate::str::contains("### Explanation"))
        .stdout(predicate::str::contains("Purpose:"))
        .stdout(predicate::str::contains("Steps:"));
}
```

- [ ] **Step 3: Add no-explain test**

```rust
#[test]
fn outputs_pseudocode_only() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("crates/algosketch-cli/fixtures/binary_search.py")
        .arg("--no-explain");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("FUNCTION binary_search"))
        .stdout(predicate::str::contains("Purpose:").not())
        .stdout(predicate::str::contains("目的：").not());
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --package algosketch-cli explains_only_in_chinese outputs_pseudocode_and_explanation_by_default outputs_pseudocode_only`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/algosketch-cli/tests/cli.rs
git commit -m "test(cli): cover explanation output flags"
```

---

### Task 15: Add locale detection CLI tests

**Files:**
- Modify: `crates/algosketch-cli/tests/cli.rs`

- [ ] **Step 1: Add LANG=zh_CN test**

```rust
#[test]
fn detects_chinese_locale_from_lang() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("crates/algosketch-cli/fixtures/binary_search.py")
        .env("LANG", "zh_CN.UTF-8");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("函数 binary_search"));
}
```

- [ ] **Step 2: Add PSEUDOCODE_LANG override test**

```rust
#[test]
fn pseudocode_lang_overrides_lang() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("crates/algosketch-cli/fixtures/binary_search.py")
        .env("LANG", "en_US.UTF-8")
        .env("PSEUDOCODE_LANG", "zh");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("函数 binary_search"));
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --package algosketch-cli detects_chinese_locale_from_lang pseudocode_lang_overrides_lang`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/algosketch-cli/tests/cli.rs
git commit -m "test(cli): cover locale detection"
```

---

## Chunk 6: README Documentation

### Task 16: Add output examples and references to README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add section after English Design highlights**

Insert after the English `### Design highlights` section and before `### License`:
```markdown
### Output Examples

#### Pseudocode

Language-neutral CLRS-style pseudocode. All source languages (Python/Java/C++) produce identical output:

- Uppercase keywords: `FUNCTION`, `IF`, `WHILE`, `FOR`, `RETURN`
- `←` for assignment
- `=` for equality comparison, `≠ ≤ ≥` for inequality
- `AND`, `OR`, `NOT` for logical operations
- `DIV`, `MOD` for integer division and modulo
- `LENGTH(x)` for length operation

Example:

```text
FUNCTION binary_search(nums, target)
    left ← 0
    right ← LENGTH(nums) - 1
    WHILE left ≤ right
        mid ← (left + right) DIV 2
        IF nums[mid] = target THEN
            RETURN mid
        ELSE IF nums[mid] < target THEN
            left ← mid + 1
        ELSE
            right ← mid - 1
        END IF
    END WHILE
    RETURN -1
END FUNCTION
```

#### Explanation

Natural-language description of algorithm logic. Supports Chinese and English (auto-detected from locale):

```text
Function binary_search(nums, target)

Purpose: search for the input (iteratively)

Steps:
  1. Assign left to 0
  2. Assign right to LENGTH(nums) - 1
  3. While left ≤ right, repeat:
    1. Assign mid to (left + right) DIV 2
    2. If nums[mid] = target, then:
      1. Return mid
    3. Otherwise if nums[mid] < target, then:
      1. Assign left to mid + 1
    4. Otherwise:
      1. Assign right to mid - 1
  4. Return -1
```

#### References

Pseudocode format references:
- Cormen, T. H., Leiserson, C. E., Rivest, R. L., & Stein, C. (2009). *Introduction to Algorithms* (3rd ed.). MIT Press.
- [《算法导论》中伪代码的约定](https://www.cnblogs.com/dreamapple/p/3080443.html)
- [Binary Search in Pseudocode](https://pseudoeditor.com/guides/binary-search)
```

- [ ] **Step 2: Add Chinese equivalent section after 中文设计要点**

Insert after the Chinese `### 设计要点` section and before `### 许可协议`:
```markdown
### 输出示例

#### 伪代码（Pseudocode）

语言无关的 CLRS 风格伪代码，所有源语言（Python/Java/C++）生成相同的输出：

- 大写关键字：`FUNCTION`、`IF`、`WHILE`、`FOR`、`RETURN`
- `←` 表示赋值
- `=` 表示相等比较，`≠ ≤ ≥` 表示不等比较
- `AND`、`OR`、`NOT` 表示逻辑运算
- `DIV`、`MOD` 表示整除和取模
- `LENGTH(x)` 表示长度操作

示例同英文部分。

#### 解释（Explanation）

自然语言描述算法逻辑，支持中文和英文（根据 locale 自动选择）：

```text
函数 binary_search(nums, target)

目的：查找输入数据（迭代）

步骤：
  1. 将 left 赋值为 0
  2. 将 right 赋值为 LENGTH(nums) - 1
  3. 当 left ≤ right 时重复以下步骤：
    1. 将 mid 赋值为 (left + right) DIV 2
    2. 如果 nums[mid] = target，则：
      1. 返回 mid
    3. 否则如果 nums[mid] < target，则：
      1. 将 left 赋值为 mid + 1
    4. 否则：
      1. 将 right 赋值为 mid - 1
  4. 返回 -1
```

#### 参考文献

伪代码格式参考：
- Cormen, T. H., Leiserson, C. E., Rivest, R. L., & Stein, C. (2009). *Introduction to Algorithms* (3rd ed.). MIT Press.
- [《算法导论》中伪代码的约定](https://www.cnblogs.com/dreamapple/p/3080443.html)
- [Binary Search in Pseudocode](https://pseudoeditor.com/guides/binary-search)
```

- [ ] **Step 3: Run markdown check by viewing diff**

Run: `git diff README.md`
Expected: README has English and Chinese output examples, no broken fences

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: document pseudocode and explanation output format"
```

---

## Chunk 7: Final Verification

### Task 17: Run full verification suite

**Files:**
- No file changes unless fixes are needed

- [ ] **Step 1: Run format check**

Run: `cargo fmt --check`
Expected: PASS

- [ ] **Step 2: If format fails, run formatter and commit**

Run: `cargo fmt`
Then:
```bash
git add crates/ Cargo.toml Cargo.lock
git commit -m "style: apply cargo fmt"
```

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

- [ ] **Step 4: Fix any clippy issues and commit**

If clippy fails, fix the exact warnings, then:
```bash
git add <changed-files>
git commit -m "fix: address clippy warnings"
```

- [ ] **Step 5: Run full test suite**

Run: `cargo test --workspace`
Expected: PASS

- [ ] **Step 6: Run manual CLI smoke test**

Run: `cargo run -- crates/algosketch-cli/fixtures/binary_search.py --lang zh`
Expected: Output includes `FUNCTION binary_search`, `### 解释`, `目的：`, `步骤：`

- [ ] **Step 7: Check git status**

Run: `git status --short`
Expected: Clean working tree

- [ ] **Step 8: Final commit if needed**

If any fixes were made:
```bash
git add <changed-files>
git commit -m "fix: finalize M4 explanation renderer"
```

---

## Implementation Notes

- Keep implementation small and direct. Do not introduce traits or external template files.
- Only explain `Item::Function`. Skip class/import/global/raw items silently.
- Use existing PseudoRenderer notation for expressions in explanations.
- Avoid unrelated refactors beyond splitting `renderer.rs` into module files for M4.
- If `cargo clippy` complains about duplicated expression rendering logic, do not over-abstract unless required; duplication is acceptable for now because renderer internals may diverge.

## Completion Criteria

M4 is done when:
- `cargo fmt --check` passes
- `cargo clippy --workspace -- -D warnings` passes
- `cargo test --workspace` passes
- Manual CLI smoke test passes for `--lang zh`
- README contains output examples and references
- Working tree is clean

