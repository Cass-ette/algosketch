# algosketch M4: Explanation Renderer — Design Spec

- Date: 2026-05-21
- Status: Approved (pending implementation)
- Milestone: M4
- Repo: https://github.com/Cass-ette/algosketch
- Author: Cass-ette

---

## 1. Purpose / 目标

M4 implements the **Explanation renderer** — a natural-language description generator that walks the IR and produces human-readable explanations of algorithm logic in Chinese or English.

This milestone completes the dual-output vision: pseudocode (M2) + explanation (M4), both independently toggleable via CLI flags.

## 2. Scope

**In scope:**
- `ExplainRenderer` struct with `lang: NaturalLang` field
- Function-level pattern recognition (search/sort/reverse, recursion/iteration detection)
- Statement-by-statement template-based explanation generation
- Locale auto-detection: `PSEUDOCODE_LANG` → `LC_ALL/LC_MESSAGES/LANG` → fallback to English
- CLI flags: `--pseudo/--no-pseudo`, `--explain/--no-explain`, `--lang zh|en|auto`
- Markdown output format: each function as `##` section with `### Pseudocode` and `### 解释/Explanation` subsections
- README update with output examples and references
- Unit tests (Chinese/English snapshots) and CLI end-to-end tests

**Out of scope:**
- Explanation for `Item::Class`, `Item::Import`, `Item::GlobalVar` — only `Item::Function` is explained (consistent with PseudoRenderer)
- Sophisticated algorithm pattern recognition (e.g., two-pointer, sliding window) — deferred to future LLM provider
- Multi-file or project-level analysis

## 3. Design Principles

1. **Simplicity over sophistication** — M4 validates the architecture (templates work, locale detection works, Chinese/English switch works), not building a perfect algorithm recognizer
2. **Symmetry with PseudoRenderer** — same structure (one struct, walk IR, emit strings), same ~200 lines of code
3. **Graceful fallback** — unrecognized patterns use generic templates, never fail
4. **CLRS-inspired format** — explanation references pseudocode logic (use `←`, `DIV`, `LENGTH` in explanations for consistency)

## 4. Core Types

### NaturalLang enum

```rust
// lib.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NaturalLang {
    Zh,
    En,
}
```

### ExplainRenderer struct

```rust
// renderer/explain.rs
pub struct ExplainRenderer {
    pub lang: NaturalLang,
}

impl ExplainRenderer {
    pub fn new(lang: NaturalLang) -> Self {
        Self { lang }
    }
    
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
    
    fn render_function(&self, f: &Function) -> String {
        // 1. Function header
        // 2. Purpose line (pattern recognition)
        // 3. Steps list (statement-by-statement walk)
    }
}
```

## 5. Pattern Recognition (Function-level Summary)

Simple heuristics to generate the "Purpose" line:

**Detection rules:**
- Function name contains `search` or `find` → "查找" / "search for"
- Function name contains `sort` → "排序" / "sort"
- Function name contains `reverse` → "反转" / "reverse"
- Body contains recursive call (function calls itself) → add "（递归）" / " (recursively)"
- Body contains loop (`While` or `For`) → add "（迭代）" / " (iteratively)"
- No match → generic "处理输入数据" / "process the input"

**Implementation sketch:**

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
        match self.lang { NaturalLang::Zh => "排序", NaturalLang::En => "sort" }
    } else if name_lower.contains("reverse") {
        match self.lang { NaturalLang::Zh => "反转", NaturalLang::En => "reverse" }
    } else {
        match self.lang { NaturalLang::Zh => "处理", NaturalLang::En => "process" }
    };
    
    let method = if has_recursion {
        match self.lang { NaturalLang::Zh => "（递归）", NaturalLang::En => " (recursively)" }
    } else if has_loop {
        match self.lang { NaturalLang::Zh => "（迭代）", NaturalLang::En => " (iteratively)" }
    } else {
        ""
    };
    
    match self.lang {
        NaturalLang::Zh => format!("{}输入数据{}", action, method),
        NaturalLang::En => format!("{} the input{}", action, method),
    }
}

fn has_loop(&self, block: &Block) -> bool {
    block.0.iter().any(|stmt| matches!(stmt, Stmt::While { .. } | Stmt::For { .. }))
}

