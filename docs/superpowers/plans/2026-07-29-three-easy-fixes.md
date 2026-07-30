# Three Easy Fixes — Post-Test Triage

## Fix 1 — catalog#1: Document template function conventions

**Repo:** `scholarpress-catalog`

**Current state:** 17 section files in `institutions/iu/template/sections/`, zero doc comments. Every function uses named parameters (`heading:`, `body:`, `author:`, etc.) but there is no indication of this convention anywhere.

**Change:** Add one doc comment to each section `.typ` file just above the function definition.

Pattern to add (example for `abstract.typ`):
```typst
/// Renders the abstract page.
///
/// All IU template functions use named parameters. Call as:
///   #abstract-page(heading: "Abstract", author: "Name", title: "Title", body: [...], committee: ())
/// - `body:` is a content block, e.g. `body: [Abstract text here.]`
/// - `committee:` is an array of dictionaries with keys `name`, `degree`, `role`
```

Top-level convention doc in `template.typ`:
```typst
// Every section function in this template uses NAMED parameters with `body: [...]` for content.
// Do NOT use positional calling like `function()[...]` — this will fail with "unclosed delimiter".
```

**Files touched:** 18 files (template.typ + 17 section files). Pure doc comments, zero code impact, no tests needed.

**Verification:** `typst compile template.typ` still produces the same PDF.

---

## Fix 2 — backend#5: Expose `markdown_text` as a `format` option on `extract_document`

**Repo:** `scholarpress-backend`

**Current state:** `ParsedDocument.markdown_text: Option<String>` already exists and is serialized in the JSON output. The agent just doesn't know about it because the tool description says "JSON ParsedDocument (pages, paragraphs, headings, metadata)."

**Change (15 lines):**

1. Add `format` to `ExtractDocumentParams`:
```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractDocumentParams {
    pub file_path: PathBuf,
    pub format: Option<String>,  // "json" (default) or "markdown"
}
```

2. Update `workspace::extract_document` to accept and handle the format:
```rust
pub fn extract_document(file_path: &Path, format: Option<&str>) -> Result<serde_json::Value, SpMcpError> {
    // ... same file-reading logic ...
    let doc = match ext.as_str() { ... };

    match format.unwrap_or("json") {
        "markdown" => Ok(serde_json::Value::String(
            doc.markdown_text
                .ok_or_else(|| SpMcpError::Extraction(
                    "markdown not available for this format".into()
                ))?
        )),
        _ => serde_json::to_value(doc)
            .map_err(|e| SpMcpError::Extraction(e.to_string())),
    }
}
```

3. Update tool description to mention the markdown option.

4. Add test: extract a DOCX with `format: "markdown"` → returns non-empty string.

**Files touched:** `tools.rs` (params struct + description), `workspace.rs` (fn signature + dispatch), `workspace.rs` tests.

---

## Fix 3 — backend#4: Surface `evidence` in `CheckOutcome`

**Repo:** `scholarpress-backend`

**Current state:** `CheckResult.evidence: Vec<EvidenceItem>` (with `page`, `bbox`, `excerpt`) is available but thrown away. Only the first evidence item's page number is surfaced.

**Change (5 lines):**

1. Add `EvidenceDetail` struct (subset of `EvidenceItem`, serializable):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDetail {
    pub page: usize,
    pub excerpt: Option<String>,
}
```

2. Add `evidence` field to `CheckOutcome`:
```rust
pub struct CheckOutcome {
    pub id: String,
    pub status: String,
    pub message: String,
    pub page: Option<usize>,
    pub evidence: Vec<EvidenceDetail>,
}
```

3. Update the mapping in `check_pdf`:
```rust
.map(|r| CheckOutcome {
    id: r.check_id,
    status: r.status.as_str().to_string(),
    message: r.detail,
    page: r.evidence.first().map(|e| e.page),
    evidence: r.evidence.into_iter().map(|e| EvidenceDetail {
        page: e.page,
        excerpt: e.excerpt,
    }).collect(),
})
```

`bbox` omitted — page-space coordinates are not actionable for the LLM. `excerpt` (text that triggered the violation) is the useful field.

**Files touched:** `workspace.rs` (struct + mapping). Existing test still asserts non-empty outcomes; new field is additive.

---

## Execution order

1. catalog#1 (pure docs)
2. backend#5 (small code)
3. backend#4 (smallest code)

All independent. One commit each. ~30 min total.
