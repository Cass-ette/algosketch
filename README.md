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

### Output Examples

#### Pseudocode

`algosketch` renders language-neutral, CLRS-style pseudocode. Python, Java, and C++ inputs that represent the same algorithm produce identical pseudocode output.

Conventions:

- Keywords are uppercase, such as `FUNCTION`, `IF`, `THEN`, `ELSE`, `WHILE`, `FOR`, and `RETURN`.
- Assignment uses `←`.
- Equality and inequality use mathematical operators such as `=`, `≠`, `≤`, `≥`, `<`, and `>`.
- Logical operations use uppercase words such as `AND`, `OR`, and `NOT`.
- Integer division and remainder use `DIV` and `MOD`.
- Sequence length is rendered as `LENGTH(x)`.

```text
FUNCTION binary_search(nums, target)
  left, right ← 0, LENGTH(nums) - 1
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

For `binary_search`, the English explanation is structured as a purpose plus numbered steps:

```text
Function binary_search(nums, target)

Purpose: search for the input (iteratively)

Steps:
  1. Assign left, right to 0, LENGTH(nums) - 1
  2. While left ≤ right, repeat:
    1. Assign mid to (left + right) DIV 2
    2. If nums[mid] = target, then:
      1. Return mid
    Otherwise if nums[mid] < target, then:
      1. Assign left to mid + 1
    Otherwise:
      1. Assign right to mid - 1
  3. Return -1
```

#### References

The pseudocode style is informed by these references:

- Cormen, T. H., Leiserson, C. E., Rivest, R. L., & Stein, C. (2009). *Introduction to Algorithms* (3rd ed.). MIT Press.
- [《算法导论》中伪代码的约定](https://www.cnblogs.com/dreamapple/p/3080443.html)
- [Binary Search in Pseudocode](https://pseudoeditor.com/guides/binary-search)

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

### 输出示例

#### 伪代码（Pseudocode）

`algosketch` 输出与具体编程语言无关、接近《算法导论》风格的伪代码。表达同一算法的 Python、Java 和 C++ 输入会生成一致的伪代码结果。

约定：

- 关键字使用大写，例如 `FUNCTION`、`IF`、`THEN`、`ELSE`、`WHILE`、`FOR` 和 `RETURN`。
- 赋值使用 `←`。
- 相等与不等关系使用数学符号，例如 `=`、`≠`、`≤`、`≥`、`<` 和 `>`。
- 逻辑运算使用大写单词，例如 `AND`、`OR` 和 `NOT`。
- 整数除法与取余使用 `DIV` 和 `MOD`。
- 序列长度写作 `LENGTH(x)`。

伪代码示例与英文部分的 `binary_search` 示例相同。

#### 解释（Explanation）

对于 `binary_search`，中文解释会按目的和编号步骤组织：

```text
函数 binary_search(nums, target)

目的：查找输入数据（迭代）

步骤：
  1. 将 left, right 赋值为 0, LENGTH(nums) - 1
  2. 当 left ≤ right 时重复以下步骤：
    1. 将 mid 赋值为 (left + right) DIV 2
    2. 如果 nums[mid] = target，则：
      1. 返回 mid
    否则如果 nums[mid] < target，则：
      1. 将 left 赋值为 mid + 1
    否则：
      1. 将 right 赋值为 mid - 1
  3. 返回 -1
```

#### 参考文献

伪代码风格主要参考：

- Cormen, T. H., Leiserson, C. E., Rivest, R. L., & Stein, C. (2009). *Introduction to Algorithms* (3rd ed.). MIT Press.
- [《算法导论》中伪代码的约定](https://www.cnblogs.com/dreamapple/p/3080443.html)
- [Binary Search in Pseudocode](https://pseudoeditor.com/guides/binary-search)

### 许可协议

双授权，任选其一：

- MIT — 见 [`LICENSE-MIT`](LICENSE-MIT)
- Apache-2.0 — 见 [`LICENSE-APACHE`](LICENSE-APACHE)
