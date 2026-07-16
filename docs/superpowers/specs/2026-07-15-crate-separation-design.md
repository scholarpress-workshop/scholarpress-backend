# Crate Separation: sp-extract / sp-check Boundary Cleanup

## Problem

`sp-check` (currently named `sp-validate`) carries a complete, independent PDF
extraction pipeline that duplicates `sp-extract`. It defines its own `Document`,
`Page`, and `TextSpan` types, its own word-span builder, and depends on
`pdf_oxide` directly — completely bypassing `sp-extract`. The `sp-extract`
dependency in `sp-check`'s `Cargo.toml` is unused.

The desired architecture is: `sp-extract` is the single door for all input
formats (PDF, DOCX, future). It produces one canonical `ParsedDocument` that
`sp-check` and `publish-service` consume without ever touching raw format
parsers.

## Design

### sp-extract: canonical document model

`ParsedPage` gains three fields to hold the low-level glyph and spatial data
that checkers need:

```rust
pub struct ParsedPage {
    pub page_number: usize,             // RENAMED from `number`
    pub text: String,
    pub width: f32,
    pub height: f32,
    pub spans: Vec<TextSpan>,           // NEW — bbox (top, bottom, x0, x1)
    pub images: Vec<(f32,f32,f32,f32)>, // NEW — bbox (top, bottom, x0, x1)
    pub paths: Vec<(f32,f32,f32,f32)>,  // NEW — bbox (top, bottom, x0, x1)
}
```

`number` is renamed to `page_number` to match `sp-validate`'s field name,
avoiding ~100+ reference changes across 33 checkers.

`TextSpan` keeps its current fields but `color` is populated from `pdf_oxide`'s
character color data (currently always `None`). Color is stored as RGB floats
in 0-1 range: `(r: f32, g: f32, b: f32)`, matching `pdf_oxide`'s native format.

New fields on `ParsedPage` (`spans`, `images`, `paths`) carry doc comments
documenting the bbox tuple convention `(top, bottom, x0, x1)` in page-space
coordinates (origin at top-left, Y increases downward).

Image and path extraction logic moves from `sp-check`'s extractor into
`sp-extract/src/pdf.rs`, populating the new fields.

Public API:
- `sp_extract::extract_pdf(bytes) -> ParsedDocument` — unchanged signature, richer result
- `sp_extract::extract_docx(bytes) -> ParsedDocument` — unchanged; new fields empty for DOCX

**Algorithmic parity:** The word-span merging logic in `sp-extract/src/pdf.rs`
(`build_spans`) and `sp-validate/src/extractor.rs` (`build_word`) share the same
approach (gap threshold 20pt, Y-delta 3pt) but differ in detail. Validate
parity by diffing `--dump-extract` JSON output on a known-good PDF before and
after migration. If output diverges, normalize to the sp-extract algorithm
before cutover.

### sp-check: pure validation consumer

**Deletions:**
- `src/extractor.rs` (125 lines) — PDF parsing moves to `sp-extract`
- `src/document.rs` (27 lines) — types come from `sp_extract::document`
- `Cargo.toml`: remove `pdf_oxide = "0.3"`

**Changes:**
- All checkers import `sp_extract::document::*` instead of `crate::document::*`
- `Checker` trait signature uses `&ParsedDocument` instead of `&Document`
- `engine::run_checks` reads file bytes, calls `sp_extract::extract_pdf`, passes
  result to checkers. Error type: `Box<dyn std::error::Error>` (same as current
  `run_checks` signature — both `sp_extract` and `std::fs::read` satisfy this trait)
- Test helpers construct `sp_extract::document::TextSpan`/`ParsedPage`/`ParsedDocument`.
  `TextSpan` is byte-identical across crates. `ParsedPage`'s field `number`
  is renamed to `page_number` to match the existing checker usage

### Rename: sp-validate → sp-check

| From | To |
|------|----|
| `crates/sp-validate/` | `crates/sp-check/` |
| crate name `sp-validate` / `sp_validate` | `sp-check` / `sp_check` |
| `POST /validate` | `POST /check` |
| `routes/validate.rs` | `routes/check.rs` |
| `ValidateRequest` | `CheckRequest` |
| `ValidationResult` | `CheckResponse` |
| `Violation` | `CheckViolation` |

