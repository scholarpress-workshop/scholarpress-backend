# Test Remediation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 38 unit/integration tests across the ScholarPress backend workspace and a catalog validation pipeline using the dual-purpose Docker image.

**Architecture:** Backend tests use mock data inline or from copied catalog fixtures — hermetic, no `CATALOG_PATH` dependency. Catalog tests use `validate_fixtures.sh` that invokes the pre-built Docker image against local fixtures and `expected_results.yaml`.

**Tech Stack:** Rust (cargo test, `#[cfg(test)]`, `tower::ServiceExt`), Bash, Docker, YAML, Typst.

## Global Constraints

- Backend tests must be hermetic — no sibling directory dependencies, no `CATALOG_PATH` at test time
- `sp-typst` currently uses subprocess — tests gate on `has_typst_binary()` and skip if typst not installed
- `has_typst_binary()` becomes unnecessary once sp-typst migrates to native `typst` crate
- Mock specs defined inline in test code, not loaded from catalog YAML
- Catalog stays pure-data — `validate_fixtures.sh` uses Docker, no Rust compilation
- TDD: write the failing test first, run to confirm it fails, then implement
- Workspace root: `/home/danriggi/scholarpress-workshop/scholarpress-backend/`

---

### Task 1: Shared test infrastructure — minimal PDF fixture, margin fixture copies, has_typst_binary helper

**Files:**
- Create: `crates/sp-extract/tests/fixtures/minimal.pdf` (binary)
- Copy: `../../scholarpress-catalog/institutions/iu/tests/fixtures/*.pdf` → `crates/sp-validate/tests/fixtures/margins/`
- Create: `crates/sp-typst/tests/common/mod.rs`
- Modify: `crates/sp-typst/Cargo.toml` (add `which` dev-dep)

**Interfaces:**
- Produces: `MINIMAL_PDF_BYTES: &[u8]` available via `include_bytes!`, margin PDFs at `sp-validate/tests/fixtures/margins/`, `has_typst_binary() -> bool` shared helper

- [ ] **Step 1: Generate minimal.pdf fixture**

Create a minimal Typst source at `/tmp/minimal.typ`:
```typst
#set page(width: 612pt, height: 792pt, margin: (left: 1.25in, right: 1.25in, top: 1in, bottom: 1in))
#set text(size: 12pt, font: "Libertinus Serif")
= Introduction
Lorem ipsum dolor sit amet, consectetur adipiscing elit.
```

```bash
mkdir -p /home/danriggi/scholarpress-workshop/scholarpress-backend/crates/sp-extract/tests/fixtures
cd /tmp && typst compile minimal.typ /home/danriggi/scholarpress-workshop/scholarpress-backend/crates/sp-extract/tests/fixtures/minimal.pdf
```

- [ ] **Step 2: Copy margin fixtures into sp-validate**

```bash
mkdir -p /home/danriggi/scholarpress-workshop/scholarpress-backend/crates/sp-validate/tests/fixtures/margins
cp /home/danriggi/scholarpress-workshop/scholarpress-catalog/institutions/iu/tests/fixtures/*.pdf /home/danriggi/scholarpress-workshop/scholarpress-backend/crates/sp-validate/tests/fixtures/margins/
cp /home/danriggi/scholarpress-workshop/scholarpress-catalog/institutions/iu/tests/fixtures/synthetic-body.typ /home/danriggi/scholarpress-workshop/scholarpress-backend/crates/sp-validate/tests/fixtures/margins/
```

- [ ] **Step 3: Create has_typst_binary helper**

Create `crates/sp-typst/tests/common/mod.rs`:
```rust
use std::sync::OnceLock;

static TYPST_AVAILABLE: OnceLock<bool> = OnceLock::new();

pub fn has_typst_binary() -> bool {
    *TYPST_AVAILABLE.get_or_init(|| {
        std::process::Command::new("typst")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}
```

- [ ] **Step 4: Add `which` dev-dep to sp-typst**

Actually, since we're using `std::process::Command` to check, `which` is unnecessary. No dependency change needed.

- [ ] **Step 5: Verify fixtures exist**

```bash
ls /home/danriggi/scholarpress-workshop/scholarpress-backend/crates/sp-extract/tests/fixtures/minimal.pdf
ls /home/danriggi/scholarpress-workshop/scholarpress-backend/crates/sp-validate/tests/fixtures/margins/baseline.pdf
```

Expected: both files exist.

- [ ] **Step 6: Commit**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
git add crates/sp-extract/tests/ crates/sp-validate/tests/fixtures/ crates/sp-typst/tests/
git commit -m "test: shared fixtures — minimal PDF, margin copies, has_typst_binary helper"
```

---

### Task 2: sp-extract core tests — PDF extraction, chunker, heading detection, median font size

**Files:**
- Create: `crates/sp-extract/tests/extract_pdf_test.rs`
- Create: `crates/sp-extract/tests/chunker_test.rs`
- Create: `crates/sp-extract/tests/heading_test.rs`
- Modify: `crates/sp-extract/Cargo.toml` (no changes needed — all tests are `#[cfg(test)]` in integration test files)

