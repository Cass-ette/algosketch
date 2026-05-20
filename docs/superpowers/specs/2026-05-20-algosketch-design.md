# algosketch — Design Spec

- Date: 2026-05-20
- Status: Approved (initial draft, pending implementation)
- Repo: https://github.com/Cass-ette/algosketch
- License: MIT OR Apache-2.0
- Author: Cass-ette

---

## 1. Purpose / 目标

`algosketch` turns real source code into language-neutral pseudocode and a
human-readable explanation, primarily targeted at:

- algorithm learning and review (LeetCode-style code, course assignments,
  competitive code)
- sharing algorithms across language communities without forcing readers to
  know the source language

The MVP scope is intentionally narrow: a CLI, single-file input, three source
languages (Python, Java, C++), rule-based parsing only.

## 2. Non-goals (v0.1)

- Cross-file / project-level analysis, dependency graphs, build-system
  awareness.
- Full type inference or semantic analysis (we deliberately operate on
  syntax-level structure).
- LLM-backed generation. A `provider` trait is reserved but unused in v0.1.
- VS Code extension, Web UI, language-server. Web comes after v0.1.

## 3. High-level architecture

```text
┌──────────────────────────────────────────────────────────────┐
│                       algosketch-cli                         │
│      (clap parses args → calls core → writes output)         │
└──────────────────────────────┬───────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────┐
│                    algosketch-core (lib)                     │
│  ┌──────────┐    ┌──────────┐    ┌────────────────────────┐  │
│  │  parser  │───▶│    ir    │───▶│       renderer         │  │
│  │  (TS)    │    │ (枢纽)   │    │ pseudo / explain(zh|en)│  │
│  └──────────┘    └──────────┘    └────────────────────────┘  │
│       ▲                                                      │
│       │ Python / Java / C++ adapters                         │
│  ┌──────────┐                                                │
│  │ provider │  ← reserved trait, no implementation in v0.1   │
│  └──────────┘                                                │
└──────────────────────────────────────────────────────────────┘
```

Key invariants:

- The **IR is the only stable interface** between parsing and rendering.
- Parsers know nothing about renderers. Renderers know nothing about source
  languages.
- A `provider` trait exists from day one (default `NoopProvider`) so future
  LLM-backed rendering can be added without rewiring the architecture.

## 4. Crate layout

```text
algosketch/
├── Cargo.toml                       # workspace
├── crates/
│   ├── algosketch-core/             # parsing + IR + rendering (library)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── error.rs
│   │   │   ├── parser/              # tree-sitter adapters
│   │   │   │   ├── mod.rs
│   │   │   │   ├── python.rs
│   │   │   │   ├── java.rs
│   │   │   │   └── cpp.rs
│   │   │   ├── ir/
│   │   │   │   └── mod.rs
│   │   │   ├── renderer/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pseudo.rs
│   │   │   │   ├── explain/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── templates_zh.rs
│   │   │   │   │   └── templates_en.rs
│   │   │   └── provider.rs          # reserved trait
│   │   └── tests/                   # parser/renderer unit tests + snapshots
│   └── algosketch-cli/              # CLI binary
│       ├── src/
│       │   └── main.rs
│       └── tests/
│           └── cli.rs               # assert_cmd end-to-end tests
└── docs/
    └── superpowers/specs/
        └── 2026-05-20-algosketch-design.md
```

## 5. IR (Intermediate Representation)

The IR is a **purpose-built, language-neutral, structure-only tree**. It is
not a full AST — language-specific noise (modifiers, decorators, namespaces)
is deliberately dropped.

### Design principles

1. **Structure over syntax**: keep control flow and data flow, drop syntax
   sugar that does not affect understanding.
2. **Graceful fallback**: any node we cannot map is preserved as `Raw(String)`
   and emitted verbatim by renderers. The tool must never crash on
   well-formed input.
3. **Spans preserved**: every node carries a `Span { start, end }` so
   diagnostics and future side-by-side rendering remain possible.

### Top-level types (sketch)

