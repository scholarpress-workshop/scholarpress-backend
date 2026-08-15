# sp-annotate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `sp-annotate` crate that produces an annotated copy of a dissertation PDF from a `sp-check` report — a prepended summary page for document-level findings, and in-place yellow highlights + sticky notes for coordinate-tagged findings — wired into the CLI and `sp-mcp`.

**Architecture:** `sp-annotate` depends on `sp-check` (to deserialize the report) and `pdf_oxide` 0.3.73 (to write annotations and prepend a page). It exposes `parse_report`, `annotate_bytes`, and `annotate_file`. The CLI gains an `annotate` subcommand (consume a report JSON *or* run checks inline); `sp-mcp` gains an `annotate_pdf` tool that re-runs checks internally (its existing `check_pdf` drops bboxes, so it can't be reused).

**Tech Stack:** Rust (edition 2021), `pdf_oxide` 0.3, `sp-check`, `serde`/`serde_json`, `thiserror`, `clap` 4, `rmcp` 3.

**Spec:** `docs/superpowers/specs/2026-08-15-sp-annotate-design.md`

## Global Constraints

- `pdf_oxide` version pinned at `0.3` (matches `sp-extract`). Do not bump.
- Summary page text must be ASCII-only (no box-drawing `─` chars — the embedded standard-14 font won't render them).
- In-place findings (evidence with a `bbox`) are **not** duplicated into the summary page.
- PASS results never get in-place annotations; they appear only in the summary counts.
- All coordinates in `sp-check` evidence are `(top, bottom, x0, x1)` in top-left-origin page space and MUST be converted to PDF bottom-left space using the page height before creating annotations.
- Pre-push gate: `cargo fmt --all && cargo clippy --all --tests -- -D warnings`.

---

### Task 1: Make `sp-check` report types deserializable

The report JSON (`format_json`) uses uppercase status strings (`"PASS"`, `"FAIL"`, `"MANUAL"`, `"ERROR"`) and the field names `check_id`, `status`, `evidence`, `detail`, `results`, `summary`, `page`, `bbox`, `excerpt`, `pass`, `fail`, `manual`, `error`. We add `Deserialize` so `sp-annotate` can round-trip the JSON.

**Files:**
- Modify: `crates/sp-check/src/checkers/mod.rs` (imports + `Status`, `EvidenceItem`, `CheckResult`)
- Modify: `crates/sp-check/src/report.rs` (imports + `Summary`, `Report`, add round-trip test)

**Interfaces:**
- Consumes: nothing new.
- Produces: `Status`, `EvidenceItem`, `CheckResult`, `Summary`, `Report` all implement `serde::Deserialize` (and keep existing `Serialize`).

- [ ] **Step 1: Add `Deserialize` to the check types**

In `crates/sp-check/src/checkers/mod.rs`, change the serde import:

```rust
use serde::{Deserialize, Serialize};
```

Change the three derives:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Pass,
    Fail,
    Manual,
    Error,
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub page: usize,
    pub bbox: Option<(f32, f32, f32, f32)>,
    pub excerpt: Option<String>,
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_id: String,
    pub status: Status,
    pub evidence: Vec<EvidenceItem>,
    pub detail: String,
}
```

Note: `#[serde(rename_all = "UPPERCASE")]` also changes `Status`'s derived `Serialize` output from `"Pass"` to `"PASS"`. This is safe — `format_json` and the MCP use `Status::as_str()`, not the derived serialization, and no test asserts the PascalCase form.

- [ ] **Step 2: Add `Deserialize` to the report types**

In `crates/sp-check/src/report.rs`, change the import:

```rust
use serde::{Deserialize, Serialize};
```

Change the two derives:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Summary {
    pub pass: usize,
    pub fail: usize,
    pub manual: usize,
    pub error: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub results: Vec<CheckResult>,
    pub summary: Summary,
}
```

- [ ] **Step 3: Add a round-trip test**

Append to the existing `mod tests` in `crates/sp-check/src/report.rs` (inside `crate::checkers` already imported there):

```rust
#[test]
fn test_report_json_roundtrip() {
    let results = vec![
        make_result(Status::Fail),
        make_result(Status::Pass),
        CheckResult {
            check_id: "global_margins".to_string(),
            status: Status::Error,
            evidence: vec![EvidenceItem {
                page: 4,
                bbox: Some((10.0, 20.0, 30.0, 40.0)),
                excerpt: Some("1 inch".to_string()),
            }],
            detail: "margins too small".to_string(),
        },
    ];
    let report = build_report(results);
    let json = format_json(&report).expect("json");
    let parsed: Report = serde_json::from_str(&json).expect("parse back");
    assert_eq!(parsed.results.len(), 3);
    assert_eq!(parsed.results[0].status, Status::Fail);
    assert_eq!(parsed.results[2].evidence[0].bbox, Some((10.0, 20.0, 30.0, 40.0)));
    assert_eq!(parsed.summary.fail, 1);
}
```

- [ ] **Step 4: Run sp-check tests**

Run: `cargo test -p sp-check`
Expected: all tests PASS (existing + the new round-trip test).

- [ ] **Step 5: Commit**

```bash
git add crates/sp-check/src/checkers/mod.rs crates/sp-check/src/report.rs
git commit -m "feat(sp-check): derive Deserialize for report types"
```

---

### Task 2: Create `sp-annotate` crate with pure helpers

Create the crate skeleton and the pure, unit-testable logic (coordinate conversion, sticky-note text, summary-page text). No PDF I/O yet.

**Files:**
- Create: `crates/sp-annotate/Cargo.toml`
- Create: `crates/sp-annotate/src/lib.rs`
- Create: `crates/sp-annotate/src/annotate.rs`

**Interfaces:**
- Consumes: `sp_check::checkers::{CheckResult, EvidenceItem, Status}`, `sp_check::report::{Report, build_report}`, `pdf_oxide::geometry::Rect`.
- Produces (used by Task 3):
  - `pub(crate) fn bbox_to_rect(bbox: (f32, f32, f32, f32), page_height: f32) -> Rect`
  - `pub(crate) fn note_contents(check_id: &str, status: &str, detail: &str, excerpt: Option<&str>) -> String`
  - `pub(crate) fn summary_text(report: &Report) -> String`

- [ ] **Step 1: Write the crate manifest**

`crates/sp-annotate/Cargo.toml`:

```toml
[package]
name = "sp-annotate"
version = "0.1.0"
edition = "2021"

[dependencies]
pdf_oxide = "0.3"
sp-check = { path = "../sp-check" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
```

- [ ] **Step 2: Write the error type and public API shell**

`crates/sp-annotate/src/lib.rs`:

```rust
pub mod annotate;

use pdf_oxide::geometry::Rect;
use sp_check::report::Report;

#[derive(Debug, thiserror::Error)]
pub enum AnnotateError {
    #[error("failed to parse report JSON: {0}")]
    Report(#[from] serde_json::Error),

    #[error("pdf error: {0}")]
    Pdf(#[from] pdf_oxide::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Parse a `sp-check --json` report back into a `Report`.
pub fn parse_report(json: &str) -> Result<Report, AnnotateError> {
    Ok(serde_json::from_str(json)?)
}

/// Annotate a PDF (given as bytes) and return the annotated copy as bytes.
pub fn annotate_bytes(input: &[u8], report: &Report) -> Result<Vec<u8>, AnnotateError> {
    annotate::annotate_bytes(input, report)
}

/// Annotate `input` and write the result to `output`.
pub fn annotate_file(
    input: &std::path::Path,
    output: &std::path::Path,
    report: &Report,
) -> Result<(), AnnotateError> {
    let bytes = std::fs::read(input)?;
    let annotated = annotate_bytes(&bytes, report)?;
    std::fs::write(output, annotated)?;
    Ok(())
}
```

The `annotate::annotate_bytes` body is added in Task 3; for now it can be a stub. But the module must compile. Add a minimal placeholder in `annotate.rs` that returns `Ok(Vec::new())` and delete it in Task 3 (the unit tests in this task exercise only the pure helpers).

- [ ] **Step 3: Write the pure helpers and unit tests**

`crates/sp-annotate/src/annotate.rs`:

```rust
use pdf_oxide::geometry::Rect;
use sp_check::checkers::Status;
use sp_check::report::Report;

/// Convert a top-left-origin bbox `(top, bottom, x0, x1)` to a PDF-space
/// `Rect` (bottom-left origin) given the page height in points.
pub(crate) fn bbox_to_rect(bbox: (f32, f32, f32, f32), page_height: f32) -> Rect {
    let (top, bottom, x0, x1) = bbox;
    Rect::new(x0, page_height - bottom, x1 - x0, bottom - top)
}

/// Build the sticky-note body for an in-place finding.
pub(crate) fn note_contents(
    check_id: &str,
    status: &str,
    detail: &str,
    excerpt: Option<&str>,
) -> String {
    let mut s = format!("[{check_id}] {status}\n{detail}");
    if let Some(ex) = excerpt {
        s.push_str(&format!("\n\u{201c}{ex}\u{201d}"));
    }
    s
}

/// Build the ASCII text rendered on the prepended summary page:
/// counts plus document-level (non-bbox) findings only.
pub(crate) fn summary_text(report: &Report) -> String {
    let mut lines = vec![
        "ScholarPress Format Check - Annotated Summary".to_string(),
        "=".repeat(60),
        format!(
            "Pass: {}  Fail: {}  Manual: {}  Error: {}",
            report.summary.pass, report.summary.fail, report.summary.manual, report.summary.error
        ),
        String::new(),
        "Document-level findings".to_string(),
        "-".repeat(60),
    ];

    let mut count = 0;
    for r in &report.results {
        if r.status == Status::Pass {
            continue;
        }
        let doc_evidence: Vec<_> = r.evidence.iter().filter(|e| e.bbox.is_none()).collect();
        if doc_evidence.is_empty() {
            continue;
        }
        count += 1;
        lines.push(format!("[{}] {}", r.status.as_str(), r.check_id));
        if !r.detail.is_empty() {
            lines.push(format!("    {}", r.detail));
        }
        for e in &doc_evidence {
            let ex = e.excerpt.as_deref().unwrap_or("");
            lines.push(format!("    page {}  {}", e.page, ex));
        }
    }
    if count == 0 {
        lines.push("(none - all findings are highlighted in place)".to_string());
    }
    lines.push(String::new());
    lines.push("In-place findings are highlighted and annotated on their pages.".to_string());
    lines.join("\n")
}

// Task 3 fills this in; stub so the crate compiles now.
pub(crate) fn annotate_bytes(_input: &[u8], _report: &Report) -> Result<Vec<u8>, crate::AnnotateError> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_check::checkers::{CheckResult, EvidenceItem, Status};
    use sp_check::report::build_report;

    fn sample_report() -> Report {
        build_report(vec![
            CheckResult {
                check_id: "test_inplace".to_string(),
                status: Status::Fail,
                detail: "margins too small".to_string(),
                evidence: vec![EvidenceItem {
                    page: 1,
                    bbox: Some((100.0, 120.0, 200.0, 300.0)),
                    excerpt: Some("1 inch".to_string()),
                }],
            },
            CheckResult {
                check_id: "test_global".to_string(),
                status: Status::Fail,
                detail: "missing section".to_string(),
                evidence: vec![EvidenceItem {
                    page: 3,
                    bbox: None,
                    excerpt: Some("Abstract".to_string()),
                }],
            },
            CheckResult {
                check_id: "test_pass".to_string(),
                status: Status::Pass,
                detail: "ok".to_string(),
                evidence: vec![],
            },
        ])
    }

    #[test]
    fn bbox_to_rect_flips_y() {
        let r = bbox_to_rect((100.0, 120.0, 200.0, 300.0), 792.0);
        assert_eq!((r.x, r.y, r.width, r.height), (200.0, 672.0, 100.0, 20.0));
    }

    #[test]
    fn note_contents_includes_fields() {
        let s = note_contents("global_margins", "FAIL", "margins too small", Some("1 inch"));
        assert!(s.contains("[global_margins] FAIL"));
        assert!(s.contains("margins too small"));
        assert!(s.contains("1 inch"));
    }

    #[test]
    fn summary_text_lists_document_level_only() {
        let text = summary_text(&sample_report());
        assert!(text.contains("test_global"));
        assert!(text.contains("missing section"));
        assert!(text.contains("Fail: 2"));
        assert!(text.contains("Pass: 1"));
        assert!(!text.contains("test_inplace"), "in-place finding must not be duplicated");
    }
}
```

- [ ] **Step 4: Add the crate to the workspace**

The workspace `Cargo.toml` uses `members = ["crates/*", "apps/*"]`, so `crates/sp-annotate` is picked up automatically. No edit needed. Verify by building.

- [ ] **Step 5: Run tests**

Run: `cargo test -p sp-annotate`
Expected: 3 tests PASS (plus the stub `annotate_bytes` compiling).

- [ ] **Step 6: Commit**

```bash
git add crates/sp-annotate
git commit -m "feat(sp-annotate): crate skeleton and pure helpers"
```

---

### Task 3: Implement `annotate_bytes` (prepend summary + write annotations)

Drive `pdf_oxide` to (1) build a one-page summary PDF, (2) merge the source PDF after it, (3) add highlight + sticky annotations for in-place findings, (4) save. Add an integration test.

**Files:**
- Modify: `crates/sp-annotate/src/annotate.rs` (replace the stub `annotate_bytes`, add integration test)

**Interfaces:**
- Consumes (from Task 2): `bbox_to_rect`, `note_contents`, `summary_text`, `crate::AnnotateError`.
- Produces: the real `annotate_bytes`, exposed via `lib.rs` as `annotate_bytes`/`annotate_file`.

pdf_oxide API facts (verified against 0.3.73 source):
- `pdf_oxide::editor::DocumentEditor::from_bytes(Vec<u8>) -> Result<Self>`
- `editor.merge_from_bytes(&[u8]) -> Result<usize>` — appends the merged document's pages.
- `editor.current_page_count() -> usize`, `editor.get_page_media_box(i) -> Result<[f32;4]>` (llx, lly, urx, ury).
- `editor.edit_page(i, |page: &mut PdfPage| { ...; Ok(()) }) -> Result<()>`; `PdfPage::add_annotation::<A: Into<WriteAnnotation>>(A)`.
- `editor.save_to_bytes(&mut self) -> Result<Vec<u8>>`.
- `pdf_oxide::writer::{DocumentBuilder, PageSize}`; `DocumentBuilder::new().page(PageSize::Letter)` returns a `FluentPageBuilder` with `.font(name, size)`, `.at(x, y)`, `.text(&str)`, `.newline()`, `.done()`; then `builder.build() -> Result<Vec<u8>>`.
- `pdf_oxide::writer::{TextMarkupAnnotation, TextAnnotation}`; `pdf_oxide::annotation_types::TextMarkupType`.
  - `TextMarkupAnnotation::from_rect(TextMarkupType::Highlight, rect).with_color(1.0,1.0,0.0).with_opacity(0.4).with_author("scholarpress")`
  - `TextAnnotation::new(rect, contents).with_author("scholarpress")`

- [ ] **Step 1: Write the failing integration test**

Append to `mod tests` in `crates/sp-annotate/src/annotate.rs`:

```rust
#[test]
fn annotate_bytes_prepends_summary_and_adds_annotations() {
    // Locate the IU baseline fixture, skipping if absent (matches sp-mcp pattern).
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = (0..6)
        .map(|i| {
            let mut p = manifest.clone();
            for _ in 0..=i {
                p.pop();
            }
            p.join("scholarpress-catalog")
        })
        .find(|p| p.is_dir());
    let baseline = match catalog {
        Some(c) => c.join("institutions/iu-indianapolis/tests/fixtures/baseline.pdf"),
        None => {
            eprintln!("SKIP: scholarpress-catalog not found");
            return;
        }
    };
    let input = match std::fs::read(&baseline) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("SKIP: baseline.pdf fixture not present (run bash compile.sh)");
            return;
        }
    };

    let original_pages = pdf_oxide::PdfDocument::from_bytes(input.clone())
        .expect("open input")
        .page_count()
        .expect("page count");

    let out = annotate_bytes(&input, &sample_report()).expect("annotate should succeed");
    assert!(out.starts_with(b"%PDF"));

    let doc = pdf_oxide::PdfDocument::from_bytes(out).expect("reopen output");
    assert_eq!(doc.page_count().expect("pages"), original_pages + 1);

    let first = doc.extract_text(0).expect("summary text");
    assert!(first.contains("Annotated Summary"), "page 0 should be the summary page");

    // In-place finding on page 1 (1-based) lives at output index 1.
    let annots = doc.get_annotations(1).expect("annotations on page 1");
    assert!(!annots.is_empty(), "expected annotations on page 1");
    assert!(
        annots.iter().any(|a| matches!(a.subtype_enum, pdf_oxide::annotation_types::AnnotationSubtype::Highlight)),
        "expected a highlight annotation"
    );
}
```

The `sample_report()` helper already exists in the Task 2 test module; ensure the test uses it. `pdf_oxide::annotation_types::AnnotationSubtype` is in scope via the full path.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sp-annotate annotate_bytes_prepends_summary`
Expected: FAIL — `annotate_bytes` is still the `Ok(Vec::new())` stub, so the reopened "PDF" fails.

- [ ] **Step 3: Implement `annotate_bytes`**

Replace the stub in `crates/sp-annotate/src/annotate.rs`:

```rust
use pdf_oxide::annotation_types::TextMarkupType;
use pdf_oxide::editor::DocumentEditor;
use pdf_oxide::geometry::Rect;
use pdf_oxide::writer::{DocumentBuilder, PageSize, TextAnnotation, TextMarkupAnnotation};
use sp_check::checkers::Status;
use sp_check::report::Report;

pub(crate) fn annotate_bytes(input: &[u8], report: &Report) -> Result<Vec<u8>, crate::AnnotateError> {
    // 1. Build a one-page summary PDF.
    let summary = summary_text(report);
    let mut builder = DocumentBuilder::new();
    {
        let mut page = builder.page(PageSize::Letter);
        page = page.font("Helvetica", 9.0).at(54.0, 720.0);
        for line in summary.lines() {
            page = page.text(line).newline();
        }
        page.done();
    }
    let summary_bytes = builder.build()?;

    // 2. Editor starting from the summary page, then append the source pages.
    let mut editor = DocumentEditor::from_bytes(summary_bytes)?;
    editor.merge_from_bytes(input)?;

    // 3. Page heights for coordinate conversion (output index == source page 1-based).
    let page_count = editor.current_page_count();
    let mut heights = vec![0.0f32; page_count];
    for i in 0..page_count {
        let mb = editor.get_page_media_box(i)?;
        heights[i] = mb[3] - mb[1];
    }

    // 4. In-place annotations.
    for result in &report.results {
        if result.status == Status::Pass {
            continue;
        }
        for ev in &result.evidence {
            let Some(bbox) = ev.bbox else { continue };
            let page_idx = ev.page; // 1-based; summary occupies index 0
            if page_idx == 0 || page_idx >= page_count {
                continue;
            }
            let h = heights[page_idx];
            let rect = bbox_to_rect(bbox, h);
            let note_rect = Rect::new(rect.x, (rect.y + rect.height - 14.0).max(0.0), 14.0, 14.0);
            let contents = note_contents(
                &result.check_id,
                result.status.as_str(),
                &result.detail,
                ev.excerpt.as_deref(),
            );
            editor.edit_page(page_idx, |page| {
                page.add_annotation(
                    TextMarkupAnnotation::from_rect(TextMarkupType::Highlight, rect)
                        .with_color(1.0, 1.0, 0.0)
                        .with_opacity(0.4)
                        .with_author("scholarpress"),
                );
                page.add_annotation(TextAnnotation::new(note_rect, contents).with_author("scholarpress"));
                Ok(())
            })?;
        }
    }

    // 5. Save.
    editor.save_to_bytes().map_err(Into::into)
}
```

Add the `use` items shown above to the top of the file (they can be combined with the existing `use` block from Task 2).

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p sp-annotate`
Expected: all tests PASS, including `annotate_bytes_prepends_summary_and_adds_annotations` (or SKIP if the fixture is absent).

Note: if the summary page's `extract_text(0)` is empty, the standard-14 font name may need adjusting (try `"Times-Roman"` or `"Courier"`). The `%PDF` and `page_count + 1` assertions do not depend on the font.

- [ ] **Step 5: Commit**

```bash
git add crates/sp-annotate/src/annotate.rs
git commit -m "feat(sp-annotate): annotate_bytes prepends summary and writes annotations"
```

---

### Task 4: CLI `annotate` subcommand

Add a `scholarpress annotate` subcommand that consumes either a report JSON or a spec (running checks inline).

**Files:**
- Modify: `apps/scholarpress-cli/Cargo.toml` (add `sp-annotate`)
- Create: `apps/scholarpress-cli/src/annotate.rs`
- Modify: `apps/scholarpress-cli/src/main.rs` (register subcommand)

**Interfaces:**
- Consumes: `sp_annotate::{parse_report, annotate_file}`, `sp_check::{spec, engine, report}`, `clap`.
- Produces: `annotate::run(&AnnotateArgs)`, wired into the `Commands` enum.

- [ ] **Step 1: Add the dependency**

In `apps/scholarpress-cli/Cargo.toml`, add under `[dependencies]`:

```toml
sp-annotate = { path = "../../crates/sp-annotate" }
```

- [ ] **Step 2: Write the subcommand**

`apps/scholarpress-cli/src/annotate.rs`:

```rust
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
pub struct AnnotateArgs {
    #[arg(short, long, help = "Path to dissertation PDF")]
    pub pdf: PathBuf,

    #[arg(short, long, help = "Output path (default: <input>-annotated.pdf)")]
    pub out: Option<PathBuf>,

    #[arg(long, help = "Path to a sp-check JSON report")]
    pub report: Option<PathBuf>,

    #[arg(short, long, help = "Path to institution spec YAML (runs checks inline)")]
    pub spec: Option<PathBuf>,

    #[arg(long, help = "Run only this specific check (by check ID); with --spec only")]
    pub check: Option<String>,

    #[arg(short = 'C', long, help = "Run only checks in this category; with --spec only")]
    pub category: Option<String>,
}

pub fn run(args: &AnnotateArgs) {
    if !args.pdf.exists() {
        eprintln!("Error: PDF not found: {}", args.pdf.display());
        process::exit(2);
    }

    let report = match (&args.report, &args.spec) {
        (Some(_), Some(_)) => {
            eprintln!("Error: --report and --spec are mutually exclusive");
            process::exit(2);
        }
        (None, None) => {
            eprintln!("Error: one of --report or --spec is required");
            process::exit(2);
        }
        (Some(path), None) => {
            let json = std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("Error reading report: {}", e);
                process::exit(2);
            });
            sp_annotate::parse_report(&json).unwrap_or_else(|e| {
                eprintln!("Error parsing report: {}", e);
                process::exit(2);
            })
        }
        (None, Some(spec_path)) => {
            let spec = sp_check::spec::load_spec(spec_path).unwrap_or_else(|e| {
                eprintln!("Error loading spec: {}", e);
                process::exit(2);
            });
            let options = sp_check::engine::CheckOptions {
                check_ids: args.check.clone().map(|id| vec![id]),
                category: args.category.clone(),
            };
            let results = sp_check::engine::run_checks(&spec, &args.pdf, &options).unwrap_or_else(|e| {
                eprintln!("Error running checks: {}", e);
                process::exit(2);
            });
            sp_check::report::build_report(results)
        }
    };

    let out = args.out.clone().unwrap_or_else(|| default_out(&args.pdf));
    if let Err(e) = sp_annotate::annotate_file(&args.pdf, &out, &report) {
        eprintln!("Error annotating: {}", e);
        process::exit(2);
    }
    println!("Wrote annotated PDF: {}", out.display());
}