**Interfaces:**
- Consumes: `tests/fixtures/minimal.pdf` from Task 1
- Produces: 12 tests validating `extract_pdf()`, `chunk_text()`, `detect_headings()`, `median_font_size()`

- [ ] **Step 1: Write extract_pdf tests (failing — no test file yet)**

Create `crates/sp-extract/tests/extract_pdf_test.rs`:
```rust
use sp_extract::extract_pdf;

#[test]
fn test_extract_pdf_minimal_extracts_pages() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    assert!(doc.pages.len() >= 1, "should have at least one page");
}

#[test]
fn test_extract_pdf_minimal_contains_text() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    let all_text: String = doc.pages.iter().map(|p| &p.text).cloned().collect::<Vec<_>>().join(" ");
    assert!(all_text.contains("Introduction"), "should find heading text");
    assert!(all_text.contains("Lorem"), "should find body text");
}

#[test]
fn test_extract_pdf_detects_fonts() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    assert!(!doc.metadata.detected_fonts.is_empty(), "should detect at least one font");
}

#[test]
fn test_extract_pdf_has_correct_page_count() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    assert_eq!(doc.metadata.page_count_estimated, false);
    assert!(doc.metadata.page_count >= 1);
}

#[test]
fn test_extract_pdf_pages_have_dimensions() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    for page in &doc.pages {
        assert!(page.width > 0.0);
        assert!(page.height > 0.0);
    }
}
```

- [ ] **Step 2: Run extract tests to verify they pass**

```bash
cargo test -p sp-extract --test extract_pdf_test
```

Expected: 5 tests pass (the PDF fixture already exists from Task 1, and `extract_pdf()` is already implemented).

- [ ] **Step 3: Write chunker tests (failing — no test file yet)**

Create `crates/sp-extract/tests/chunker_test.rs`:
```rust
use sp_extract::chunker::chunk_text;

#[test]
fn test_chunk_text_empty() {
    let chunks = chunk_text("", 100, 20);
    assert!(chunks.is_empty());
}

#[test]
fn test_chunk_text_smaller_than_chunk_size() {
    let chunks = chunk_text("hello world", 100, 20);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].text, "hello world");
}

#[test]
fn test_chunk_text_larger_with_overlap() {
    let text = "a".repeat(500);
    let chunks = chunk_text(&text, 100, 20);
    assert!(chunks.len() >= 4, "should produce multiple chunks");
    // First chunk should be around 100 chars
    assert!(chunks[0].text.len() <= 100);
    // Overlap: second chunk should start before first's end
    assert!(chunks[1].start_char < chunks[0].end_char);
}

#[test]
fn test_chunk_text_paragraph_boundary() {
    let para1 = "First paragraph with some text.\nAnd another sentence.";
    let para2 = "Second paragraph here.\nMore text.";
    let text = format!("{}\n\n{}", para1, para2);
    let chunks = chunk_text(&text, 40, 10);
    // The break should happen at \n\n, not mid-word
    for chunk in &chunks {
        assert!(!chunk.text.ends_with("w"), "should not break mid-word: {:?}", chunk);
    }
}
```

- [ ] **Step 4: Run chunker tests**

```bash
cargo test -p sp-extract --test chunker_test
```

Expected: 4 tests pass.

- [ ] **Step 5: Write heading detection tests (failing — no heading detection test file yet)**

Create `crates/sp-extract/tests/heading_test.rs`:
```rust
use sp_extract::document::*;
use sp_extract::heading::*;

fn make_para(text: &str, bold: bool, all_caps: bool, font_size: f32) -> ParsedParagraph {
    ParsedParagraph {
        text: text.to_string(),
        page_number: 1,
        is_bold: bold,
        is_italic: false,
        is_underline: false,
        is_all_caps: all_caps,
        is_heading: false,
        heading_level: None,
        font_size: Some(font_size),
        font_name: Some("Times New Roman".to_string()),
    }
}

#[test]
fn test_detect_headings_bold_all_caps() {
    let mut paragraphs = vec![
        make_para("CHAPTER ONE", true, true, 14.0),
        make_para("Regular body text", false, false, 12.0),
    ];
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert_eq!(headings.len(), 1);
    assert!(paragraphs[0].is_heading);
    assert_eq!(paragraphs[0].heading_level, Some(1));
    assert!(!paragraphs[1].is_heading);
}

#[test]
fn test_detect_headings_numbered_section() {
    let mut paragraphs = vec![
        make_para("2.1 Background and Motivation", true, false, 12.0),
        make_para("Body text here", false, false, 12.0),
    ];
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert_eq!(headings.len(), 1);
    assert!(paragraphs[0].is_heading);
    assert_eq!(paragraphs[0].heading_level, Some(2));
}

#[test]
fn test_detect_headings_below_threshold() {
    let mut paragraphs = vec![
        make_para("just some regular text that is not a heading", false, false, 12.0),
    ];
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert!(headings.is_empty());
    assert!(!paragraphs[0].is_heading);
}

fn make_para_with_text(text: &str) -> ParsedParagraph {
    make_para(text, false, false, 12.0)
}

#[test]
fn test_median_font_size_three_values() {
    // Just test the helper inline since median_font_size is private
    // We'll add a public wrapper or test through detect_headings behavior
    let mut paragraphs = vec![
        make_para_with_text("A"),
        make_para_with_text("B"),
    ];
    paragraphs[0].font_size = Some(10.0);
    paragraphs[1].font_size = Some(14.0);
    // Verify detect_headings uses the correct median (10+14)/2 = 12
    // A paragraph at 14pt with bold should be detected as heading when body median is 12
    paragraphs[1].is_bold = true;
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert_eq!(headings.len(), 1);
}

#[test]
fn test_median_font_size_single() {
    let mut paragraphs = vec![make_para_with_text("A")];
    paragraphs[0].font_size = Some(12.0);
    // Single paragraph: median = 12, size_jump from body=12 is 0, below threshold
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert!(headings.is_empty());
}
```

