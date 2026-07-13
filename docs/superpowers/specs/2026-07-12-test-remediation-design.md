# Test Remediation Plan

Date: 2026-07-12

## 1. Objective

Close the test coverage gaps identified in the ScholarPress ecosystem audit. Separate testing concerns between `scholarpress-backend` (tests the code) and `scholarpress-catalog` (tests the data), using catalog fixtures as shared test assets without duplicating effort.

## 2. Test Responsibility Matrix

| Domain | Repository | Tests What | Fixture Source |
|--------|------------|------------|---------------|
| Engine correctness | `scholarpress-backend` | Algorithms, parsing, heuristics, API handlers | Mock data defined inline or copied from catalog |
| Data correctness | `scholarpress-catalog` | Spec YAML validity, template output, fixture expectations | Catalog's own spec.yaml and Typst templates |

**Key constraint**: Backend tests must be hermetic — a test failure means Rust code broke, not that a sibling directory is missing. Catalog tests assume the engine works and validate the configuration independently.

## 3. Backend Tests (`scholarpress-backend`)

All backend tests use mock specs defined in code or minimal file fixtures copied into the crate. No dependency on `CATALOG_PATH` or sibling directories at test time.

### 3.1 `sp-extract` — Core Extraction Tests

**Status**: 4 trivial helper tests (median, invalid DOCX). Zero tests on extraction pipelines.

#### New Tests

**`tests/fixtures/minimal.pdf`** — A 1-page Typst PDF generated from:
```typst
#set page(width: 612pt, height: 792pt, margin: (left: 1.25in, right: 1.25in, top: 1in, bottom: 1in))
#set text(size: 12pt, font: "Libertinus Serif")
= Introduction
Lorem ipsum dolor sit amet, consectetur adipiscing elit.
```

Check this PDF into `crates/sp-extract/tests/fixtures/minimal.pdf` as a binary test asset.

**Test: `test_extract_pdf_minimal`**

```rust
#[test]
fn test_extract_pdf_minimal() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    assert!(doc.pages.len() >= 1);
    assert!(doc.pages[0].text.contains("Introduction"));
    assert!(!doc.metadata.detected_fonts.is_empty());
}
```

**Test: `test_extract_pdf_returns_pages`** — single page count assertion, non-zero width/height.

**Test: `test_extract_pdf_detects_fonts`** — detected_fonts is non-empty after extraction.

**Test: `test_chunk_text_empty`** — empty string returns empty vec.

**Test: `test_chunk_text_smaller_than_chunk`** — text shorter than max_chars returns single chunk.

**Test: `test_chunk_text_with_overlap`** — 1000-char text with max_chars=200, overlap=50 produces staggered chunks. Assert first chunk ends near 200, second starts before 200.

**Test: `test_chunk_text_paragraph_boundary`** — text with `\n\n` breaks at natural boundaries, not mid-word.

**Test: `test_detect_headings_bold_allcaps`** — a ParsedParagraph with `is_bold: true, is_all_caps: true` scores above threshold, gets `is_heading: true`.

**Test: `test_detect_headings_numbered_section`** — paragraph starting with `"2.1"` matches section regex, assigned level 2.

**Test: `test_detect_headings_below_threshold`** — plain lowercase paragraph with no formatting signals scores below threshold, remains `is_heading: false`.

**Test: `test_median_font_size`** — three paragraphs at 10pt, 12pt, 14pt → median = 12pt.

**Test: `test_median_font_size_single`** — single paragraph → font_size returned unchanged.

Target: 12 new tests.

### 3.2 `sp-typst` — Compilation Tests

**Status**: 2 trivial helper tests (variable substitution, empty dir). `compile()` has zero tests.

#### New Tests

**Test: `test_compile_valid_typst`** — compile minimal Typst source, assert output starts with `%PDF-`:
```rust
#[test]
fn test_compile_valid_typst() {
    if !has_typst_binary() { return; } // skip if typst not installed
    let source = r#"#set page(width: 100pt, height: 100pt); "hello""#;
    let pdf = compile(source, None).expect("compilation should succeed");
    assert!(!pdf.is_empty());
    assert_eq!(&pdf[0..5], b"%PDF-");
}
```

