# M5 Polish — Design Spec

- Date: 2026-09-04
- Status: Approved (design)
- Amends: [`2026-05-20-algosketch-design.md`](2026-05-20-algosketch-design.md)
  (main spec), milestone **M5 — Polish**.

M5 is the last milestone before v0.1.0. Much of its literal acceptance
criteria ("Markdown output, `Raw` fallback warnings, exit codes, full
`assert_cmd` suite pass") already landed during M1–M4: md/text output, `-q`,
exit codes 0–3, 19 assert_cmd tests, and the `<unparsed>` / explanation
fallback markers are all in place. This spec covers only the remaining gaps.

## 1. Decisions (locked)

| # | Question | Decision |
| - | -------- | -------- |
| 1 | Raw fallback warning detail | Full: filename + line numbers, per main spec §8 format |
| 2 | CI matrix | `ubuntu-latest` + `macos-latest`, stable toolchain |
| 3 | `--debug-ir` | **Not in M5.** Remove from main spec §11 open questions |
| 4 | v0.1.0 release shape | Bump version + `git tag v0.1.0` only; no crates.io, no GitHub Release binaries |
| 5 | `PSEUDOCODE_LANG` env var | Rename to `ALGOSKETCH_LANG` (project was renamed pseudocode → algosketch) |

## 2. Parse-time raw diagnostics

Today the CLI warns `warning: N unparsed nodes preserved as raw fallback
(items: X, statements: Y, expressions: Z)`, computed after parsing by walking
the module (`collect_raw_stats`). The spec format requires file name and line
numbers, which are not recoverable from the IR (`Raw(String)` carries no
span).

### Approach

Collect diagnostics **at parse time** (approach A from brainstorming):

- `RawDiagnostics` in `diagnostics.rs` gains line numbers:
  ```rust
  pub struct RawDiagnostics {
      pub items: usize,
      pub statements: usize,
      pub expressions: usize,
      pub lines: Vec<usize>,   // source line numbers, unsorted, may duplicate
  }
  ```
  plus `total()` and `sorted_unique_lines()` helpers.
- `LanguageParser::parse` changes signature:
  ```rust
  fn parse(&self, source: &str) -> Result<(Module, RawDiagnostics), PseudoError>;
  ```
  Each parser records `node.start_position().row + 1` at every site where it
  constructs an `Item::Raw` / `Stmt::Raw` / `Expr::Raw`, via a small shared
  helper in `parser/common.rs`.
- `collect_raw_stats` (the post-hoc module walk) is **deleted**; the CLI uses
  the returned `RawDiagnostics` directly. `tests/diagnostics.rs` is rewritten
  as parser-level tests.

### Rationale

- Keeps the IR unchanged — the main spec's "IR is the only stable interface"
  invariant holds, and `Raw(String)` stays as specified in §5.
- Line numbers come free from tree-sitter at exactly the point where the Raw
  node is born; no fragile post-hoc text search.
- `LanguageParser` is an internal trait with three implementations; the
  signature change is cheap and mechanical.

### Warning format (main spec §8, now fully implemented)

```text
warning: 3 unparsed nodes in input.cpp (lines 12, 45, 67)
```

- `<file>` is the input path as given; `<stdin>` when reading from stdin.
- Line numbers are sorted and deduplicated. If more than 5 distinct lines,
  show the first 5 then `+N more`:
  `warning: 9 unparsed nodes in main.java (lines 3, 4, 7, 12, 15, +4 more)`.
- Emitted to stderr unless `-q`. Never affects the exit code (§8: Raw
  fallback does not constitute failure).

## 3. CLI completion

Per main spec §7, both missing shortcut flags:

- `--pseudo-only` ≡ `--no-explain`
- `--explain-only` ≡ `--no-pseudo`

Conflict rules: `--pseudo-only` conflicts with `--no-pseudo` (and vice versa
for explain). A combination that disables both outputs (e.g. `--no-pseudo
--no-explain`) is a user error → exit 1 with a clear message.

### Environment variable rename

`PSEUDOCODE_LANG` → `ALGOSKETCH_LANG` in `detect_locale`, with no alias
retention (pre-1.0, no users to break). Main spec §7 is edited in the same
PR per the spec's own "material changes" rule.

## 4. CI

New `.github/workflows/ci.yml`:

- Triggers: `push` to `main`, `pull_request` to `main`.
- Matrix: `ubuntu-latest`, `macos-latest` × stable toolchain.
- Steps (each matrix job):
  1. `cargo fmt --check`
  2. `cargo clippy --workspace -- -D warnings`
  3. `cargo test --workspace`
  4. `cargo build --release`
- `Swatinem/rust-cache` (or equivalent) for dependency caching.

This is the exact command set from main spec §9.

## 5. Version bump & release housekeeping

- Workspace `version` 0.0.0 → 0.1.0 (single place: root `Cargo.toml`).
- README status line "Pre-alpha — scaffolding only" → v0.1.0 state; ensure
  the usage examples still match reality (env var rename).
- Main spec edits:
  - §7: `PSEUDOCODE_LANG` → `ALGOSKETCH_LANG`.
  - §10: mark M1–M5 complete.
  - §11: remove the `--debug-ir` open question (decided: not doing it).
- After merge to `main` and green CI: tag `v0.1.0` on the merge commit and
  push the tag.

## 6. Testing

Parser level (new, one per language):

- `RawDiagnostics` reports correct counts **and** 1-based line numbers for a
  source containing unparsed constructs (items / statements / expressions
  covered across the three parsers).

CLI level (assert_cmd, additions):

- Warning includes filename and sorted unique line numbers.
- Warning on stdin uses `<stdin>`.
- `--pseudo-only` / `--explain-only` behave like their `--no-*` equivalents.
- `--pseudo-only --no-pseudo` (and the explain analogue) → exit 1.
- `ALGOSKETCH_LANG=zh` selects Chinese output; `PSEUDOCODE_LANG` is no
  longer honored (negative test).
- Exit code 3 is not tested (internal-panic path).

Existing exit-code tests (0/1/2), `-q` suppression, md/text formatting, and
locale tests stay unchanged.

## 7. Non-goals for M5

- `--debug-ir` (removed from open questions — not building it).
- ANSI colors (deferred until users ask, per main spec §11).
- `insta` snapshot migration (current hand-written assertions stay).
- crates.io publish / GitHub Release artifacts / cargo-dist.
- Any v0.2 LLM-provider or v0.3 WASM work.

## 8. Acceptance criteria

M5 is done when:

- [ ] Warning matches §2 format with filename + line numbers on all three
      languages, and respects `-q`.
- [ ] `--pseudo-only` / `--explain-only` work; conflicting combinations exit 1.
- [ ] `ALGOSKETCH_LANG` works; `PSEUDOCODE_LANG` is ignored.
- [ ] CI workflow runs the four commands from §9 of the main spec on both
      platforms and is green.
- [ ] Workspace version is 0.1.0; README and main spec updated per §5.
- [ ] `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and
      `cargo test --workspace` all pass locally.
- [ ] v0.1.0 tag pushed after merge.