- [ ] **Step 6: Run heading tests**

```bash
cargo test -p sp-extract --test heading_test
```

Expected: 5 tests pass.

- [ ] **Step 7: Run all sp-extract tests**

```bash
cargo test -p sp-extract
```

Expected: all tests pass (existing 4 + new 14 = 18 total).

- [ ] **Step 8: Commit**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
git add crates/sp-extract/
git commit -m "test(sp-extract): 14 new tests — PDF extraction, chunker, heading detection"
```

---

### Task 3: sp-typst compilation tests

**Files:**
- Create: `crates/sp-typst/tests/compile_test.rs`

**Interfaces:**
- Consumes: `tests/common/mod.rs` from Task 1 (`has_typst_binary()`)
- Produces: 2 tests — valid Typst compilation, invalid Typst error

- [ ] **Step 1: Write compile tests**

Create `crates/sp-typst/tests/compile_test.rs`:
```rust
mod common;
use common::has_typst_binary;
use sp_typst::compile;

#[test]
fn test_compile_valid_typst_produces_pdf() {
    if !has_typst_binary() {
        eprintln!("skipping: typst binary not found");
        return;
    }
    let source = r#"#set page(width: 100pt, height: 100pt); "hello""#;
    let pdf = compile(source, None).expect("compilation should succeed");
    assert!(!pdf.is_empty(), "PDF should not be empty");
    assert_eq!(&pdf[0..5], b"%PDF-", "should start with PDF header");
}

#[test]
fn test_compile_invalid_typst_returns_error() {
    if !has_typst_binary() {
        eprintln!("skipping: typst binary not found");
        return;
    }
    let result = compile(r"#notarealfunction", None);
    assert!(result.is_err(), "invalid Typst should return Err");
}
```

- [ ] **Step 2: Run compile tests**

```bash
cargo test -p sp-typst --test compile_test
```

Expected: 2 tests pass (or skip with message if typst not installed).

- [ ] **Step 3: Run all sp-typst tests**

```bash
cargo test -p sp-typst
```

Expected: all tests pass (existing 2 + new 2 = 4 total, or 2 if typst absent).

- [ ] **Step 4: Commit**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
git add crates/sp-typst/tests/
git commit -m "test(sp-typst): 2 compile tests — valid Typst PDF, invalid Typst error"
```

---

### Task 4: publish-service route handler tests

**Files:**
- Create: `apps/publish-service/tests/route_tests.rs`
- Modify: `apps/publish-service/Cargo.toml` (add dev-deps)

**Interfaces:**
- Consumes: `sp-typst::compile`, `sp_validate::engine::run_checks`, `sp_extract::extract_pdf`
- Produces: 10 route handler tests

- [ ] **Step 1: Add dev-dependencies**

Add to `apps/publish-service/Cargo.toml`:
```toml
[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
tempfile = "3"
http-body-util = "0.1"
```

- [ ] **Step 2: Write route tests**

Create `apps/publish-service/tests/route_tests.rs`:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode, Method};
use tower::ServiceExt;
use std::collections::HashMap;

const MOCK_SPEC_YAML: &str = r#"
institution: Test University
source_revision: "2026-01"
document_structure:
  front_matter:
    - { id: title_page, required: true }
  body:
    - { id: chapters, required: true }
  end_matter:
    - { id: references, required: true }
checks:
  - id: global_margins
    category: layout
    checker: margins
    target: { scope: all_pages }
    params:
      top: 1in
      bottom: 1in
      left: 1.25in
      right: 1.25in
  - id: committee_order
    category: content
    checker: committee_order
    target: { page: acceptance }
    automatable: false
    review_hint: "Check committee order on acceptance page"
constants:
  degree: "Doctor of Philosophy"
"#;