**Test: `test_compile_invalid_typst`** — invalid source returns `Err`:
```rust
#[test]
fn test_compile_invalid_typst() {
    if !has_typst_binary() { return; }
    let result = compile(r"#notarealfunction", None);
    assert!(result.is_err());
}
```

Helper: `fn has_typst_binary() -> bool` checks `which::which("typst").is_ok()`.

Target: 2 new tests.

### 3.3 `publish-service` — Route Handler Tests

**Status**: 0 tests. Empty `tests/` directory.

#### Test Infrastructure

A `TestApp` helper that builds an axum `Router` with a `Registry` loaded from a temp directory containing a minimal mock institution:

```rust
fn test_app() -> (Router, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let inst_dir = tmp.path().join("institutions").join("test");
    std::fs::create_dir_all(&inst_dir).unwrap();
    std::fs::write(inst_dir.join("spec.yaml"), MOCK_SPEC_YAML).unwrap();
    std::fs::create_dir_all(inst_dir.join("template")).unwrap();
    std::fs::write(inst_dir.join("template").join("template.typ"), "hello").unwrap();

    let config = AppConfig {
        port: 0,
        catalog_path: tmp.path().to_path_buf(),
    };
    let registry = Registry::load(&config.catalog_path).unwrap();
    let router = routes::router(registry);
    (router, tmp)
}
```

Mock spec YAML (`MOCK_SPEC_YAML`):
```yaml
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
```

#### New Tests

| Test | Route | Asserts |
|------|-------|---------|
| `test_health_returns_ok` | `GET /health` | 200, body "ok" |
| `test_institutions_lists_ids` | `GET /institutions` | 200, JSON array with test institution entry |
| `test_spec_returns_yaml` | `GET /institutions/test/spec` | 200, contains "institution", "checks", automated_checks count |
| `test_spec_not_found` | `GET /institutions/nonexistent/spec` | 404, error JSON with "Institution not found" |
| `test_template_returns_files` | `GET /institutions/test/template` | 200, entry is "template.typ", files non-empty |
| `test_template_not_found` | `GET /institutions/nonexistent/template` | 404 |
| `test_extract_no_file` | `POST /extract` (empty body) | 500, error message contains "No file" |
| `test_compile_missing_institution` | `POST /compile` (missing `?institution=`) | 500 |
| `test_validate_invalid_base64` | `POST /validate` (garbage base64) | 500, error message contains "Invalid base64" |
| `test_validate_missing_institution` | `POST /validate` (valid base64, missing institution) | 404 |

Tests use `tower::ServiceExt` or `axum_test::TestServer` to send real HTTP requests to the router. No actual PDF/DOCX files needed for route-level tests — just verify status codes, error messages, and JSON structure.

Target: 10 new tests.

### 3.4 `sp-validate` — Untested Checkers

**Status**: 8 of 36 checkers have zero tests. Concentrated in `sections.rs` (4), `structure.rs` (3), `content.rs` (1).

All new tests follow the existing pattern: construct a mock `Document` with specific `TextSpan` data, assert the checker returns `Pass` or `Fail` as expected.

#### `sections.rs` (4 untested)

| Test | Checker | Asserts |
|------|---------|---------|
| `test_references_heading_font_mismatch` | `ReferencesHeadingChecker` | References section heading has different font from body → FAIL |
| `test_cv_heading_font_mismatch` | `CvHeadingChecker` | CV heading font differs from body → FAIL |
| `test_cv_name_not_found` | `CvNamePositionChecker` | Name text missing from CV page → FAIL |
| `test_abstract_centered_pass` | `AbstractTextCenteredChecker` | Abstract text centered on page → PASS |

#### `structure.rs` (3 untested)

| Test | Checker | Asserts |
|------|---------|---------|
| `test_acceptance_page_has_no_page_number` | `AcceptancePagePageNumberChecker` | Acceptance page has page number → FAIL (it should not) |
| `test_front_matter_arabic_page_number` | `PageNumbersFormatChecker` | Arabic numeral on page iii → FAIL |
| `test_headings_font_differs_from_body` | `HeadingsConsistentChecker` | Chapter heading "CHAPTER ONE" at 14pt when body is 12pt → FAIL |