fn default_out(pdf: &Path) -> PathBuf {
    let stem = pdf.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    pdf.with_file_name(format!("{stem}-annotated.pdf"))
}
```

- [ ] **Step 3: Register the subcommand**

In `apps/scholarpress-cli/src/main.rs`:

```rust
mod annotate;
mod calibrate;
mod check;
```

Add to the `Commands` enum:

```rust
    /// Annotate a dissertation PDF with check-report findings
    Annotate(annotate::AnnotateArgs),
```

Add to the `match`:

```rust
        Commands::Annotate(args) => annotate::run(args),
```

- [ ] **Step 4: Build and smoke-test the CLI**

Run: `cargo build -p scholarpress-cli`
Then: `cargo run -p scholarpress-cli -- annotate --help`
Expected: usage text listing `--pdf`, `--out`, `--report`, `--spec`, `--check`, `--category`.

Also verify the mutual-exclusion guard: `cargo run -p scholarpress-cli -- annotate --pdf x.pdf` (with no `--report`/`--spec`) exits with the "one of --report or --spec is required" error.

- [ ] **Step 5: Commit**

```bash
git add apps/scholarpress-cli
git commit -m "feat(cli): add annotate subcommand"
```

---

### Task 5: MCP `annotate_pdf` tool

Add an `annotate_pdf` tool to `sp-mcp` that re-runs checks internally (retaining bboxes) and writes an annotated PDF into the workspace's `out/` directory.

**Files:**
- Modify: `crates/sp-mcp/Cargo.toml` (add `sp-annotate`)
- Modify: `crates/sp-mcp/src/error.rs` (add `Annotate` variant)
- Modify: `crates/sp-mcp/src/workspace.rs` (add `annotate_pdf` + test)
- Modify: `crates/sp-mcp/src/tools.rs` (params, handler, instructions)

**Interfaces:**
- Consumes: `sp_annotate::annotate_file`, existing `check::spec::load_spec`, `check::engine::run_checks`, `check::report::build_report`, `resolve_workspace`, `existing_under`, `canonical_root`, `output_under`, `SpMcpError`.
- Produces: `workspace::annotate_pdf(config, workspace, pdf_path, check_ids) -> Result<PathBuf, SpMcpError>`, exposed as the MCP tool `annotate_pdf`.

- [ ] **Step 1: Add the dependency and error variant**

In `crates/sp-mcp/Cargo.toml`, add under `[dependencies]`:

```toml
sp-annotate = { path = "../sp-annotate" }
```

In `crates/sp-mcp/src/error.rs`, add:

```rust
    #[error("annotate failed: {0}")]
    Annotate(String),
