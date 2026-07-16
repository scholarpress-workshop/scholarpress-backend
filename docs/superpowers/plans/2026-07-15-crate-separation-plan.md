# Crate Separation: sp-extract / sp-check Boundary Cleanup — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete the duplicate PDF extraction pipeline in sp-validate, make sp-extract the single document model, and rename sp-validate to sp-check.

**Architecture:** sp-extract gains low-level page data (spans, images, paths, color); sp-validate loses its extractor.rs, document.rs, and pdf_oxide dep, consuming sp_extract::document types directly; the crate is renamed sp-check and publish-service types follow.

**Tech Stack:** Rust 1.88+, pdf_oxide 0.3, axum 0.7, clap 4

## Global Constraints

- `cargo build && cargo test` must pass workspace-wide after every task
- All existing checkers must produce identical results after migration
- Diff `--dump-extract` output on a known-good PDF before/after for algorithmic parity
- No changes to sp-typst, DOCX extraction, institution specs, or catalog data

---

### Task 1: Update sp-extract document model

**Files:**
- Modify: `crates/sp-extract/src/document.rs:14-20`
- Modify: `crates/sp-extract/src/pdf.rs:34-39`

**Interfaces:**
- Produces: `ParsedPage` with renamed field `page_number` (was `number`), new fields `spans`, `images`, `paths`
- Consumes: nothing new

- [ ] **Step 1: Add doc comments and new fields to ParsedPage, rename `number` → `page_number`**

Replace the `ParsedPage` struct in `crates/sp-extract/src/document.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ParsedPage {
    pub page_number: usize,
    pub text: String,
    pub width: f32,
    pub height: f32,
    /// Word-level glyph spans on this page. Bbox tuple is (top, bottom, x0, x1)
    /// in page-space coordinates (origin at top-left, Y increases downward).
    pub spans: Vec<TextSpan>,
    /// Image bounding boxes as (top, bottom, x0, x1) in page-space coordinates.
    pub images: Vec<(f32, f32, f32, f32)>,
    /// Path/vector bounding boxes as (top, bottom, x0, x1) in page-space coordinates.
    pub paths: Vec<(f32, f32, f32, f32)>,
}
```

- [ ] **Step 2: Update `sp-extract/src/pdf.rs` to use `page_number` and include new fields**

Replace the `ParsedPage` construction in `extract_pdf` (line 34-39):

```rust
        pages.push(ParsedPage {
            page_number: page_idx + 1,
            text: page_text,
            width,
            height,
            spans,
            images: Vec::new(),
            paths: Vec::new(),
        });
```

- [ ] **Step 3: Update `sp-extract/src/docx.rs` to use `page_number` and include new fields**

Replace the `ParsedPage` construction in `extract_docx` (line 57-62):

```rust
        pages: vec![ParsedPage {
            page_number: 1,
            text: raw_text.clone(),
            width: 612.0,
            height: 792.0,
            spans: Vec::new(),
            images: Vec::new(),
            paths: Vec::new(),
        }],
```

- [ ] **Step 4: Run sp-extract tests**

```bash
rtk cargo test -p sp-extract
```

Expected: all 3 `median` tests pass.

- [ ] **Step 5: Commit**

```bash
rtk git add crates/sp-extract/src/document.rs crates/sp-extract/src/pdf.rs crates/sp-extract/src/docx.rs
rtk git commit -m "feat(sp-extract): rename number->page_number, add spans/images/paths fields to ParsedPage"
```

---

### Task 2: Port image/path extraction + color to sp-extract

**Files:**
- Modify: `crates/sp-extract/src/pdf.rs`

**Interfaces:**
- Consumes: `ParsedPage` with new fields from Task 1
- Produces: populated `ParsedPage.images`, `ParsedPage.paths`, `TextSpan.color`

- [ ] **Step 1: Add image and path extraction to `extract_pdf`**

In `crates/sp-extract/src/pdf.rs`, inside the `for page_idx in 0..page_count` loop, after `let spans = build_spans(&chars, height);` (line 19), add image and path extraction:

```rust
        let chars: Vec<TextChar> = doc.extract_chars(page_idx)?;
        let spans = build_spans(&chars, height);

        let images: Vec<(f32, f32, f32, f32)> = match doc.extract_images(page_idx) {
            Ok(imgs) => imgs
                .iter()
                .filter_map(|img| {
                    let bbox = img.bbox()?;
                    let img_top = height - (bbox.y + bbox.height);
                    let img_bottom = height - bbox.y;
                    let img_x0 = bbox.x;
                    let img_x1 = bbox.x + bbox.width;
                    Some((img_top.max(0.0), img_bottom, img_x0, img_x1))
                })
                .collect(),
            Err(_) => Vec::new(),
        };

        let paths: Vec<(f32, f32, f32, f32)> = match doc.extract_paths(page_idx) {
            Ok(ps) => ps
                .iter()
                .map(|p| {
                    let path_top = height - (p.bbox.y + p.bbox.height);
                    let path_bottom = height - p.bbox.y;
                    let path_x0 = p.bbox.x;
                    let path_x1 = p.bbox.x + p.bbox.width;
                    (path_top.max(0.0), path_bottom, path_x0, path_x1)
                })
                .collect(),
            Err(_) => Vec::new(),
        };
```

- [ ] **Step 2: Update ParsedPage construction to include images and paths**

Replace the `pages.push(ParsedPage { ... })` block from Task 1 step 2:

```rust
        pages.push(ParsedPage {
            page_number: page_idx + 1,
            text: page_text,
            width,
            height,
            spans,
            images,
            paths,
        });
```

- [ ] **Step 3: Populate color in `build_word_span`**

Replace `color: None,` (line 132) in the `build_word_span` function body:

```rust
        color: Some((first.color.r, first.color.g, first.color.b)),
```

- [ ] **Step 4: Build and test sp-extract**

```bash
rtk cargo build -p sp-extract
rtk cargo test -p sp-extract
```

Expected: build succeeds, all tests pass.

- [ ] **Step 5: Commit**

```bash
rtk git add crates/sp-extract/src/pdf.rs
rtk git commit -m "feat(sp-extract): add image/path extraction and populate TextSpan color"
```

---

### Task 3: Rewrite sp-validate engine to use sp_extract

**Files:**
- Modify: `crates/sp-validate/src/engine.rs`
- Delete: `crates/sp-validate/src/extractor.rs`
- Delete: `crates/sp-validate/src/document.rs`
- Modify: `crates/sp-validate/src/lib.rs`
- Modify: `crates/sp-validate/Cargo.toml`

**Interfaces:**
- Consumes: `sp_extract::extract_pdf()` (workspace build after Task 2)
- Produces: `run_checks` returns `Result<Vec<CheckResult>, Box<dyn std::error::Error>>` (unchanged)

- [ ] **Step 1: Rewrite `engine.rs` to use `sp_extract::extract_pdf`**

Replace the full content of `crates/sp-validate/src/engine.rs`:

```rust
use crate::checkers::{get_checker, CheckResult, Status};
use crate::spec::InstitutionSpec;
use std::path::Path;
use sp_extract::document::ParsedDocument;

#[derive(Default)]
pub struct CheckOptions {
    pub check_id: Option<String>,
    pub category: Option<String>,
}

pub fn run_checks(
    spec: &InstitutionSpec,
    pdf_path: &Path,
    options: &CheckOptions,
) -> Result<Vec<CheckResult>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(pdf_path)?;
    let doc: ParsedDocument = sp_extract::extract_pdf(&bytes)?;
    let mut results: Vec<CheckResult> = Vec::new();

    for check_def in &spec.checks {
        if let Some(ref filter_id) = options.check_id {
            if check_def.id != *filter_id {
                continue;
            }
        }
        if let Some(ref filter_cat) = options.category {
            if check_def.category != *filter_cat {
                continue;
            }
        }

        if !check_def.automatable {
            results.push(CheckResult {
                check_id: check_def.id.clone(),
                status: Status::Manual,
                evidence: vec![],
                detail: check_def
                    .review_hint
                    .clone()
                    .unwrap_or_else(|| "Manual review required".to_string()),
            });
            continue;
        }

        match get_checker(&check_def.category, &check_def.checker) {
            Some(checker) => {
                let params = serde_yaml::to_value(&check_def.params).unwrap_or_default();
                let mut result = checker.check(&doc, &params);
                result.check_id = check_def.id.clone();
                results.push(result);
            }
            None => {
                results.push(CheckResult {
                    check_id: check_def.id.clone(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: format!(
                        "No checker registered for {}/{}",
                        check_def.category, check_def.checker
                    ),
                });
            }
        }
    }

    Ok(results)
}
```

