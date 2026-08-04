<img src="backend.png" width="100%" alt="ScholarPress Backend">

# ScholarPress Backend

Rust monorepo for document extraction, formatting validation, Typst compilation, and an MCP server that powers the ScholarPress agent workflow.

## Architecture

```
sp-extract (pdf_oxide, quick-xml, zip)
    |
    +---- sp-check (serde_yaml, regex) ----+
    |                                       |
    +---- sp-typst                          |
    |                                       |
    +---- sp-mcp (rmcp, tokio)              |
              |                             |
              +---- scholarpress-cli (clap 4)
```

**`sp-extract`** — Single door for all document formats. Reads PDF/DOCX, produces one canonical `ParsedDocument` with paragraphs, headings, metadata, and per-page glyph spans.

**`sp-check`** — Formatting validation engine. 40 checkers across 9 categories (layout, typography, structure, content, footnotes, sections, title page, TOC, optional pages). Runs institution-defined specs from YAML against the extracted document.

**`sp-typst`** — Typst compilation wrapper. Compiles entry files with `--root` pointing to the workspace directory, shells out to the `typst` binary for PDF generation.

**`sp-mcp`** — MCP (Model Context Protocol) server. The primary product. Exposes workspace management tools (`list_profiles`, `create_workspace`), document conversion (`pandoc_convert`), Typst compilation and checking (`compile_typst`, `check_typst`, `format_typst`, `check_pdf`), and template introspection (`interface_doc`). Agents interact with ScholarPress through this server.

**`scholarpress-cli`** — Local command-line interface for checking dissertation PDFs against spec files. Used by the catalog's fixture validation suite.

## Quick Start

```bash
# Build everything
cargo build

# Run tests
cargo test --all

# Format and lint
cargo fmt --all --check
cargo clippy --all --tests -- -D warnings
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
let spec = sp_check::spec::load_spec(&spec_path)?;
let results = sp_check::engine::run_checks(&spec, &pdf_path, &CheckOptions::default())?;
let report = sp_check::report::build_report(results);
```

Checker categories:
- `layout` — margins, margin symmetry
- `typography` — font size, weight, family, justification, title page formatting
- `structure` — section presence/order, page numbering, headings, hyperlinks, new chapters
- `content` — boilerplate matching, committee order, TOC/title parity, word counts
- `footnotes` — font consistency
- `sections` — references heading/font, CV heading/name position, abstract formatting
- `title_page` — all-caps enforcement, clause centering and spacing
- `toc_details` — page number alignment, overhang, leader dots
- `optional_pages` — copyright page format

### sp-typst

```rust
let pdf_bytes = sp_typst::compile(&source, Some(workspace_dir))?;
```

Takes Typst source and optional workspace root. Returns compiled PDF bytes. On failure, generates clean `<file>:<line>:<col>` error text that agents read directly.

### sp-mcp

11 MCP tools available to agents:

| Tool | Purpose |
|------|---------|
| `list_workspaces` | Enumerate existing workspaces |
| `list_profiles` | Discover available formatting profiles |
| `create_workspace` | Fork a catalog profile into a scratch workspace |
| `compile_typst` | Compile a Typst entry file to PDF |
| `check_pdf` | Run all spec-defined checks against a PDF |
| `check_typst` | Validate Typst syntax (via typstyle) |
| `format_typst` | Format Typst in-place (via typstyle) |
| `pandoc_convert` | DOCX → Typst or JSON AST conversion |
| `interface_doc` | Structured reference of all section function signatures |

## Apps

### scholarpress-cli

```bash
# Run checks on a dissertation
scholarpress-cli check --spec spec.yaml dissertation.pdf

# Filter by category
scholarpress-cli check -C typography --spec spec.yaml dissertation.pdf

# Output as JSON
scholarpress-cli check --json --spec spec.yaml dissertation.pdf

# Dump extracted document model
scholarpress-cli check --dump-extract dissertation.pdf
```

## Requirements

- Rust 1.88+
- `typst` binary on PATH (for compilation)
- `pandoc` binary on PATH (for DOCX conversion)
- `typstyle` binary on PATH (for formatting and syntax checking)
