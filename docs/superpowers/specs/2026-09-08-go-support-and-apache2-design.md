# Go Language Support + Apache-2.0 Relicense — Design Spec

- Date: 2026-09-08
- Status: Approved (design)
- Baseline: v0.1.0 (main @ `a2f52cb`)
- Release vehicle: **v0.2.0**

## 1. Decisions (locked)

| # | Question | Decision |
| - | -------- | -------- |
| 1 | Scope | Full Go support mirroring the existing three-language pattern (tree-sitter adapter, not stub recognition) |
| 2 | License | **Apache-2.0 only**, effective v0.2.0 (sole-author relicense; v0.1.0 snapshots keep their granted dual-license rights forever — licenses already granted cannot be revoked) |
| 3 | Version | 0.2.0 (new language = minor bump); roadmap shifts: LLM provider → v0.3, WASM → v0.4 |
| 4 | Grammar | Pin whichever `tree-sitter-go` release is compatible with the workspace's `tree-sitter = "0.24"` runtime (try newest compatible; only bump the runtime if no compatible grammar exists — bumping the runtime is a separate decision requiring all four grammars re-verified) |

## 2. Go parser (`crates/algosketch-core/src/parser/go.rs`)

`GoParser` implements `LanguageParser` exactly like the existing adapters, including
parse-time `RawDiagnostics` (every `Raw` construction goes through
`record_raw_item/stmt/expr` from `parser/common.rs`; line = `node.start_position().row + 1`).

### Construct mapping (v0.2 subset)