```rust
pub struct Module {
    pub source_language: SourceLang,   // Python / Java / Cpp
    pub items: Vec<Item>,
}

pub enum Item {
    Function(Function),
    Class(Class),
    Import(Import),
    GlobalVar(VarDecl),
    Raw(String),
}

pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeHint>,
    pub body: Block,
    pub span: Span,
}

pub struct Class {
    pub name: String,
    pub parents: Vec<String>,
    pub fields: Vec<VarDecl>,
    pub methods: Vec<Function>,
    pub span: Span,
}

pub struct Block(pub Vec<Stmt>);

pub enum Stmt {
    Assign { target: Expr, value: Expr },
    VarDecl(VarDecl),
    If { cond: Expr, then_block: Block, else_block: Option<Block> },
    While { cond: Expr, body: Block },
    For { kind: ForKind, body: Block },
    Return(Option<Expr>),
    Break,
    Continue,
    ExprStmt(Expr),
    Raw(String),
}

pub enum ForKind {
    CStyle { init: Box<Stmt>, cond: Expr, step: Expr },
    ForEach { var: String, iter: Expr },
    Range  { var: String, start: Expr, end: Expr, step: Option<Expr> },
}

pub enum Expr {
    Literal(Literal),
    Ident(String),
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Unary  { op: UnOp,  expr: Box<Expr> },
    Call   { callee: Box<Expr>, args: Vec<Expr> },
    Index  { obj: Box<Expr>, index: Box<Expr> },
    Field  { obj: Box<Expr>, name: String },
    Raw(String),
}
```

### Parser contract

```rust
pub trait LanguageParser {
    fn language(&self) -> SourceLang;
    fn parse(&self, source: &str) -> Result<Module, PseudoError>;
}
```

One implementation per language. Each adapter uses tree-sitter to obtain a
CST and walks it into the IR. Any node it does not recognize becomes
`Stmt::Raw(original_text)` or `Expr::Raw(original_text)`.

## 6. Renderers

A renderer is a pure function `(Module, RenderOptions) -> String`.

### Options

```rust
pub struct RenderOptions {
    pub pseudo: bool,        // --pseudo / --no-pseudo
    pub explain: bool,       // --explain / --no-explain
    pub lang: NaturalLang,   // Zh | En
    pub indent: usize,       // default 2
}
```

### Pseudocode renderer (CLRS-style)

Normalization rules:

| Source concept                       | Pseudocode form        |
| ------------------------------------ | ---------------------- |
| `=` assignment                       | `←`                    |
| `==` comparison                      | `=`                    |
| `!=`                                 | `≠`                    |
| `<=` / `>=`                          | `≤` / `≥`              |
| `&&` / `\|\|` / `!`                  | `AND` / `OR` / `NOT`   |
| `len(x)` / `x.size()` / `x.length`   | `LENGTH(x)`            |
| `//` / integer-`/`                   | `DIV`                  |
| `%`                                  | `MOD`                  |
| `for x in xs` / `for (T x : xs)`     | `FOR EACH x IN xs`     |
| `for (i=0;i<n;i++)`                  | `FOR i ← 0 TO n - 1`   |
| `range(a, b)`                        | `FOR i ← a TO b - 1`   |

Sample output:

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

`Raw` nodes are emitted verbatim with a trailing `// <unparsed>` marker.

### Explanation renderer

A hierarchical natural-language summary, per function / class. Strategy in
v0.1 is fully template-based, no LLM:

1. **Per-function summary line**: heuristic from function name + signature +
   top-level structure (search / sort / compute / loop over collection /
   recursive call).
2. **Stepwise list**: walk the top-level `Block`, one bullet per statement;
   nested `If` / `While` / `For` indented as sub-bullets.
3. **Statement templates** live in `templates_zh.rs` and `templates_en.rs`,
   keyed by `Stmt` variant.

Sample (Chinese):

```text
函数 binary_search(nums, target):
  目的：在有序数组 nums 中查找 target，返回其下标，找不到则返回 -1。
  步骤：
    1. 初始化左右指针 left = 0，right = nums 长度 - 1。
    2. 当 left ≤ right 时，循环执行：
       - 计算中点 mid = (left + right) / 2。
       - 若 nums[mid] 等于 target，返回 mid。
       - 否则若 nums[mid] 小于 target，把 left 移到 mid + 1。
       - 否则把 right 移到 mid - 1。
    3. 循环结束仍未找到，返回 -1。
```

### Output format

Default: Markdown wrapping. Each function/class becomes a `##` section with
a `### Pseudocode` fenced code block and a `### 解释 / Explanation`
sub-section.

`--format text` strips Markdown decoration and emits plain text, useful for
piping to other tools.

## 7. CLI