- [ ] **Step 2: Delete `extractor.rs`**

```bash
rtk git rm crates/sp-validate/src/extractor.rs
```

- [ ] **Step 3: Delete `document.rs`**

```bash
rtk git rm crates/sp-validate/src/document.rs
```

- [ ] **Step 4: Update `lib.rs` to remove deleted modules**

Replace the full content of `crates/sp-validate/src/lib.rs`:

```rust
pub mod calibration;
pub mod checkers;
pub mod engine;
pub mod report;
pub mod spec;
```

- [ ] **Step 5: Remove `pdf_oxide` from sp-validate Cargo.toml**

Remove the line `pdf_oxide = "0.3"` from `crates/sp-validate/Cargo.toml`, leaving:

```toml
[package]
name = "sp-validate"
version = "0.1.0"
edition = "2021"

[dependencies]
sp-extract = { path = "../sp-extract" }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
regex = "1"
```

- [ ] **Step 6: Build sp-validate (will fail — checker trait still uses old type)**

```bash
rtk cargo build -p sp-validate
```

Expected: compile errors in checkers because they still reference `crate::document::Document`. This is expected — fixed in Task 4.

- [ ] **Step 7: Commit**

```bash
rtk git add crates/sp-validate/src/engine.rs crates/sp-validate/src/lib.rs crates/sp-validate/Cargo.toml
rtk git commit -m "refactor(sp-validate): switch engine to sp_extract, remove pdf_oxide dep"
```

---

### Task 4: Update all checkers to use sp_extract::document types

**Files:**
- Modify: `crates/sp-validate/src/checkers/mod.rs`
- Modify: `crates/sp-validate/src/checkers/typography.rs`
- Modify: `crates/sp-validate/src/checkers/layout.rs`
- Modify: `crates/sp-validate/src/checkers/structure.rs`
- Modify: `crates/sp-validate/src/checkers/footnotes.rs`
- Modify: `crates/sp-validate/src/checkers/toc_details.rs`
- Modify: `crates/sp-validate/src/checkers/content.rs`
- Modify: `crates/sp-validate/src/checkers/optional_pages.rs`
- Modify: `crates/sp-validate/src/checkers/sections.rs`
- Modify: `crates/sp-validate/src/checkers/title_page.rs`

**Interfaces:**
- Consumes: `sp_extract::document::{ParsedDocument, ParsedPage, TextSpan}`
- Produces: `Checker::check(&self, doc: &ParsedDocument, ...) -> CheckResult`

- [ ] **Step 1: Update the `Checker` trait and registry in `checkers/mod.rs`**

In `crates/sp-validate/src/checkers/mod.rs`, replace line 1:
```rust
use crate::document::Document;
```
with:
```rust
use sp_extract::document::ParsedDocument;
```

Replace the `Checker` trait signature (line 40-44):
```rust
pub trait Checker: Send + Sync {
    fn category(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn check(&self, doc: &ParsedDocument, params: &serde_yaml::Value) -> CheckResult;
}
```

- [ ] **Step 2: Update every checker file's main import**

In each checker file, replace `use crate::document::Document;` with:
```rust
use sp_extract::document::ParsedDocument as Document;
```

This keeps all `check(&self, doc: &Document, ...)` signatures and `&Document` references working without body changes.