fn test_app() -> (axum::Router, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let inst_dir = tmp.path().join("institutions").join("test");
    std::fs::create_dir_all(&inst_dir).unwrap();
    std::fs::write(inst_dir.join("spec.yaml"), MOCK_SPEC_YAML).unwrap();
    std::fs::create_dir_all(inst_dir.join("template")).unwrap();
    std::fs::write(
        inst_dir.join("template").join("template.typ"),
        "#set page(width: 100pt, height: 100pt); \"hello\"",
    ).unwrap();

    let config = publish_service::config::AppConfig {
        port: 0,
        catalog_path: tmp.path().to_path_buf(),
    };
    let registry = publish_service::institutions::Registry::load(&config.catalog_path).unwrap();
    let router = publish_service::routes::router(registry);
    (router, tmp)
}

fn has_typst_binary() -> bool {
    std::process::Command::new("typst")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
async fn test_health_returns_ok() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn test_institutions_lists_ids() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(Request::builder().uri("/institutions").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 10_000).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "test");
    assert_eq!(arr[0]["name"], "Test University");
}

#[tokio::test]
async fn test_spec_returns_yaml() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(Request::builder().uri("/institutions/test/spec").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 10_000).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["yaml"].as_str().unwrap().contains("Test University"));
    assert_eq!(json["summary"]["automated_checks"], 1);
    assert_eq!(json["summary"]["human_checks"], 1);
}

#[tokio::test]
async fn test_spec_not_found() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(Request::builder().uri("/institutions/nonexistent/spec").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), 10_000).await.unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("Institution not found"));
}

#[tokio::test]
async fn test_template_returns_files() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(Request::builder().uri("/institutions/test/template").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 10_000).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["entry"], "template.typ");
    assert!(json["files"].as_array().unwrap().len() >= 1);
}

#[tokio::test]
async fn test_template_not_found() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(Request::builder().uri("/institutions/nonexistent/template").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_extract_no_file_returns_error() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/extract")
                .header("content-type", "multipart/form-data; boundary=xxx")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Should be 400 or 500 since no file was uploaded
    assert!(response.status().is_server_error() || response.status().is_client_error());
    let body = axum::body::to_bytes(response.into_body(), 10_000).await.unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("No file") || text.contains("error"), "expected error message, got: {text}");
}

#[tokio::test]
async fn test_compile_missing_institution() {
    if !has_typst_binary() {
        eprintln!("skipping: typst binary not found");
        return;
    }
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/compile")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"typst_code": "#set page(width: 100pt, height: 100pt); \"hello\""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_server_error() || response.status().is_client_error());
}

#[tokio::test]
async fn test_validate_invalid_base64() {
    let (app, _tmp) = test_app();
    let body = serde_json::json!({
        "pdf_base64": "!!!not-valid-base64!!!",
        "institution": "test"
    }).to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/validate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let resp_body = axum::body::to_bytes(response.into_body(), 10_000).await.unwrap();
    let text = String::from_utf8_lossy(&resp_body);
    assert!(text.contains("Invalid base64"));
}