```text
USAGE:
    algosketch <INPUT> [OPTIONS]

ARGS:
    <INPUT>                       Path to source file, or "-" for stdin

OPTIONS:
    -l, --source-lang <LANG>      python | java | cpp (auto from extension)
        --pseudo / --no-pseudo    Toggle pseudocode output       [default: on]
        --explain / --no-explain  Toggle explanation output      [default: on]
        --lang <NAT>              zh | en | auto                 [default: auto]
        --format <FMT>            md | text                      [default: md]
    -o, --output <FILE>           Write to FILE instead of stdout
        --indent <N>              Indent width                   [default: 2]
        --pseudo-only             Shortcut for --no-explain
        --explain-only            Shortcut for --no-pseudo
    -q, --quiet                   Suppress warnings
    -h, --help / -V, --version
```

Extension → language map (no content sniffing):

| Extension                              | Language |
| -------------------------------------- | -------- |
| `.py`                                  | Python   |
| `.java`                                | Java     |
| `.cpp` `.cc` `.cxx` `.hpp` `.h`        | C++      |

stdin requires explicit `--source-lang`.

`--lang auto` resolves in order:

1. `PSEUDOCODE_LANG` env var (escape hatch)
2. `LC_ALL` → `LC_MESSAGES` → `LANG`
3. Prefix match: `zh*` → Chinese; otherwise English
4. Fallback: English

## 8. Error model

Single error enum in core, mapped to exit codes in CLI:

```rust
#[derive(thiserror::Error, Debug)]
pub enum PseudoError {
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),
    #[error("cannot infer source language; pass --source-lang")]
    UnknownLanguage,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error in {file}: {message}")]
    Parse { file: String, message: String },
    #[error("internal error: {0}")]
    Internal(String),
}
```

Exit codes:

| Code | Meaning                                                                    |
| ---- | -------------------------------------------------------------------------- |
| 0    | Success (Raw fallback nodes do not constitute failure)                     |
| 1    | User error: missing/unreadable file, invalid flag combo, unknown language  |
| 2    | Parse error tree-sitter cannot recover from                                |
| 3    | Internal error (panic caught, surfaced as exit 3, no raw backtrace)        |

Unparsed-node behavior:

- Pseudocode: emit source verbatim + ` // <unparsed>`.
- Explanation: insert one `> 此处源代码未能结构化解析，已原样保留。` /
  `> Unparsed source preserved as-is.` line.
- stderr (unless `-q`): one summary line, e.g.
  `warning: 3 unparsed nodes in input.cpp (lines 12, 45, 67)`

## 9. Testing strategy

Layered:

- **Parser unit tests** per language: source → IR (snapshot via `insta`).
- **Renderer unit tests**: IR → pseudocode, IR → zh, IR → en (snapshots).
- **Cross-language skeleton test**: for each fixture, the same algorithm in
  three languages must produce equivalent control-flow IR skeletons. This is
  the canary that proves the IR abstraction works.
- **End-to-end fixtures**: 15 samples (5 algorithms × 3 languages):
  `binary_search`, `reverse_string`, `reverse_linked_list`, `quick_sort`,
  `two_sum`.
- **CLI tests** via `assert_cmd`: exit codes, stdout/stderr, `--output`,
  stdin path, unparsed warnings.

CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`,
`cargo build --release`.

## 10. Milestones

| M  | Name             | Acceptance criteria                                                                 |
| -- | ---------------- | ----------------------------------------------------------------------------------- |
| M1 | Skeleton         | Cargo workspace builds; CLI prints `--help`; CI green.                              |
| M2 | Python → pseudo  | Python parser + IR + PseudoRenderer; `binary_search.py` end-to-end passes.          |
| M3 | Java + C++       | Both parsers in; cross-language skeleton test passes for all 5 fixtures.            |
| M4 | Explanation      | Templates_zh/en in; `--explain`, `--lang`, locale auto-detect all work.             |
| M5 | Polish           | Markdown output, `Raw` fallback warnings, exit codes, full `assert_cmd` suite pass. |

v0.1.0 = M1 through M5. Out-of-scope future tracks:

- v0.2: LLM provider hooked in for `Raw` node fallback.
- v0.3: WASM build + minimal web UI.

## 11. Open questions (intentionally deferred)

- Tree-sitter grammar pinning policy: vendor as crate dep or as git submodule?
  Decide at M2 when concrete dep choices are made.
- Whether to surface a `--debug-ir` flag dumping the IR as JSON. Useful for
  contributors; defer to M5 if cheap.
- Output color/ANSI handling. Defer until users ask.

---

This spec is the source of truth for v0.1. Material changes must be made by
editing this file in a PR.