- [ ] **Step 3: Replace inline `crate::document::TextSpan` references**

Checker body code has inline path references like `&crate::document::TextSpan` (e.g., `layout.rs:47,51,140`). In all checker files, replace:

```
crate::document::TextSpan
```
with:
```
sp_extract::document::TextSpan
```

```bash
for f in crates/sp-validate/src/checkers/*.rs; do
    sed -i 's/crate::document::TextSpan/sp_extract::document::TextSpan/g' "$f"
done
```

- [ ] **Step 4: Update test module imports in all checker files**

In each checker file, replace the test-module `use crate::document::{Document, Page, TextSpan};` with `use sp_extract::document::{ParsedDocument, ParsedPage, TextSpan};`.

In test code within those modules:
- Replace `Document { pages: vec![Page { ... }] }` → `ParsedDocument { pages: vec![ParsedPage { ... }], raw_text: String::new(), paragraphs: vec![], headings: vec![], metadata: sp_extract::document::ParsedMetadata { title: None, author: None, page_count: 1, page_count_estimated: false, detected_fonts: vec![] } }`
- Replace `Page { page_number: ... }` → `ParsedPage { page_number: ..., text: String::new(), spans: ..., images: vec![], paths: vec![] }`
- `TextSpan` stays the same (same name in both crates)

Files with test modules to update:
- `crates/sp-validate/src/checkers/typography.rs:681` — `use crate::document::{Document, Page, TextSpan};`
- `crates/sp-validate/src/checkers/layout.rs:378` — `use crate::document::{Document, Page, TextSpan};`
- `crates/sp-validate/src/checkers/structure.rs:833` — `use crate::document::{Document, Page, TextSpan};`
- `crates/sp-validate/src/checkers/footnotes.rs:158` — `use crate::document::{Document, Page};`
- `crates/sp-validate/src/checkers/toc_details.rs:329` — `use crate::document::{Document, Page};`
- `crates/sp-validate/src/checkers/content.rs:578` — `use crate::document::{Document, Page, TextSpan};`
- `crates/sp-validate/src/checkers/optional_pages.rs:140` — `use crate::document::{Document, Page};`
- `crates/sp-validate/src/checkers/sections.rs:895` — `use crate::document::{Document, Page};`
- `crates/sp-validate/src/checkers/title_page.rs:470` — `use crate::document::{Document, Page};`

- [ ] **Step 5: Update test helper functions that construct `Page`/`Document`**

Each checker's test module has helper functions (e.g., `make_doc`, `make_page`, `build_doc`) that construct `Document`/`Page` directly. Update each to use `ParsedDocument`/`ParsedPage`.

Example for `typography.rs` test helpers (lines 678-717):

```rust
    fn make_span(text: &str, font_size: f32, font_name: &str, bbox: (f32, f32, f32, f32), is_bold: bool, is_italic: bool) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            font_name: font_name.to_string(),
            font_size,
            bbox,
            is_bold,
            is_italic,
            color: None,
        }
    }

    fn make_page(spans: Vec<TextSpan>) -> ParsedPage {
        ParsedPage {
            page_number: 1,
            text: String::new(),
            width: 612.0,
            height: 792.0,
            spans,
            images: vec![],
            paths: vec![],
        }
    }

    fn make_doc(spans: Vec<TextSpan>) -> ParsedDocument {
        ParsedDocument {
            raw_text: String::new(),
            pages: vec![make_page(spans)],
            paragraphs: vec![],
            headings: vec![],
            metadata: sp_extract::document::ParsedMetadata {
                title: None,
                author: None,
                page_count: 1,
                page_count_estimated: false,
                detected_fonts: vec![],
            },
        }
    }
```

Apply the same pattern to `layout.rs` (`build_doc`, `make_span`), `structure.rs`, `content.rs`, `sections.rs`, etc. Each `Page { page_number, width, height, spans, images, paths }` → `ParsedPage { page_number, text: String::new(), width, height, spans, images, paths }`. Each `Document { pages }` → `ParsedDocument { raw_text: String::new(), pages, paragraphs: vec![], headings: vec![], metadata }`.