fn has_recursion(&self, f: &Function) -> bool {
    // Walk body, check if any Call expr's callee is Ident(f.name)
}
```

## 6. Statement-by-Statement Templates

Walk the function body and generate natural-language descriptions for each statement type:

**Template mapping:**

| Stmt variant | Chinese template | English template |
|--------------|------------------|------------------|
| `Assign { target, value }` | "将 {target} 赋值为 {value}" | "Assign {target} to {value}" |
| `If { cond, then_block, else_block }` | "如果 {cond}，则：" | "If {cond}, then:" |
| `While { cond, body }` | "当 {cond} 时重复以下步骤：" | "While {cond}, repeat:" |
| `For { kind, body }` | (depends on ForKind) | (depends on ForKind) |
| `Return(expr)` | "返回 {expr}" | "Return {expr}" |
| `Break` | "跳出循环" | "Break out of loop" |
| `Continue` | "继续下一次迭代" | "Continue to next iteration" |
| `ExprStmt(e)` | "{e}" | "{e}" |
| `Raw(text)` | "> 此处源代码未能结构化解析，已原样保留。" | "> Unparsed source preserved as-is." |

**Nested blocks** (then_block, else_block, loop body) are rendered recursively with increased indentation.

**Implementation sketch:**

```rust
fn render_steps(&self, block: &Block, depth: usize) -> String {
    let mut out = String::new();
    for (i, stmt) in block.0.iter().enumerate() {
        let step_num = format!("{}. ", i + 1);
        let indent = "  ".repeat(depth);
        out.push_str(&format!("{}{}", indent, step_num));
        self.render_stmt(stmt, depth, &mut out);
    }
    out
}

fn render_stmt(&self, stmt: &Stmt, depth: usize, out: &mut String) {
    match stmt {
        Stmt::Assign { target, value } => {
            let tmpl = match self.lang {
                NaturalLang::Zh => "将 {} 赋值为 {}",
                NaturalLang::En => "Assign {} to {}",
            };
            out.push_str(&format!("{}\n", 
                format_args!(tmpl, expr_to_text(target), expr_to_text(value))));
        }
        
        Stmt::If { cond, then_block, else_block } => {
            let tmpl = match self.lang {
                NaturalLang::Zh => "如果 {}，则：",
                NaturalLang::En => "If {}, then:",
            };
            out.push_str(&format!("{}\n", format_args!(tmpl, expr_to_text(cond))));
            out.push_str(&self.render_steps(then_block, depth + 1));
            
            if let Some(else_blk) = else_block {
                let else_tmpl = match self.lang { 
                    NaturalLang::Zh => "否则：", 
                    NaturalLang::En => "Otherwise:" 
                };
                out.push_str(&format!("{}  {}\n", "  ".repeat(depth), else_tmpl));
                out.push_str(&self.render_steps(else_blk, depth + 1));
            }
        }
        
        Stmt::While { cond, body } => {
            let tmpl = match self.lang {
                NaturalLang::Zh => "当 {} 时重复以下步骤：",
                NaturalLang::En => "While {}, repeat:",
            };
            out.push_str(&format!("{}\n", format_args!(tmpl, expr_to_text(cond))));
            out.push_str(&self.render_steps(body, depth + 1));
        }
        
        Stmt::Return(expr) => {
            let tmpl = match self.lang {
                NaturalLang::Zh => "返回 {}",
                NaturalLang::En => "Return {}",
            };
            let val = expr.as_ref().map(expr_to_text).unwrap_or_else(|| "".to_string());
            out.push_str(&format!("{}\n", format_args!(tmpl, val)));
        }
        
        // For/Break/Continue/ExprStmt/Raw similar handling
        _ => { /* ... */ }
    }
}

// Helper: convert Expr to text using pseudocode notation (←, DIV, LENGTH, etc.)
fn expr_to_text(expr: &Expr) -> String {
    // Reuse logic from PseudoRenderer's render_expr
}
```

## 7. Locale Auto-Detection

Resolve `NaturalLang` from CLI argument or environment:

```rust
fn resolve_natural_lang(arg: NaturalLangArg) -> NaturalLang {
    match arg {
        NaturalLangArg::Zh => NaturalLang::Zh,
        NaturalLangArg::En => NaturalLang::En,
        NaturalLangArg::Auto => detect_locale(),
    }
}