```

- [ ] **Step 2: Implement `annotate_pdf` in workspace.rs**

Add after the existing `check_pdf` function in `crates/sp-mcp/src/workspace.rs`:

```rust
pub fn annotate_pdf(
    config: &Config,
    workspace: &Path,
    pdf_path: &Path,
    check_ids: Option<&[String]>,
) -> Result<PathBuf, SpMcpError> {
    let workspace = resolve_workspace(config, workspace)?;
    let spec_path = workspace.join("spec.yaml");
    if !spec_path.is_file() {
        return Err(SpMcpError::SpecMissing(spec_path));
    }
    let pdf_abs = existing_under(&workspace, pdf_path, "pdf path")?;
    if !pdf_abs.is_file() {
        return Err(SpMcpError::Check(format!(
            "pdf not found: {}",
            pdf_abs.display()
        )));
    }

    let spec = check::spec::load_spec(&spec_path)
        .map_err(|e| SpMcpError::Check(format!("failed to load spec: {}", e)))?;
    let options = check::engine::CheckOptions {
        check_ids: check_ids.map(|ids| ids.to_vec()),
        ..Default::default()
    };
    let results = check::engine::run_checks(&spec, &pdf_abs, &options)
        .map_err(|e| SpMcpError::Check(format!("check run failed: {}", e)))?;
    let report = check::report::build_report(results);

    let stem = pdf_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "out".to_string());
    let out_name = format!("{stem}.annotated.pdf");

    let out_dir = workspace.join("out");
    std::fs::create_dir_all(&out_dir)?;
    let out_root = canonical_root(&out_dir, "output directory")?;
    let out_path = output_under(&out_root, Path::new(&out_name), "output path")?;

    sp_annotate::annotate_file(&pdf_abs, &out_path, &report)
        .map_err(|e| SpMcpError::Annotate(e.to_string()))?;

    Ok(out_path)
}
```

- [ ] **Step 3: Add a fixture-gated test**

Append to the `mod tests` in `crates/sp-mcp/src/workspace.rs` (reuse the existing `check_pdf_against_iu_baseline` fixture-location pattern):

```rust
#[test]
fn annotate_pdf_writes_annotated_copy() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = (0..6)
        .map(|i| {
            let mut p = manifest_dir.clone();
            for _ in 0..=i {
                p.pop();
            }
            p.join("scholarpress-catalog")
        })
        .find(|p| p.is_dir());
    let catalog = match catalog {
        Some(p) => p,
        None => {
            eprintln!("SKIP: scholarpress-catalog not found");
            return;
        }
    };
    let baseline = catalog.join("institutions/iu-indianapolis/tests/fixtures/baseline.pdf");
    if !baseline.is_file() {
        eprintln!("SKIP: IU baseline fixture not present (run bash compile.sh)");
        return;
    }

    let ws = local_tempdir();
    fs::write(
        ws.join("spec.yaml"),
        fs::read_to_string(catalog.join("institutions/iu-indianapolis/spec.yaml")).unwrap(),
    )
    .unwrap();
    fs::copy(&baseline, ws.join("baseline.pdf")).unwrap();

    let cfg = Config::new(catalog, ws.parent().unwrap().to_path_buf());
    let out = annotate_pdf(&cfg, &ws, Path::new("baseline.pdf"), None).unwrap();
    assert!(out.is_file(), "annotated pdf should exist at {}", out.display());
    assert!(out.ends_with("baseline.annotated.pdf"));
    assert!(fs::read(&out).unwrap().starts_with(b"%PDF"));
}
```

- [ ] **Step 4: Expose the MCP tool**

In `crates/sp-mcp/src/tools.rs`, add the params struct near the other `*Params` structs:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnnotatePdfParams {
    /// Absolute workspace path returned by create_workspace.
    pub workspace: PathBuf,
    /// PDF path relative to the workspace, normally out/entry.pdf.
    pub pdf_path: PathBuf,
    /// Optional check IDs to run; omit to run every check.
    pub check_ids: Option<Vec<String>>,
}
```