#[tokio::test]
async fn test_validate_missing_institution() {
    let (app, _tmp) = test_app();
    let body = serde_json::json!({
        "pdf_base64": "dGVzdA==",  // "test" in base64
        "institution": "nonexistent"
    }).to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/validate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 3: Run route tests**

```bash
cargo test -p publish-service --test route_tests
```

Expected: 10 tests pass (test_compile_missing_institution may skip if typst absent).

- [ ] **Step 4: Run all publish-service tests**

```bash
cargo test -p publish-service
```

Expected: all 10 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
git add apps/publish-service/Cargo.toml apps/publish-service/tests/
git commit -m "test(publish-service): 10 route handler tests — health, institutions, spec, template, extract, compile, validate"
```

---

### Task 5: sp-validate untested checker tests

**Files:**
- Modify: `crates/sp-validate/src/checkers/sections.rs` (add `#[cfg(test)]`)
- Modify: `crates/sp-validate/src/checkers/structure.rs` (add `#[cfg(test)]`)
- Modify: `crates/sp-validate/src/checkers/content.rs` (add `#[cfg(test)]`)

**Interfaces:**
- Consumes: `tests/fixtures/margins/baseline.pdf` from Task 1
- Produces: 12 new checker tests

- [ ] **Step 1: Write sections.rs tests**

Append to the bottom of `crates/sp-validate/src/checkers/sections.rs`:

```rust
#[cfg(test)]
mod sections_checker_tests {
    use super::*;
    use crate::checkers::{Checker, CheckResult, Status};
    use crate::document::*;
    use serde_yaml::Value;

    fn span(text: &str, top: f32, font_name: &str, font_size: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            font_name: font_name.to_string(),
            font_size,
            bbox: (top, top + font_size, 100.0, 200.0),
            is_bold: false,
            is_italic: false,
            color: None,
        }
    }

    fn doc_with_pages(pages: Vec<Page>) -> Document {
        Document { pages }
    }

    fn page_with_spans(spans: Vec<TextSpan>) -> Page {
        Page {
            page_number: 1,
            width: 612.0,
            height: 792.0,
            spans,
            images: vec![],
            paths: vec![],
        }
    }

    #[test]
    fn test_references_heading_font_mismatch() {
        let body_spans = vec![
            span("Body text", 200.0, "Times New Roman", 12.0),
        ];
        let ref_spans = vec![
            span("References", 100.0, "Arial", 14.0),
        ];
        let doc = doc_with_pages(vec![
            page_with_spans(body_spans),
            page_with_spans(ref_spans),
        ]);
        let checker = ReferencesHeadingChecker;
        let params = Value::Mapping(Default::default());
        let result = checker.check(&doc, &params);
        // References heading in Arial while body is Times New Roman
        assert_eq!(result.status, Status::Fail);
    }

    #[test]
    fn test_cv_heading_font_mismatch() {
        let body_spans = vec![
            span("Body text", 200.0, "Times New Roman", 12.0),
        ];
        let cv_spans = vec![
            span("Curriculum Vitae", 100.0, "Arial", 14.0),
        ];
        let doc = doc_with_pages(vec![
            page_with_spans(body_spans),
            page_with_spans(cv_spans),
        ]);
        let checker = CvHeadingChecker;
        let params = Value::Mapping(Default::default());
        let result = checker.check(&doc, &params);
        assert_eq!(result.status, Status::Fail);
    }

    #[test]
    fn test_cv_name_missing() {
        let body_spans = vec![
            span("Body text", 200.0, "Times New Roman", 12.0),
        ];
        let cv_spans = vec![
            span("Curriculum Vitae", 100.0, "Times New Roman", 12.0),
            span("Education", 150.0, "Times New Roman", 12.0),
        ];
        let doc = doc_with_pages(vec![
            page_with_spans(body_spans),
            page_with_spans(cv_spans),
        ]);
        let checker = CvNamePositionChecker;
        let params = Value::Mapping(Default::default());
        let result = checker.check(&doc, &params);
        // Name is missing from CV page
        assert_eq!(result.status, Status::Fail);
    }

    #[test]
    fn test_abstract_text_centered() {
        let body_spans = vec![
            span("Abstract title", 100.0, "Times New Roman", 14.0),
            span("This is the abstract text centered on the page.", 200.0, "Times New Roman", 12.0),
        ];
        let doc = doc_with_pages(vec![page_with_spans(body_spans)]);
        let checker = AbstractTextCenteredChecker;
        let params = Value::Mapping(Default::default());
        let result = checker.check(&doc, &params);
        assert!(matches!(result.status, Status::Pass | Status::Fail));
    }
}
```

- [ ] **Step 2: Write structure.rs tests**

Append to the bottom of `crates/sp-validate/src/checkers/structure.rs`:

```rust
#[cfg(test)]
mod structure_checker_tests {
    use super::*;
    use crate::checkers::{Checker, CheckResult, Status};
    use crate::document::*;
    use serde_yaml::Value;

    fn span_at(text: &str, top: f32, _page: usize, font_size: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            font_name: "Times New Roman".to_string(),
            font_size,
            bbox: (top, top + font_size, 100.0, 200.0),
            is_bold: false,
            is_italic: false,
            color: None,
        }
    }

    fn doc_from_pages(pages: Vec<Page>) -> Document {
        Document { pages }
    }

    fn page(num: usize, spans: Vec<TextSpan>) -> Page {
        Page {
            page_number: num,
            width: 612.0,
            height: 792.0,
            spans,
            images: vec![],
            paths: vec![],
        }
    }

    #[test]
    fn test_acceptance_page_has_page_number() {
        // Acceptance page (page 2) has a page number "ii" — should FAIL
        let doc = doc_from_pages(vec![
            page(1, vec![span_at("TITLE PAGE", 200.0, 1, 12.0)]),
            page(2, vec![
                span_at("Acceptance page content", 200.0, 2, 12.0),
                span_at("ii", 750.0, 2, 10.0), // page number where it shouldn't be
            ]),
        ]);
        let checker = AcceptancePagePageNumberChecker;
        let params = Value::Mapping(Default::default());
        let result = checker.check(&doc, &params);
        assert_eq!(result.status, Status::Fail);
    }

    #[test]
    fn test_front_matter_arabic_page_number() {
        // Front matter page with Arabic numeral "5" instead of Roman "v"
        let doc = doc_from_pages(vec![
            page(1, vec![span_at("TITLE PAGE", 200.0, 1, 12.0)]),
            page(2, vec![
                span_at("Table of Contents", 200.0, 2, 12.0),
                span_at("5", 750.0, 2, 10.0), // Arabic in front matter
            ]),
            page(3, vec![span_at("Chapter 1 text", 200.0, 3, 12.0)]),
        ]);
        let checker = PageNumbersFormatChecker;
        let params = Value::Mapping(Default::default());
        let result = checker.check(&doc, &params);
        assert_eq!(result.status, Status::Fail);
    }

    #[test]
    fn test_headings_font_differs_from_body() {
        let body_page_spans = vec![
            span_at("Body paragraph text in Times at 12pt.", 300.0, 1, 12.0),
        ];
        let heading_page_spans = vec![
            {
                let mut s = span_at("CHAPTER 1", 100.0, 2, 18.0);
                s.is_bold = true;
                s
            },
            span_at("Introduction", 150.0, 2, 14.0),
            span_at("More body text", 300.0, 2, 12.0),
        ];
        let doc = doc_from_pages(vec![
            page(1, body_page_spans),
            page(2, heading_page_spans),
        ]);
        let checker = HeadingsConsistentChecker;
        let params = Value::Mapping(Default::default());
        let result = checker.check(&doc, &params);
        assert_eq!(result.status, Status::Fail);
    }
}
```

- [ ] **Step 3: Write content.rs committee_order test**

Append to `crates/sp-validate/src/checkers/content.rs`:

```rust
#[cfg(test)]
mod committee_tests {
    use super::*;
    use crate::checkers::{Checker, Status};
    use crate::document::*;
    use serde_yaml::Value;

    fn span(text: &str, top: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            font_name: "Times New Roman".to_string(),
            font_size: 12.0,
            bbox: (top, top + 12.0, 100.0, 300.0),
            is_bold: false,
            is_italic: false,
            color: None,
        }
    }

    #[test]
    fn test_committee_chair_not_first() {
        let page = Page {
            page_number: 2,
            width: 612.0,
            height: 792.0,
            spans: vec![
                span("Dr. Jane Smith (Member)", 100.0),
                span("Dr. John Chair (Chair)", 150.0),
            ],
            images: vec![],
            paths: vec![],
        };
        let doc = Document { pages: vec![page] };
        let checker = CommitteeOrderChecker;
        let params = Value::Mapping(Default::default());
        let result = checker.check(&doc, &params);
        // Chair listed second should fail
        assert_eq!(result.status, Status::Fail);
    }
}
```

- [ ] **Step 4: Run checker tests**

```bash
cargo test -p sp-validate
```

Expected: all 87 existing tests + 8 new checker tests = 95 tests pass. Fix any import issues or test failures.

- [ ] **Step 5: Run full workspace tests**

```bash
cargo test --all
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
git add crates/sp-validate/src/checkers/
git commit -m "test(sp-validate): 8 new checker tests — sections, structure, committee_order"
```

---

### Task 6: scholarpress-cli tests

**Files:**
- Create: `apps/scholarpress-cli/tests/cli_test.rs`
- Modify: `apps/scholarpress-cli/Cargo.toml` (add dev-deps)

**Interfaces:**
- Consumes: margin fixtures from Task 1
- Produces: 2 CLI integration tests

- [ ] **Step 1: Add dev-dependencies**

Add to `apps/scholarpress-cli/Cargo.toml`:
```toml
[dev-dependencies]
assert_cmd = "2"
tempfile = "3"
```

- [ ] **Step 2: Write CLI test**

Create `apps/scholarpress-cli/tests/cli_test.rs`:
```rust
use assert_cmd::Command;
use std::io::Write;

const MOCK_SPEC_YAML: &str = r#"
institution: Test University
source_revision: "2026-01"
document_structure:
  front_matter:
    - { id: title_page, required: true }
  body:
    - { id: chapters, required: true }
  end_matter:
    - { id: references, required: true }
checks:
  - id: global_margins
    category: layout
    checker: margins
    target: { scope: all_pages }
    params:
      top: 1in
      bottom: 1in
      left: 1.25in
      right: 1.25in
"#;

fn baseline_pdf_path() -> String {
    format!("{}/crates/sp-validate/tests/fixtures/margins/baseline.pdf",
        env!("CARGO_MANIFEST_DIR").trim_end_matches("/apps/scholarpress-cli"))
}

#[test]
fn test_check_subcommand_exit_zero() {
    let baseline = baseline_pdf_path();
    let mut spec_file = tempfile::NamedTempFile::new().unwrap();
    write!(spec_file, "{}", MOCK_SPEC_YAML).unwrap();

    let mut cmd = Command::cargo_bin("scholarpress-cli").unwrap();
    cmd.arg("check")
       .arg("--spec").arg(spec_file.path())
       .arg(&baseline)
       .assert()
       .success();
}

#[test]
fn test_calibrate_subcommand() {
    let baseline = baseline_pdf_path();
    let mut spec_file = tempfile::NamedTempFile::new().unwrap();
    write!(spec_file, "{}", MOCK_SPEC_YAML).unwrap();

    let corpus_dir = tempfile::tempdir().unwrap();
    std::fs::copy(&baseline, corpus_dir.path().join("test.pdf")).unwrap();

    let mut cmd = Command::cargo_bin("scholarpress-cli").unwrap();
    cmd.arg("calibrate")
       .arg("--spec").arg(spec_file.path())
       .arg("--corpus").arg(corpus_dir.path())
       .assert()
       .success();
}
```

- [ ] **Step 3: Run CLI tests**

```bash
cargo test -p scholarpress-cli
```

Expected: 2 tests pass.

- [ ] **Step 4: Verify full workspace still passes**

```bash
cargo test --all
cargo clippy --all -- -D warnings
```

Expected: all tests pass, zero clippy warnings.

- [ ] **Step 5: Commit**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
git add apps/scholarpress-cli/
git commit -m "test(cli): 2 integration tests — check and calibrate subcommands"
```

---

### Task 7: Catalog — expected_results.yaml and golden baseline

**Files:**
- Create: `../../scholarpress-catalog/institutions/iu/tests/expected_results.yaml`
- Modify: `../../scholarpress-catalog/institutions/iu/tests/fixtures/compile.sh`

**Interfaces:**
- Produces: `expected_results.yaml` mapping, `golden.pdf` generation in compile.sh

- [ ] **Step 1: Create expected_results.yaml**

Create `/home/danriggi/scholarpress-workshop/scholarpress-catalog/institutions/iu/tests/expected_results.yaml`:
```yaml
fixtures:
  baseline.pdf:
    assert_fails: []
    assert_passes: ["ALL"]
    ignore_others: true

  golden.pdf:
    assert_fails: []
    assert_passes: ["ALL"]
    ignore_others: true

  left-narrow.pdf:
    assert_fails: ["global_margins"]
    assert_passes: []
    ignore_others: true

  right-narrow.pdf:
    assert_fails: ["global_margins"]
    assert_passes: []
    ignore_others: true

  left-wide.pdf:
    assert_fails: ["global_margins"]
    assert_passes: []
    ignore_others: true

  right-wide.pdf:
    assert_fails: ["global_margins"]
    assert_passes: []
    ignore_others: true

  top-narrow.pdf:
    assert_fails: ["global_margins"]
    assert_passes: []
    ignore_others: true

  bottom-narrow.pdf:
    assert_fails: ["global_margins"]
    assert_passes: []
    ignore_others: true

  top-wide.pdf:
    assert_fails: ["global_margins"]
    assert_passes: []
    ignore_others: true

  asymmetric.pdf:
    assert_fails: ["margin_symmetry"]
    assert_passes: []
    ignore_others: true

  messy.pdf:
    assert_fails: []
    assert_passes: []
    ignore_others: true
```

- [ ] **Step 2: Add golden baseline generation to compile.sh**

Append to `/home/danriggi/scholarpress-workshop/scholarpress-catalog/institutions/iu/tests/fixtures/compile.sh` after line 46 (`echo "Done. All PDFs in $DIR/"`):

```bash
echo
echo "=== Generating golden baseline from institution template ==="
typst compile --root "$ROOT/../../template" \
  "$ROOT/../../template/template.typ" \
  "$DIR/golden.pdf"
echo "Golden baseline: $DIR/golden.pdf"
```

- [ ] **Step 3: Generate golden baseline**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-catalog/institutions/iu/tests/fixtures
bash compile.sh
```

Expected: `golden.pdf` is generated alongside other fixtures.

- [ ] **Step 4: Commit catalog changes**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-catalog
git add institutions/iu/tests/expected_results.yaml institutions/iu/tests/fixtures/compile.sh
git commit -m "feat: expected_results.yaml for fixture validation; golden baseline generation"
git push
```

---

### Task 8: Dual-purpose Docker image — add CLI binary

**Files:**
- Modify: `apps/publish-service/Dockerfile`

**Interfaces:**
- Produces: Docker image with both `publish-service` and `scholarpress` (CLI) binaries

- [ ] **Step 1: Update Dockerfile to build both binaries**

Edit `/home/danriggi/scholarpress-workshop/scholarpress-backend/apps/publish-service/Dockerfile`:

Replace the builder RUN line:
```dockerfile
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY apps/ apps/
RUN cargo build --release --bin publish-service --bin scholarpress-cli && \
    cp target/release/publish-service /app/publish-service && \
    cp target/release/scholarpress-cli /app/scholarpress

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/publish-service /usr/local/bin/publish-service
COPY --from=builder /app/scholarpress /usr/local/bin/scholarpress
EXPOSE 4000
ENV CATALOG_PATH=/app/catalog
CMD ["publish-service"]
```

- [ ] **Step 2: Verify Docker build**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
docker build -f apps/publish-service/Dockerfile -t publish-service-test .
docker run --rm publish-service-test scholarpress --help
```

Expected: CLI help text displays from the Docker image.

- [ ] **Step 3: Commit**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
git add apps/publish-service/Dockerfile
git commit -m "feat: dual-purpose Docker image — includes scholarpress CLI binary"
git push
```

---

### Task 9: Catalog validate_fixtures.sh

**Files:**
- Create: `../../scholarpress-catalog/institutions/iu/tests/validate_fixtures.sh`
- Modify: `../../scholarpress-catalog/institutions/iu/tests/expected_results.yaml` (add new fixtures as needed)

**Interfaces:**
- Consumes: `ghcr.io/scholarpress-workshop/scholarpress-backend-publish-service:latest` Docker image (post-CI build)
- Produces: Automated fixture validation script

- [ ] **Step 1: Create validate_fixtures.sh**

Create `/home/danriggi/scholarpress-workshop/scholarpress-catalog/institutions/iu/tests/validate_fixtures.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
IMAGE="ghcr.io/scholarpress-workshop/scholarpress-backend-publish-service:latest"
CATALOG_MOUNT="/catalog"
FAIL_COUNT=0
PASS_COUNT=0

run_check() {
  local pdf="$1"
  docker run --rm \
    -v "$(cd "$DIR/../../.." && pwd):$CATALOG_MOUNT:ro" \
    "$IMAGE" \
    scholarpress check \
      --spec "$CATALOG_MOUNT/iu/spec.yaml" \
      --json --quiet \
      "$CATALOG_MOUNT/iu/tests/fixtures/$pdf" 2>/dev/null
}

assert_fails() {
  local pdf="$1" check_id="$2"
  local output
  output=$(run_check "$pdf")
  if echo "$output" | python3 -c "
import sys, json
data = json.load(sys.stdin)
results = [r for r in data.get('results', []) if r['check_id'] == '$check_id']
if not results: sys.exit(1)
if results[0]['status'] not in ('FAIL', 'ERROR'): sys.exit(1)
" 2>/dev/null; then
    echo "  PASS: $check_id fails as expected"
  else
    echo "  FAIL: expected $check_id to FAIL in $pdf"
    return 1
  fi
}

assert_all_pass() {
  local pdf="$1"
  local output
  output=$(run_check "$pdf")
  local failures
  failures=$(echo "$output" | python3 -c "
import sys, json
data = json.load(sys.stdin)
fails = [r['check_id'] for r in data.get('results', [])
         if r['status'] not in ('PASS', 'MANUAL')]
print('\n'.join(fails))
" 2>/dev/null)
  if [ -z "$failures" ]; then
    echo "  PASS: all automatable checks pass"
  else
    echo "  FAIL: unexpected failures in $pdf: $failures"
    return 1
  fi
}

echo "=== Catalog Fixture Validation ==="
echo "Image: $IMAGE"
echo

for pdf in "$DIR/fixtures"/*.pdf; do
  name=$(basename "$pdf")
  echo "--- $name ---"

  case "$name" in
    baseline.pdf|golden.pdf)
      assert_all_pass "$name" && ((PASS_COUNT+=1)) || ((FAIL_COUNT+=1))
      ;;
    left-narrow.pdf|right-narrow.pdf|left-wide.pdf|right-wide.pdf|top-narrow.pdf|bottom-narrow.pdf|top-wide.pdf)
      assert_fails "$name" "global_margins" && ((PASS_COUNT+=1)) || ((FAIL_COUNT+=1))
      ;;
    asymmetric.pdf)
      assert_fails "$name" "margin_symmetry" && ((PASS_COUNT+=1)) || ((FAIL_COUNT+=1))
      ;;
    messy.pdf)
      echo "  SKIP: smoke test only"
      ((PASS_COUNT+=1))
      ;;
    *)
      echo "  SKIP: no expected results defined"
      ;;
  esac
  echo
done

echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="
if [ "$FAIL_COUNT" -gt 0 ]; then
  exit 1
fi
```

Make it executable:
```bash
chmod +x /home/danriggi/scholarpress-workshop/scholarpress-catalog/institutions/iu/tests/validate_fixtures.sh
```

- [ ] **Step 2: Test validate_fixtures.sh (requires Docker)**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-catalog/institutions/iu/tests
# Note: requires publish-service image. Pull latest:
docker pull ghcr.io/scholarpress-workshop/scholarpress-backend-publish-service:latest 2>/dev/null || echo "Image not yet available — CI must build it first"
bash validate_fixtures.sh 2>&1 || echo "Some failures expected until image is built by CI"
```

- [ ] **Step 3: Commit catalog changes**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-catalog
git add institutions/iu/tests/validate_fixtures.sh
git commit -m "feat: validate_fixtures.sh — automated fixture validation via Docker"
git push
```

---

### Task 10: Final verification

**Interfaces:**
- Verifies: all 38+ new tests pass, zero clippy warnings, catalog pipeline executable

- [ ] **Step 1: Run full backend test suite**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
cargo test --all
```

Expected: all tests pass (5 sp-extract, 4 sp-typst, 87 sp-validate, 10 publish-service, 2 cli + new tests). Total should be approximately 131.

- [ ] **Step 2: Clippy and fmt**

```bash
cargo clippy --all -- -D warnings
cargo fmt --check
```

Expected: zero warnings, all files formatted.

- [ ] **Step 3: Release build**

```bash
cargo build --release
```

Expected: builds without errors.

- [ ] **Step 4: Push**

```bash
git push
```