- [ ] **Step 6: Checker body code — `&Document` → `&ParsedDocument` is handled by the `as Document` alias**

In each checker's `impl Checker` block, change `doc: &Document` → `doc: &ParsedDocument`. Since the checker files previously did `use crate::document::Document;`, the body references `Document` — add `use sp_extract::document::ParsedDocument as Document;` as a simple alias in each file, OR do a blanket replacement of `&Document` → `&ParsedDocument` in checker files. The simpler approach: keep the type alias at the top of each checker file.

No changes needed in fn signatures — the `use sp_extract::document::ParsedDocument as Document;` alias means `fn check(&self, doc: &Document, ...)` already resolves to `&ParsedDocument`, matching the trait from Step 1.

- [ ] **Step 7: Build sp-validate**

```bash
rtk cargo build -p sp-validate
```

Expected: compiles without errors. Fix any remaining `Document`/`Page` references.

- [ ] **Step 8: Run sp-validate tests**

```bash
rtk cargo test -p sp-validate
```

Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
rtk git add crates/sp-validate/src/checkers/
rtk git commit -m "refactor(sp-validate): switch checkers to sp_extract::document types"
```

---

### Task 5: Rename sp-validate crate to sp-check

**Files:**
- Rename: `crates/sp-validate/` → `crates/sp-check/`
- Modify: `crates/sp-check/Cargo.toml`
- Modify: All `.rs` files in `crates/sp-check/src/` (replace `sp_validate` → `sp_check`)
- Modify: `Cargo.toml` (workspace root) — member paths are glob, no change needed

**Interfaces:**
- Produces: crate `sp-check` replaces `sp-validate`
- Consumes: Task 4's working build

- [ ] **Step 1: Rename the crate directory**

```bash
rtk git mv crates/sp-validate crates/sp-check
```

- [ ] **Step 2: Update crate name in Cargo.toml**

In `crates/sp-check/Cargo.toml`, change:
```toml
name = "sp-validate"
```
to:
```toml
name = "sp-check"
```

- [ ] **Step 3: Replace all `sp_validate::` → `sp_check::` in source files**

```bash
for f in crates/sp-check/src/**/*.rs; do
    sed -i 's/sp_validate::/sp_check::/g' "$f"