Add the tool handler inside `impl ScholarPressService` (after `check_pdf`):

```rust
    #[tool(
        description = "Run the workspace spec's checks against a PDF, then write an annotated copy (prepended summary page + in-place highlights and sticky notes for findings) below <workspace>/out/. Returns the annotated PDF's absolute path."
    )]
    async fn annotate_pdf(
        &self,
        params: Parameters<AnnotatePdfParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let out = workspace::annotate_pdf(
            &self.config,
            &p.workspace,
            &p.pdf_path,
            p.check_ids.as_deref(),
        )
        .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            out.display().to_string(),
        )]))
    }
```

Update the `get_info()` instructions string to append a sentence, e.g. after the `check_pdf` mention: "After check_pdf reports failures, call annotate_pdf to produce a visually annotated copy for the author."

- [ ] **Step 5: Run sp-mcp tests**

Run: `cargo test -p sp-mcp`
Expected: existing tests PASS; `annotate_pdf_writes_annotated_copy` PASS or SKIP (if the fixture is absent).

- [ ] **Step 6: Commit**

```bash
git add crates/sp-mcp
git commit -m "feat(sp-mcp): add annotate_pdf tool"
```

---

## Final verification

- [ ] Run the full pre-push gate:

```bash
cargo fmt --all && cargo clippy --all --tests -- -D warnings
```

- [ ] Run the whole test suite: `cargo test --workspace`
- [ ] Fix any clippy warnings or test failures before push.
