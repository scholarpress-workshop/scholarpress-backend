<img src="backend.png" width="100%" alt="ScholarPress Backend">

# ScholarPress Backend

Rust monorepo for scholarly document extraction, formatting checks, Typst compilation, and a publish-service API.

## Architecture

```
sp-extract (pdf_oxide, quick-xml, zip)
    |
    +---- sp-check (serde_yaml, regex)
    |
    +---- publish-service (axum 0.7)
    |         |
    |         +---- sp-typst (serde_json)
    |
    +---- scholarpress-cli (clap 4)
```

**`sp-extract`** — Single door for all document formats. Reads PDF/DOCX, produces one canonical `ParsedDocument` with paragraphs, headings, metadata, and per-page glyph spans.

**`sp-check`** — Formatting validation engine. 33 checkers across 9 categories (layout, typography, structure, content, footnotes, sections, title page, TOC, optional pages). Runs institution-defined specs from YAML against the extracted document.

**`sp-typst`** — Typst template rendering and native compilation. Template substitution from JSON data, shells out to the `typst` binary for PDF generation.

**`publish-service`** — Axum web server exposing extraction, checking, compilation, and institution catalog endpoints.

**`scholarpress-cli`** — Local command-line interface for checking dissertation PDFs and calibrating specs against a corpus.

## Quick Start

```bash
# Build everything
cargo build

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings
```

## Crates

### sp-extract

```rust
let doc = sp_extract::extract_pdf(&pdf_bytes)?;
let doc = sp_extract::extract_docx(&docx_bytes)?;
```

Returns `ParsedDocument` with:
- `pages` — per-page text, dimensions, word-level `TextSpan`s with font/bbox/color, image and path bounding boxes
- `paragraphs` — line-grouped text blocks with font properties and heading detection
- `headings` — detected document headings with levels
- `metadata` — title, author, page count, font list

### sp-check

```rust
let spec = sp_check::spec::load_spec(&path)?;
let results = sp_check::engine::run_checks(&spec, &pdf_path, &CheckOptions::default())?;
let report = sp_check::report::build_report(results);
```

Checker categories:
- `layout` — margins, margin symmetry
- `typography` — font size, weight, family, justification, title page formatting
- `structure` — section presence/order, page numbering, headings, hyperlinks
- `content` — boilerplate matching, committee order, TOC/title parity, word counts
- `footnotes` — font consistency
- `sections` — references heading/font, CV heading/name position, abstract formatting
- `title_page` — all-caps enforcement, clause centering and spacing
- `toc_details` — page number alignment, overhang, leader dots
- `optional_pages` — copyright page format

### sp-typst

```rust
let template = sp_typst::template::load_template(&template_dir)?;
let src = sp_typst::template::render_template(&template.main, &data)?;
let pdf = sp_typst::compile(&src)?;
```

## Apps

### publish-service

```
POST /extract       — Multipart upload (PDF/DOCX), returns ParsedDocument JSON
POST /check         — Base64 PDF + institution, runs validation checks
POST /compile       — Template data JSON, returns compiled PDF
GET  /health        — Health check
GET  /institutions  — List available institutions
GET  /institutions/:id/spec    — Institution specification
GET  /institutions/:id/template — Institution template
```

Default port: 3000.

### scholarpress-cli

```bash
# Run checks on a dissertation
scholarpress check --spec spec.yaml dissertation.pdf

# Filter by category
scholarpress check -C typography --spec spec.yaml dissertation.pdf

# Output as JSON
scholarpress check --json --spec spec.yaml dissertation.pdf

# Dump extracted document model
scholarpress check --dump-extract dissertation.pdf

# Calibrate against a corpus
scholarpress calibrate --spec spec.yaml --corpus path/to/pdfs/
```

## Requirements

- Rust 1.88+
- `typst` binary on PATH (for `sp-typst` compilation)