#### `content.rs` (1 untested — `TocTitleParityChecker` already covered above; `CommitteeOrderChecker` is the gap)

| Test | Checker | Asserts |
|------|---------|---------|
| `test_committee_chair_not_first` | `CommitteeOrderChecker` | Non-chair name listed before chair on acceptance page → FAIL |

Target: 12 new tests.

### 3.5 `scholarpress-cli` — CLI Tests

**Status**: 0 tests.

#### New Tests

**Test: `test_check_subcommand_exit_zero`** — runs `scholarpress-cli check --spec <mock_spec> <fixture_pdf>` via `assert_cmd::Command`, asserts exit code 0:
```rust
#[test]
fn test_check_subcommand_exit_zero() {
    let mut cmd = assert_cmd::Command::cargo_bin("scholarpress-cli").unwrap();
    let spec = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(spec.path(), MOCK_SPEC_YAML).unwrap();

    cmd.arg("check")
       .arg("--spec").arg(spec.path())
       .arg(fixture_pdf_path())
       .assert()
       .success();
}
```

**Test: `test_calibrate_subcommand`** — runs `calibrate --spec <spec> --corpus <dir>` with a temp corpus dir containing one PDF, asserts exit code 0 and stdout is non-empty.

Adds `assert_cmd` and `tempfile` as dev-dependencies to the CLI crate.

Target: 2 new tests.

---

## 4. Catalog Tests (`scholarpress-catalog`)

Catalog testing assumes the backend engine works. Validates that the data (spec YAML, Typst templates, fixture PDFs) accurately reflects institutional policies. Uses the pre-built Docker image to invoke the CLI.

### 4.1 Shared Fixtures (Recycle)

Copy the 10 margin fixture PDFs into `sp-validate/tests/fixtures/margins/` alongside `synthetic-body.typ`. These become hermetic test assets for the engine tests in 3.4.

The catalog retains its original copies at `institutions/iu/tests/fixtures/`. Backend engine tests use the copies.

### 4.2 `expected_results.yaml`

**File**: `institutions/iu/tests/expected_results.yaml`

```yaml
fixtures:
  baseline.pdf:
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

  golden.pdf:
    assert_fails: []
    assert_passes: ["ALL"]
    ignore_others: true
```

**`"ALL"` semantics**: The runner parses `spec.yaml`, extracts all check IDs where `automatable: true`, and asserts each returns PASS (or is skipped due to scope — e.g., a check targeting the dedication page when the fixture has no dedication). Only an explicit FAIL triggers assertion failure. Manual-review checks are excluded.

**`ignore_others: true`**: Checks not listed in `assert_fails` or `assert_passes` are not asserted against. Adding a new check to the spec does not break existing fixture expectations.

### 4.3 `validate_fixtures.sh`

**File**: `institutions/iu/tests/validate_fixtures.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
IMAGE="ghcr.io/scholarpress-workshop/scholarpress-publish-service:latest"
CATALOG_MOUNT="/catalog"

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
if not results:
    sys.exit(1)
if results[0]['status'] not in ('FAIL', 'ERROR'):
    sys.exit(1)
" 2>/dev/null; then
    echo "  [PASS] $check_id fails as expected"
  else
    echo "  [FAIL] Expected $check_id to FAIL in $pdf"
    return 1
  fi
}

assert_passes() {
  local pdf="$1" check_id="$2"
  local output
  output=$(run_check "$pdf")
  if echo "$output" | python3 -c "
import sys, json
data = json.load(sys.stdin)
results = [r for r in data.get('results', []) if r['check_id'] == '$check_id']
if not results:
    sys.exit(1)
if results[0]['status'] not in ('PASS', 'MANUAL', 'SKIPPED'):
    sys.exit(1)
" 2>/dev/null; then
    echo "  [PASS] $check_id passes as expected"
  else
    echo "  [FAIL] Expected $check_id to PASS in $pdf"
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
    echo "  [PASS] All automatable checks pass"
  else
    echo "  [FAIL] Unexpected failures in $pdf: $failures"
    return 1
  fi
}

echo "=== Catalog fixture validation ==="
echo

PASS_COUNT=0
FAIL_COUNT=0

for pdf in "$DIR/fixtures"/*.pdf; do
  name=$(basename "$pdf")
  echo "--- $name ---"

  # Parse expected_results.yaml (simplified: hardcoded for now,
  # full YAML parsing via python script in future)
  case "$name" in
    baseline.pdf|golden.pdf)
      assert_all_pass "$name" && ((PASS_COUNT++)) || ((FAIL_COUNT++))
      ;;
    left-narrow.pdf|right-narrow.pdf|left-wide.pdf|right-wide.pdf|top-narrow.pdf|bottom-narrow.pdf|top-wide.pdf)
      assert_fails "$name" "global_margins" && ((PASS_COUNT++)) || ((FAIL_COUNT++))
      ;;
    asymmetric.pdf)
      assert_fails "$name" "margin_symmetry" && ((PASS_COUNT++)) || ((FAIL_COUNT++))
      ;;
    messy.pdf)
      echo "  [SKIP] messy.pdf — smoke test only"
      ((PASS_COUNT++))
      ;;
    *)
      echo "  [SKIP] No expected results defined for $name"
      ;;
  esac
  echo
done

echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="
if [ "$FAIL_COUNT" -gt 0 ]; then
  exit 1
fi
```