The rest of the codebase already uses "check" terminology (`Checker`, `CheckResult`,
`CheckOptions`, `run_checks`, `checkers/` directory, `commands/check.rs`) — no
further renames needed.

### App layer

**publish-service:**
- `routes/extract.rs` — already calls `sp_extract::extract_pdf`. No change.
  JSON output gains `spans`/`images`/`paths` fields per page (additive).
- `routes/check.rs` (renamed from `routes/validate.rs`) — already calls
  `sp_validate::engine::run_checks`. Rename to `sp_check::engine::run_checks`.
  Engine internals change is invisible to caller.

**scholarpress-cli:**
- `commands/check.rs` `--dump-extract` flag — switches from
  `sp_validate::extractor::extract_document` to `sp_extract::extract_pdf`.
  JSON output schema changes (see Breaking changes).
- `check` command — `sp_check::engine::run_checks` call unchanged beyond
  rename.

Both apps need `sp-validate` → `sp-check` dep rename in Cargo.toml (same crate,
new name).

### Dependency graph (after)

```
sp-extract (pdf_oxide, quick-xml, zip)
    |
    +---- sp-check (serde_yaml, regex) — no pdf_oxide
    |
    +---- publish-service — imports all three lib crates
    |
    +---- scholarpress-cli — imports sp-extract + sp-check

sp-typst (serde_json) — standalone, unchanged
```

## Files affected

| File | Change |
|------|--------|
| `crates/sp-extract/src/document.rs` | Add fields to `ParsedPage` |
| `crates/sp-extract/src/pdf.rs` | Add image/path extraction, populate span `color` |
| `crates/sp-check/src/extractor.rs` | Delete |
| `crates/sp-check/src/document.rs` | Delete |
| `crates/sp-check/src/lib.rs` | Remove `pub mod document; pub mod extractor;` |
| `crates/sp-check/src/engine.rs` | Use `sp_extract::extract_pdf` instead of `crate::extractor` |
| `crates/sp-check/src/calibration.rs` | Rename `sp_validate::` → `sp_check::` imports (no logic change) |
| `crates/sp-check/src/checkers/*.rs` | Import from `sp_extract::document`, use `ParsedDocument` |
| `crates/sp-check/src/checkers/mod.rs` | Update `Checker` trait signature |
| `crates/sp-check/Cargo.toml` | Remove `pdf_oxide`; rename package name `sp-validate` → `sp-check` |
| `apps/scholarpress-cli/src/commands/check.rs` | `--dump-extract` uses `sp_extract` directly |
| `apps/publish-service/src/routes/check.rs` | Rename from `validate.rs` + update imports |
| `apps/publish-service/src/routes/mod.rs` | Update route path `/validate` → `/check` and module name |
| `apps/publish-service/src/error.rs` | `AppError::Validation` → `AppError::Check` (renamed variant) |
| `apps/publish-service/Cargo.toml` | Update dep `sp-validate = { path = "../../crates/sp-validate" }` → `sp-check = { path = "../../crates/sp-check" }` |
| `apps/scholarpress-cli/Cargo.toml` | Update dep `sp-validate = { path = "../../crates/sp-validate" }` → `sp-check = { path = "../../crates/sp-check" }` |
| `Cargo.toml` (workspace root) | Update member path `crates/sp-validate` → `crates/sp-check` |

## Non-goals

- No changes to `sp-typst`
- No changes to DOCX extraction
- No changes to institution specs or catalog data
- No new checker implementations

## Breaking changes

- `POST /validate` → `POST /check` — route path rename. Clients calling the
  validate endpoint must update. No backward-compatible alias.
- `--dump-extract` JSON schema changes from `sp-validate`'s `Document { pages }`
  to `sp-extract`'s `ParsedDocument { raw_text, pages, paragraphs, headings,
  metadata }`. This is a debug flag; no production impact.

## Verification

- All existing tests in `sp-check` must pass after migration
- Diff `--dump-extract` output on a known-good PDF before/after to validate
  algorithmic parity of word-span extraction
- `cargo build && cargo test` must succeed workspace-wide
