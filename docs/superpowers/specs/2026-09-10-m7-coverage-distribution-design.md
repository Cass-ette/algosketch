# M7 Coverage + Distribution — Design Spec

- Date: 2026-09-10
- Status: Approved (design)
- Baseline: v0.2.0 (main @ `bdf6c64`)
- Release vehicle: **v0.2.1**

## 1. Decisions (locked)

| # | Question | Decision |
| - | -------- | -------- |
| 1 | Scope | Parser coverage for the highest-frequency real-world gaps + crates.io publish readiness; no renderer/IR/architecture changes |
| 2 | Version | 0.2.1 (feature additions within the 0.2 line; roadmap unchanged: v0.3 LLM, v0.4 WASM) |
| 3 | crates.io names | All three available (verified 2026-09-10); publish is prepared + dry-run-verified but the actual `cargo publish` is gated on the owner's `cargo login` (surfaced to the user at the end — a credential, not a code matter) |
| 4 | Fixtures | Cross-language fixtures UNCHANGED (canary stability); new constructs covered by parser unit tests |

## 2. Python coverage (`parser/python.rs`)

1. **Tuple-unpacking for**: `for a, b in expr:` → `ForKind::ForEach { var: "a, b", iter }`.
   The var field is a display string; rendering `FOR EACH a, b IN expr` is honest
   about unpacking. (Currently the whole loop falls to Raw.)
2. **enumerate loops**: `for i, x in enumerate(e):` → `ForKind::ForEach { var: "i, x", iter: e }`.
   Documented trade-off: the "index from 0" semantics is not represented, but the loop
   body stays fully structured instead of the whole statement going Raw. `enumerate(e, start)` keeps
   the same treatment (start ignored).
   Single-var `for x in enumerate(e)` → ForEach { var: "x", iter: e } (same rule).
3. **Class method extraction**: `class C:` bodies — each `function_definition` inside
   becomes an `Item::Function` (mirrors Java's `collect_methods`). Other class-body
   statements (field assignments, docstrings, decorators) are skipped silently, matching
   Java's established behavior for class fields. This unlocks LeetCode-style Python
   (`class Solution:` with methods), the most common real-world shape. `__init__`
   renders like any other method.

### Non-goals (Python)

Comprehensions, dict/set literals as IR nodes (currently verbatim Raw exprs — acceptable),
async constructs, decorators as metadata, module-level non-function items (still Raw).

## 3. Go coverage (`parser/go.rs`)

1. **if-with-initializer**: `if x := g(); cond { … }` → `Block([VarDecl(x, g), If(cond, …)])` —
   the init is emitted as a preceding statement, then the If parses normally (replacing the
   whole-statement Raw fallback added in the Go final review). `if x, y := f(); cond` follows
   the multi-name assign rule. `if x = g(); cond` (plain assign init) → Assign then If.
2. **Two-var range**: `for i, v := range xs` → `ForKind::ForEach { var: "i, v", iter: xs }`
   (replacing the Raw fallback; consistent with Python's tuple-unpack rule).
3. **switch → if/else-if chains**:
   - Tagless: `switch { case c1: A; case c2: B; default: C }` →
     `If c1 A else If c2 B else C`.
   - Tagged: `switch x { case v1: A; default: B }` → `If x = v1 A else B` (Binary Eq).
   - Multiple expressions per case (`case a, b:`) → OR chain.
   - Raw (loud, whole statement): `fallthrough` present, type switches (`switch y := t.(type)`),
     switches with an init clause (`switch z := f(); z`), labeled cases with empty bodies are
     still fine (empty block).
   - `select` stays Raw.

### Non-goals (Go)

defer/go/goroutines/channels semantics, `goto`, generics rendering.

## 4. Renderer impact

None. All mappings land in existing IR variants (`ForEach` var is already a display
string; if/else-if chains already render). No new IR nodes.

## 5. Testing

- Python: unit tests — tuple-for shape, enumerate loop shape, class extraction
  (`class Solution:` + two methods → two `Item::Function`s, fields skipped silently,
  diag 0 for methods), existing class-Raw behavior test updated/removed accordingly.
- Go: unit tests — if-init emits VarDecl+If (diag 0), two-var range ForEach shape,
  tagless switch → else-if chain skeleton, tagged switch → Eq condition, fallthrough → Raw.
- CLI pinning: python `class Solution:` file renders both methods; go file with
  `if err := f(); err != nil` renders structured (decl + IF), no warning.
- Cross-language canary: untouched, must stay green (5×4 budgets unchanged).

## 6. Distribution (crates.io readiness)

- Package metadata on `algosketch-cli` (the publishable binary, name `algosketch`):
  `description` ("Turn real source code into language-neutral pseudocode and
  human-readable explanations"), `keywords` (["pseudocode", "cli", "tree-sitter",
  "algorithm", "code-review"] — max 5), `categories` (["command-line-utilities",
  "development-tools"]), `homepage`/`repository` (github), `readme` (README.md),
  `license-file` inheritance already correct (Apache-2.0 field + LICENSE-APACHE).
- `algosketch-core` gets matching metadata (it publishes as a dependency).
- `cargo publish --dry-run` clean for both crates (network metadata check included).
- Actual publish: NOT executed — requires `cargo login` with the owner's crates.io
  token. The final report surfaces the exact two commands for the user.
- README: add a one-line install section (`cargo install algosketch`) after publish
  is confirmed — added in this milestone behind the assumption the user runs the
  publish; wording notes "or build from source".

## 7. Version & docs

- 0.2.1 across manifests + lockfile; tag `v0.2.1` after merge.
- Main spec: §6/§7 unchanged (no CLI surface change); §10 add M7 row
  (`| M7 | Coverage + distribution | Python classes/enumerate/tuple-for; Go if-init/switch/two-var range; crates.io-ready metadata; v0.2.1. |`);
  completion line → M1–M7 / v0.2.1.
- README: status line → v0.2.1; the two lingering "v0.1 is intentionally not…" scope
  sentences reworded to version-neutral ("algosketch is intentionally not…").

## 8. Acceptance criteria

- [ ] `for i, x in enumerate(xs)` and `for k, v in pairs` render as structured FOR EACH loops.
- [ ] `class Solution:` python files render their methods.
- [ ] `if err := f(); err != nil` renders structured (init + IF) with zero warnings.
- [ ] Two-var `range` renders structured; switch renders as if/else-if chains; fallthrough/type-switch stay loud Raw.
- [ ] Cross-language canary green, budgets unchanged; full workspace suite green; fmt/clippy clean.
- [ ] `cargo publish --dry-run` succeeds for both crates.
- [ ] `algosketch --version` → 0.2.1; `v0.2.1` tagged after merge; publish commands surfaced to the owner.

## 9. Non-goals (milestone)

LLM provider (v0.3), WASM (v0.4), GitHub Release binaries/brew taps, comprehension IR,
ANSI colors, `select` mapping.