The initial implementation hardcodes expected results to keep the script simple. A future iteration can parse `expected_results.yaml` via a Python helper script for full schema-driven validation.

### 4.4 Dual-Purpose Docker Image (Prerequisite)

The publish-service Dockerfile must include both the Axum server binary and the CLI binary. Update the builder stage:

```dockerfile
RUN cargo build --release --bin publish-service --bin scholarpress-cli && \
    cp target/release/publish-service /app/publish-service && \
    cp target/release/scholarpress-cli /app/scholarpress-cli

FROM debian:bookworm-slim
COPY --from=builder /app/publish-service /usr/local/bin/publish-service
COPY --from=builder /app/scholarpress-cli /usr/local/bin/scholarpress
```

Now the image runs the server by default but can be invoked as a CLI:
```bash
docker run ... ghcr.io/.../publish-service:latest \
    scholarpress check --spec /catalog/iu/spec.yaml document.pdf
```

### 4.5 Golden Baseline PDF

Generate `institutions/iu/tests/fixtures/golden.pdf` by compiling the institution's Typst template at `institutions/iu/template/template.typ`. Add to `compile.sh`:

```bash
# Golden baseline: compile the institution template
echo "=== Generating golden baseline ==="
typst compile --root "$ROOT/../template" \
  "$ROOT/../template/template.typ" \
  "$DIR/golden.pdf"
```

This PDF must pass all automatable checks when validated against the spec.

---

## 5. Execution Order

| Phase | Tasks | Tests Added | Depends On |
|-------|-------|-------------|------------|
| **P0-A** | 3.1 sp-extract core tests | 12 | — |
| **P0-B** | 3.2 sp-typst compile tests | 2 | — |
| **P0-C** | 3.3 publish-service route tests | 10 | 3.1 (extraction) |
| **P1-A** | 3.4 sp-validate untested checkers | 12 | 3.1 (fixture copies) |
| **P1-B** | 4.1 Copy fixtures to backend, 4.2 expected_results.yaml, 4.4 Dockerfile update | — | — |
| **P1-C** | 4.5 Golden baseline, 4.3 validate_fixtures.sh | — | 4.4 (Docker image) |
| **P2** | 3.5 scholarpress-cli tests, 2.x non-layout violation fixtures | 2 + N | P1 |

**Total**: 38 backend unit/integration tests + catalog validation pipeline.

---

## 6. Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Backend fixture access | Copy into crate `tests/fixtures/` | Hermetic — no sibling directory dependency |
| Catalog CI invocation | Docker image | Stable, no platform-specific binary downloads |
| expected_results schema | Static, partial assertions | Adding a check doesn't break unrelated fixture assertions |
| `"ALL"` semantics | All automatable checks, exclude manual | CI can't assert human review outcomes |
| Check ID naming | Actual spec IDs (`global_margins`) | No translation layer between test schema and engine output |