| Go source | IR |
| --------- | -- |
| `func f(a T, b U) V { … }` (no receiver) | `Item::Function` |
| `func (r *T) f(…) { … }` (method) | `Item::Function`, receiver prepended to params as a normal parameter named `r` |
| `x := expr` (single name) | `Stmt::VarDecl` (it declares — mirrors Java/C++ decl lines) |
| `a, b := v1, v2` (multi-name) | `Stmt::Assign` with tuple target/value (same as Python's tuple assign; no current fixture uses it) |
| `x = expr` | `Stmt::Assign` |
| `a, b = b, a` (multi-assign) | `Stmt::Assign` with tuple target/value |
| `var x T`, `var x T = v`, `const x = v` | `Stmt::VarDecl` |
| `if c { } else if { } else { }` | `Stmt::If` chains |
| `for i := 0; i < n; i++ { }` | `ForKind::CStyle`; the update clause is recorded as a **Raw expr** exactly like the C++/Java adapters (their `j = j + 1`-style updates are Raw — this drives the per-fixture raw budget) |
| `for x := range xs { }`, `for _ := range xs { }` | `ForKind::ForEach` (var = the named one; `_` kept as-is) |
| `for i, v := range xs { }` (two vars) | **Raw** (matches Python's `enumerate` fallback; can improve later) |
| `for cond { }` | `Stmt::While` |
| `for { }` (infinite) | `Stmt::While` with `Literal::Bool(true)` |
| `return`, `return expr`, `return a, b` | `Stmt::Return(None / Some / Some(Tuple))` |
| `break` / `continue` | `Stmt::Break` / `Stmt::Continue` |
| operators `== != < <= > >= && \|\| !` etc. | `BinOp`/`UnOp` via existing `parse_*` helpers in common.rs |
| `nil`, `true`, `false`, int/string literals | `Literal` (`nil` maps to the same null literal Java/Python use) |
| `len(x)` | plain `Call` (renderer already normalizes to `LENGTH(x)`) |
| other calls, indexing `x[i]`, field `x.f`, type conversions `T(x)` | `Call`/`Index`/`Field` (conversions parse as calls; harmless) |

### Raw fallback (warning-visible, never crashes)

`go` statements, `defer`, `select`, `switch`/`type switch`, channel ops (`<-`, `ch <- v`),
`type … struct/interface` at top level, goroutine spawns, labeled statements, `range` over
int/channel, any unrecognized node.

### Top-level handling

- `package` and `import` declarations: **skipped silently** (Go files always have them;
  warning on every file would be noise — matches Java's silent handling of non-methods).
- `func`/`func (r …)` → `Item::Function` (collected).
- Everything else (`type`, top-level `var`/`const`, …) → `Item::Raw` with diagnostics.
  (Top-level `var`/`const` could map to `Item::GlobalVar`, but no fixture needs it and no
  renderer surfaces it differently — YAGNI; leave Raw.)

## 3. Wiring changes

- `SourceLang::Go` + `from_extension("go")`.
- CLI `CliLang::Go` → `--source-lang go`; `.go` auto-detected; add the `SourceLang::Go`
  dispatch arm in `run()`'s parse match and the `GoParser` import in `main.rs`.
- `parser/mod.rs` exports `GoParser`; no trait changes.
- Renderers: zero changes (IR is the interface — this is the payoff of the M1–M5 architecture).

## 4. Testing

- Go fixtures for the 5 canonical algorithms (`binary_search`, `reverse_string`,
  `reverse_linked_list`, `quick_sort`, `two_sum`) in `crates/algosketch-core/tests/fixtures/*.go`,
  structurally aligned with the existing fixtures (same function names/snake_case, same
  skeleton) so cross-language equality holds. **Skeleton-tag discipline**: lines that are
  `decl` in the other languages are written `var x T = v` (or single-name `x := v`) in Go;
  lines that are `assign` use `=` or multi-name `a, b := v1, v2`. (The skeleton test
  distinguishes `assign` vs `decl` tags.)
- `tests/cross_language.rs`: `parse_fixture` gains the `"go"` arm; budget assertions gain
  `expected_raw_total(algorithm, "go")` entries (empirically pinned per fixture —
  `reverse_linked_list` will carry ≥1 Raw for the `type` decl if the fixture needs one;
  prefer fixtures that need none).
- Parser-level diagnostics test: `go_reports_raw_statement_line` (a `defer` produces a
  Raw statement with the right line), plus a Raw-expr case pinning the for-update budget
  semantics (e.g. a CStyle loop whose `i++` update records one Raw expression).
- CLI tests: `.go` file auto-detection, `--source-lang go` via stdin, go fixture
  pseudocode + explanation output.

## 5. License switch (v0.2.0, sole author)

- Delete `LICENSE-MIT`.
- Root `Cargo.toml` `[workspace.package] license = "Apache-2.0"`.
- README (EN + 中文 license sections): dual → Apache-2.0 only, keep `LICENSE-APACHE` link.
- Main design spec header line: `License: Apache-2.0`.
- No NOTICE file (optional under Apache-2.0; YAGNI for a source-only tool).
- Historical docs (M5 spec/plan, older plans) keep their original wording — they describe
  the state at their time of writing.

## 6. Version & roadmap

- Workspace version → `0.2.0` (root + cli path-dep constraint + Cargo.lock), tag `v0.2.0`
  after merge.
- Main spec §10/§11 roadmap lines updated: v0.3 = LLM provider for Raw fallback,
  v0.4 = WASM + web UI. Language-count statements (§1, §2, §7 extension table, §9 fixture
  count "15 samples" → 20) updated to include Go.

## 7. Acceptance criteria

- [ ] `algosketch foo.go` renders pseudocode + explanation; `--source-lang go` works for stdin.
- [ ] Cross-language skeleton test passes for all 5 algorithms × 4 languages.
- [ ] Go Raw diagnostics report correct lines; `defer`/`switch` fall back with warnings.
- [ ] `cargo fmt/clippy -D warnings/test --workspace` green; CI green on both platforms.
- [ ] License switched: `grep -n "LICENSE-MIT\|MIT —" README.md` → empty; root
      `Cargo.toml` `license = "Apache-2.0"`; `LICENSE-MIT` deleted. (Note: README's
      "MIT Press" bibliography citations stay — they are not license references.)
- [ ] `algosketch --version` → `0.2.0`; `v0.2.0` tagged after merge.

## 8. Non-goals

- Two-variable `range` (Raw for now), generics rendering nuances, Go modules awareness,
  switch/type-switch mapping, channels/goroutines semantics.
- LLM provider, WASM, ANSI colors (deferred until asked, per main spec §11).