fn detect_locale() -> NaturalLang {
    // 1. PSEUDOCODE_LANG environment variable (escape hatch)
    if let Ok(val) = std::env::var("PSEUDOCODE_LANG") {
        if val.starts_with("zh") { return NaturalLang::Zh; }
        if val.starts_with("en") { return NaturalLang::En; }
    }
    
    // 2. LC_ALL → LC_MESSAGES → LANG (standard locale chain)
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if val.starts_with("zh") || val.starts_with("zh_") {
                return NaturalLang::Zh;
            }
        }
    }
    
    // 3. Fallback: English
    NaturalLang::En
}
```

## 8. CLI Integration

### New CLI flags

```rust
#[derive(Debug, Parser)]
struct Cli {
    // ... existing fields ...
    
    /// Enable pseudocode output (default: true)
    #[arg(long = "pseudo", default_value_t = true)]
    pseudo: bool,
    
    /// Disable pseudocode output
    #[arg(long = "no-pseudo", conflicts_with = "pseudo")]
    no_pseudo: bool,
    
    /// Enable explanation output (default: true)
    #[arg(long = "explain", default_value_t = true)]
    explain: bool,
    
    /// Disable explanation output
    #[arg(long = "no-explain", conflicts_with = "explain")]
    no_explain: bool,
    