done
```

(This affects `calibration.rs`, `checkers/mod.rs`, `check.rs` in CLI, `validate.rs` in publish-service — those are in subsequent tasks. For now, only sp-check internal files matter.)

Also check for `"sp-validate"` string references — none expected in sp-check source.

- [ ] **Step 4: Verify sp-check builds**

```bash
rtk cargo build -p sp-check
```

Expected: compiles. If it fails, check for remaining `sp_validate` references.

- [ ] **Step 5: Verify sp-check tests pass**

```bash
rtk cargo test -p sp-check
```

- [ ] **Step 6: Commit**

```bash
rtk git add crates/sp-check/ crates/sp-validate/
rtk git commit -m "refactor: rename sp-validate crate to sp-check"
```

---

### Task 6: Update app-layer cargo deps and imports

**Files:**
- Modify: `apps/publish-service/Cargo.toml`
- Modify: `apps/publish-service/src/routes/mod.rs`
- Modify: `apps/publish-service/src/routes/validate.rs` → rename to `routes/check.rs`
- Modify: `apps/publish-service/src/error.rs`
- Modify: `apps/scholarpress-cli/Cargo.toml`
- Modify: `apps/scholarpress-cli/src/commands/check.rs`

**Interfaces:**
- Consumes: `sp-check` crate from Task 5
- Produces: working publish-service and CLI builds

- [ ] **Step 1: Update publish-service Cargo.toml**

In `apps/publish-service/Cargo.toml`, replace:
```toml
sp-validate = { path = "../../crates/sp-validate" }
```
with:
```toml
sp-check = { path = "../../crates/sp-check" }
```

- [ ] **Step 2: Rename route file**

```bash
rtk git mv apps/publish-service/src/routes/validate.rs apps/publish-service/src/routes/check.rs
```

- [ ] **Step 3: Update `routes/mod.rs`**

In `apps/publish-service/src/routes/mod.rs`, replace line 6:
```rust
pub mod validate;
```
with:
```rust
pub mod check;
```

Replace the route registration (lines 19-22):
```rust
        .route(
            "/validate",
            post(validate::handler).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
```
with:
```rust
        .route(
            "/check",
            post(check::handler).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
```

- [ ] **Step 4: Update `routes/check.rs`**

In `apps/publish-service/src/routes/check.rs` (renamed file), replace all `sp_validate::` with `sp_check::`.

Replace the type names:
- `ValidateRequest` → `CheckRequest`
- `ValidationResult` → `CheckResponse`
- `Violation` → `CheckViolation`

Update `AppError::Validation(...)` → `AppError::Check(...)` calls in this file.

- [ ] **Step 5: Update `error.rs`**

In `apps/publish-service/src/error.rs`, rename the variant:
```rust
    Validation(String),
```
→
```rust
    Check(String),
```

And the display impl:
```rust
            AppError::Validation(m) => write!(f, "Validation failed: {}", m),
```
→
```rust
            AppError::Check(m) => write!(f, "Check failed: {}", m),
```

- [ ] **Step 6: Update CLI Cargo.toml**

In `apps/scholarpress-cli/Cargo.toml`, replace:
```toml
sp-validate = { path = "../../crates/sp-validate" }
```
with:
```toml
sp-check = { path = "../../crates/sp-check" }
```

- [ ] **Step 7: Update CLI `commands/check.rs`**

In `apps/scholarpress-cli/src/commands/check.rs`, replace all `sp_validate::` with `sp_check::`.

Replace the `--dump-extract` call (lines 42-58):
```rust
    if args.dump_extract {
        match sp_extract::extract_pdf(&std::fs::read(&args.pdf).unwrap_or_else(|e| {
            eprintln!("Error reading PDF: {}", e);
            process::exit(2);
        })) {
            Ok(doc) => match serde_json::to_string_pretty(&doc) {
                Ok(output) => {
                    println!("{}", output);
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("Error serializing document: {}", e);
                    process::exit(2);
                }
            },
            Err(e) => {
                eprintln!("Error extracting document: {}", e);
                process::exit(2);
            }
        }
    }
```

- [ ] **Step 8: Build entire workspace**

```bash
rtk cargo build
```

Expected: compiles. Fix any remaining import errors.

- [ ] **Step 9: Commit**

```bash
rtk git add apps/
rtk git commit -m "refactor: update app layer for sp-check rename"
```

---

### Task 7: Full verification

**Files:** (none new)

- [ ] **Step 1: Run workspace tests**

```bash
rtk cargo test
```

Expected: all tests pass.

- [ ] **Step 2: Algorithmic parity check — diff dump-extract output before migration**

```bash
# On a branch/revision before migration, capture dump-extract output for a known PDF
git stash
cargo run -p scholarpress-cli -- check --dump-extract /path/to/sample.pdf > /tmp/before.json
git stash pop
cargo run -p scholarpress-cli -- check --dump-extract /path/to/sample.pdf > /tmp/after.json
diff /tmp/before.json /tmp/after.json || echo "Diffs found — review carefully"
```

Expected: minor structural differences (new ParsedDocument wrapper vs old Document), but span-level data (text, font_size, bbox) should be equivalent for the same pages. If significant divergence, investigate the word-span algorithm in `build_spans` vs the old `build_word`.

- [ ] **Step 3: Verify no remaining references to old paths**

```bash
rtk grep "sp.validate\|sp_validate\|crates/sp-validate" --include="*.rs" --include="*.toml" -r .
```

Expected: zero matches.

- [ ] **Step 4: Final commit**

```bash
rtk git add -A
rtk git diff --staged
rtk git commit -m "verify: workspace builds and tests pass after crate separation"
```
