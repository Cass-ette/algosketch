# algosketch

> Turn real source code into pseudocode and human-readable explanations — for learning, reviewing, and sharing algorithms.
>
> 把真实的源代码翻译成伪代码与人类可读的解释 —— 用于学习、回顾和分享算法。

**Status / 状态**: Pre-alpha — scaffolding only. See [`docs/superpowers/specs/2026-05-20-algosketch-design.md`](docs/superpowers/specs/2026-05-20-algosketch-design.md) for the full design.

---

## English

`algosketch` is a CLI that reads a single source file (Python, Java, or C++) and produces:

- A language-neutral **pseudocode** rendering in CLRS-style (uppercase keywords, `←` assignment, `≤ ≥ ≠`).
- A natural-language **explanation** of what the code does, in Chinese or English (auto-detected from your locale).

Both outputs are independently toggleable. Default output format is Markdown.

### Planned usage

```bash
# Pseudocode + explanation (locale-aware)
algosketch input.py

# Pseudocode only
algosketch Solution.java --no-explain

# Explanation only, in English
algosketch main.cpp --no-pseudo --lang en

# From stdin
cat snippet.py | algosketch - --source-lang python
```

### Design highlights

- Rust workspace, single binary, three tree-sitter grammars (Python / Java / C++) compiled in.
- Source code is normalized into a **unified IR** so renderers do not care which language the input came from.
- Provider trait reserved for future LLM-backed mode; v0.1 is fully rule-based.
- CLI first, Web later: core is a library, ready for WASM or HTTP backend down the line.

See the design spec for the full architecture, IR shape, error model, and milestones.

### License

Dual-licensed under either of:

- MIT — see [`LICENSE-MIT`](LICENSE-MIT)
- Apache-2.0 — see [`LICENSE-APACHE`](LICENSE-APACHE)

at your option.

---

## 中文

`algosketch` 是一个命令行工具，读取一份 Python / Java / C++ 源文件，生成：

- 与具体编程语言无关的**伪代码**（算法导论风格：大写关键字、`←` 赋值、`≤ ≥ ≠`）。
- 这段代码"在做什么"的**自然语言解释**，中文或英文，自动跟随系统 locale。

两路输出可以独立开关。默认输出格式为 Markdown。

### 计划中的用法

```bash
# 伪代码 + 解释（自动选择中/英文）
algosketch input.py

# 只要伪代码
algosketch Solution.java --no-explain

# 只要英文解释
algosketch main.cpp --no-pseudo --lang en

# 从标准输入读取
cat snippet.py | algosketch - --source-lang python
```

### 设计要点

- Rust workspace，单二进制，三种 tree-sitter grammar（Python / Java / C++）直接编进可执行文件。
- 输入源码先归一化为一份**统一 IR**，渲染器完全不关心源语言是什么。
- 预留 provider trait，将来可接入 LLM；v0.1 完全走规则路径。
- CLI 优先，Web 在后：核心是库，方便后续编 WASM 或做 HTTP 后端。

完整架构、IR 形状、错误模型与里程碑见设计文档。

### 许可协议

双授权，任选其一：

- MIT — 见 [`LICENSE-MIT`](LICENSE-MIT)
- Apache-2.0 — 见 [`LICENSE-APACHE`](LICENSE-APACHE)