    /// Natural language for explanation: zh | en | auto
    #[arg(long = "lang", value_enum, default_value_t = NaturalLangArg::Auto)]
    lang: NaturalLangArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum NaturalLangArg {
    Zh,
    En,
    Auto,
}
```

### Output assembly logic

```rust
fn run(cli: Cli) -> Result<(), PseudoError> {
    // ... parse source into module ...
    
    let show_pseudo = cli.pseudo && !cli.no_pseudo;
    let show_explain = cli.explain && !cli.no_explain;
    
    let mut sections = Vec::new();
    
    for item in &module.items {
        if let Item::Function(f) = item {
            let mut func_output = String::new();
            
            // Markdown: function name as ## heading
            if cli.format == OutFormat::Md {
                func_output.push_str(&format!("## {}\n\n", f.name));
            }
            
            // Pseudocode section
            if show_pseudo {
                let pseudo = PseudoRenderer { indent_width: cli.indent }
                    .render_function(f);
                
                if cli.format == OutFormat::Md {
                    if show_explain {
                        func_output.push_str("### Pseudocode\n\n");
                    }
                    func_output.push_str(&format!("```text\n{pseudo}```\n\n"));
                } else {
                    func_output.push_str(&pseudo);
                }
            }
            
            // Explanation section
            if show_explain {
                let natural_lang = resolve_natural_lang(cli.lang);
                let explain = ExplainRenderer::new(natural_lang)
                    .render_function(f);
                
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
    
    // Write to file or stdout
    if let Some(path) = cli.output {
        fs::write(path, output)?;
    } else {
        write_to_stdout(&output)?;
    }
    
    Ok(())
}
```

## 9. Output Format Examples

### Chinese example

```
## binary_search

### Pseudocode

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

### 解释

函数 binary_search(nums, target)

目的：在有序数组中查找目标值（迭代）

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

### English example

```
## binary_search

### Pseudocode

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

### Explanation

Function binary_search(nums, target)

Purpose: Search for target value in sorted array (iteratively)

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

## 10. Testing Strategy

### Unit tests (renderer/explain.rs)

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
    
    assert!(out.contains("函数 binary_search"));
    assert!(out.contains("目的："));
    assert!(out.contains("查找"));
    assert!(out.contains("步骤："));
    assert!(out.contains("初始化") || out.contains("赋值"));
}

#[test]
fn renders_binary_search_explanation_en() {
    let source = /* same */;
    let module = PythonParser::new().parse(source).unwrap();
    let out = ExplainRenderer::new(NaturalLang::En).render_module(&module);
    
    assert!(out.contains("Function binary_search"));
    assert!(out.contains("Purpose:"));
    assert!(out.contains("search"));
    assert!(out.contains("Steps:"));
    assert!(out.contains("Assign"));
}

#[test]
fn detects_recursion_pattern() {
    let source = r#"
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)
"#;
    let module = PythonParser::new().parse(source).unwrap();
    let out = ExplainRenderer::new(NaturalLang::Zh).render_module(&module);
    
    assert!(out.contains("递归"));
}

#[test]
fn detects_iteration_pattern() {
    let source = r#"
def sum_array(nums):
    total = 0
    for x in nums:
        total += x
    return total
"#;
    let module = PythonParser::new().parse(source).unwrap();
    let out = ExplainRenderer::new(NaturalLang::En).render_module(&module);
    
    assert!(out.contains("iteratively"));
}
```

### CLI end-to-end tests (cli/tests/cli.rs)

```rust
#[test]
fn test_explain_only_zh() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("fixtures/binary_search.py")
        .arg("--no-pseudo")
        .arg("--lang").arg("zh");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("函数 binary_search"))
        .stdout(predicate::str::contains("目的："))
        .stdout(predicate::str::contains("步骤："));
}

#[test]
fn test_pseudo_and_explain() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("fixtures/binary_search.py")
        .arg("--lang").arg("en");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("FUNCTION binary_search"))
        .stdout(predicate::str::contains("Purpose:"))
        .stdout(predicate::str::contains("Steps:"));
}

#[test]
fn test_locale_detection() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("fixtures/binary_search.py")
        .env("LANG", "zh_CN.UTF-8");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("函数"));
}

#[test]
fn test_pseudocode_lang_env_override() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("fixtures/binary_search.py")
        .env("LANG", "en_US.UTF-8")
        .env("PSEUDOCODE_LANG", "zh");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("函数"));
}
```

## 11. README Update

Add a new section after "设计要点 / Design highlights":

```markdown
### 输出示例 / Output Examples

#### 伪代码（Pseudocode）

语言无关的 CLRS 风格伪代码，所有源语言（Python/Java/C++）生成相同的输出：

Language-neutral CLRS-style pseudocode. All source languages (Python/Java/C++) produce identical output:

- 大写关键字 / Uppercase keywords: `FUNCTION`, `IF`, `WHILE`, `FOR`, `RETURN`
- `←` 表示赋值 / for assignment
- `=` 表示相等比较 / for equality comparison, `≠ ≤ ≥` for inequality
- `AND`, `OR`, `NOT` 表示逻辑运算 / for logical operations
- `DIV`, `MOD` 表示整除和取模 / for integer division and modulo
- `LENGTH(x)` 表示长度操作 / for length operation

示例 / Example:
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

#### 解释（Explanation）

自然语言描述算法逻辑，支持中文和英文（根据 locale 自动选择）：

Natural-language description of algorithm logic. Supports Chinese and English (auto-detected from locale):

**中文示例 / Chinese example:**
```
函数 binary_search(nums, target)

目的：在有序数组中查找目标值（迭代）

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

**English example:**
```
Function binary_search(nums, target)

Purpose: Search for target value in sorted array (iteratively)

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

#### 参考文献 / References

伪代码格式参考 / Pseudocode format references:
- Cormen, T. H., Leiserson, C. E., Rivest, R. L., & Stein, C. (2009). *Introduction to Algorithms* (3rd ed.). MIT Press.
- [《算法导论》中伪代码的约定](https://www.cnblogs.com/dreamapple/p/3080443.html)
- [Binary Search in Pseudocode](https://pseudoeditor.com/guides/binary-search)
```

## 12. File Changes Summary

**New files:**
- `crates/algosketch-core/src/renderer/explain.rs` (~200 lines)

**Modified files:**
- `crates/algosketch-core/src/renderer/mod.rs` — export `ExplainRenderer`
- `crates/algosketch-core/src/lib.rs` — add `NaturalLang` enum
- `crates/algosketch-cli/src/main.rs` — CLI flags, locale detection, output assembly
- `README.md` — add "输出示例 / Output Examples" section
- `crates/algosketch-cli/tests/cli.rs` — add CLI tests for explanation flags and locale

**Test fixtures needed:**
- `crates/algosketch-cli/fixtures/binary_search.py` (if not already present)

## 13. Acceptance Criteria

M4 is complete when:

1. ✅ `ExplainRenderer` renders Chinese and English explanations for `binary_search.py`
2. ✅ Pattern recognition detects search/sort/reverse, recursion/iteration
3. ✅ Locale auto-detection works: `PSEUDOCODE_LANG` → `LC_ALL/LC_MESSAGES/LANG` → English fallback
4. ✅ CLI flags `--pseudo/--no-pseudo`, `--explain/--no-explain`, `--lang zh|en|auto` work correctly
5. ✅ Markdown output format: `##` function heading, `### Pseudocode`, `### 解释/Explanation`
6. ✅ Unit tests pass (Chinese/English snapshots, pattern detection)
7. ✅ CLI end-to-end tests pass (explain-only, pseudo+explain, locale detection)
8. ✅ README updated with output examples and references
9. ✅ `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` all pass

---

This spec is the source of truth for M4. Material changes must be made by editing this file in a PR.
