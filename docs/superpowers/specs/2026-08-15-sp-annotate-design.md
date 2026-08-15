# ScholarPress: `sp-annotate` — PDF annotation of check reports

## Context

`sp-check` produces a `Report` of formatting-check results against a
dissertation PDF. Today that report only exists as console text or JSON —
there is no way for an author to *see* the findings in the document itself.
This crate produces an annotated copy of the input PDF: a prepended summary
page for document-level findings, and in-place highlights + sticky notes for
findings tied to specific coordinates.

`sp-extract` already uses `pdf_oxide` 0.3.73 for text extraction. `pdf_oxide`
also supports writing annotations (text "sticky" notes, and "text markup"
including Highlight) and saving/editing documents, so no new PDF dependency is
required.

## Design

### New crate `sp-annotate` (`crates/sp-annotate`)

Dependencies: `pdf_oxide 0.3`, `sp-check` (path), `serde`, `serde_json`,
`thiserror`.

Public API in `src/lib.rs`:

- `parse_report(json: &str) -> Result<Report>` — deserialize the `sp-check`
  JSON report (the output of `format_json`).
- `annotate_bytes(input: &[u8], report: &Report) -> Result<Vec<u8>>` — open the
  PDF, prepend the summary page, add highlights + sticky notes, save, return
  the annotated copy as bytes.
- `annotate_file(input: &Path, output: &Path, report: &Report) -> Result<()>`
  — file wrapper over `annotate_bytes`.

### `sp-check` change (small)

Add `Deserialize` to `Status` (with `#[serde(rename_all = "UPPERCASE")]` so it
round-trips the `"PASS"/"FAIL"/"MANUAL"/"ERROR"` strings `format_json` emits),
and to `EvidenceItem`, `CheckResult`, `Summary`, and `Report`. No other
behavior changes.

### Annotation rules

For each `CheckResult`, partition evidence:

- **`bbox: Some(b)`** → in-place annotation: a yellow highlight quad over the
  region plus a sticky note whose text is
  `[{check_id}] {status}\n{detail}` (with `excerpt` appended when present).
  Applies to FAIL/ERROR/MANUAL evidence that carries coordinates.
- **`bbox: None`** (or the whole result if PASS) → collected into the
  prepended report page only.

In-place (bbox) findings are *not* duplicated into the prepended page — they
are already annotated where they occur.

### Prepended report page

Insert a new first page rendering `sp_check::report::format_text(&report)`.
Reuses the existing summary text (counts + non-bbox/global findings). No new
formatting logic.

### Coordinate conversion

`bbox` is `(top, bottom, x0, x1)` in top-left-origin page space (see
`sp-extract/src/document.rs`). `pdf_oxide` highlights expect `quad_points` in
bottom-left PDF space. With page height `H`:

```
y_pdf = H - y
quad = [(x0, H-top), (x1, H-top), (x0, H-bottom), (x1, H-bottom)]
```

### CLI

New `annotate` subcommand in `apps/scholarpress-cli`:

| Arg | Description |
|-----|-------------|
| `--pdf <in>` | Input PDF path |
| `--out <out>` | Output path; default `<stem>-annotated.pdf` next to input |
| `--report <json>` | Consume a pre-existing `sp-check --json` report |
| `--spec <yaml>` | Run checks inline, then annotate (mutually exclusive with `--report`) |
| `--check` / `--category` | Existing filters, valid only with `--spec` |

Exactly one of `--report` / `--spec` is required.

### MCP

New tool `annotate_pdf(workspace, pdf_path, check_ids?)` in `sp-mcp`:

- Re-runs `check::engine::run_checks` internally (NOT via the existing
  `check_pdf`, whose `CheckOutcome`/`EvidenceDetail` drops `bbox` — see
  `workspace.rs:323-337`), keeping the full `Report` with coordinates.
- Calls `sp_annotate::annotate_file`, writing to
  `<workspace>/out/<stem>.annotated.pdf`.
- Returns the output path.

Adds `sp-annotate` to `sp-mcp` dependencies.

### File-level changes

| File | Change |
|------|--------|
| `crates/sp-annotate/src/lib.rs` | New crate: `parse_report`, `annotate_bytes`, `annotate_file`, annotation + coordinate logic. |
| `crates/sp-annotate/Cargo.toml` | New crate manifest. |
| `crates/sp-check/src/checkers/mod.rs` | Add `Deserialize` to `Status` (+`rename_all`), `EvidenceItem`, `CheckResult`. |
| `crates/sp-check/src/report.rs` | Add `Deserialize` to `Summary`, `Report`. |
| `apps/scholarpress-cli/src/main.rs` | Register `annotate` subcommand. |
| `apps/scholarpress-cli/src/annotate.rs` | New module: arg parsing + orchestration. |
| `crates/sp-mcp/src/tools.rs` | `AnnotatePdfParams`, `annotate_pdf` tool handler, `get_info()` instructions. |
| `crates/sp-mcp/src/workspace.rs` | `annotate_pdf` impl (re-run checks + call sp-annotate). |
| `crates/sp-mcp/Cargo.toml` | Add `sp-annotate` dependency. |

## Non-goals

- **Re-annotating from `check_pdf`'s JSON output.** That output omits `bbox`,
  so `annotate_pdf` re-runs checks internally instead.
- **Configurable annotation colors / icons.** Fixed yellow highlight + default
  sticky-note icon for now. Add knobs if a need emerges.
- **Rich prepended report styling.** Plain text rendered from the existing
  `format_text` output; no tables/graphics on the summary page.
- **`--report` in MCP.** The CLI supports a report artifact; MCP re-runs
  internally since it already has the workspace + spec on hand.

## Verification

- `cargo test -p sp-check` — report round-trips through JSON.
- `cargo test -p sp-annotate` — quad-point math, evidence partitioning, and
  (fixture-gated) annotating the IU `baseline.pdf` asserting output starts
  with `%PDF` and has `page_count + 1` pages.
- `cargo test -p sp-mcp` — `annotate_pdf` writes `<stem>.annotated.pdf`
  (skip-if-no-fixture, matching existing tests).
- `cargo fmt --all && cargo clippy --all --tests -- -D warnings` clean.

## Risks

- **Exact `pdf_oxide` write-annotation/save API.** The `Annotation` struct
  (`subtype_enum`, `quad_points`, `contents`, `rect`, `color`) and page-add /
  incremental-save features are confirmed present in 0.3.73, but the precise
  method names for attaching an annotation to a page and prepending a rendered
  text page must be verified against the 0.3.73 source before implementation.
  De-risk as the first implementation task.
