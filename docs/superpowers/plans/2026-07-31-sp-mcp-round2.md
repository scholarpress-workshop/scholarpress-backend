# sp-mcp Round 2: Pandoc Pipeline + Prompting Strategy — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `extract_document` MCP tool with a pandoc-based `pandoc_convert` tool that writes output to files, and update the server instructions with a Map–Scaffold–Migrate–Verify prompting strategy.

**Architecture:** `pandoc_convert` shells out to `/usr/bin/pandoc` (PATH discovery, same pattern as `typst`/`typstyle`). Writes output to `<workspace>/out/<stem>.<ext>` and returns the path — no inline content, no truncation. `extract_document` is removed from the MCP tool surface; `sp-extract` crate stays as `sp-check`'s internal dependency but is dropped from `sp-mcp`'s Cargo.toml.

**Tech Stack:** Rust (rmcp v3, serde, tokio), pandoc 3.7 on PATH, Typst catalog templates

## Global Constraints

- Pandoc 3.7.0.2 on PATH (no compile-time dep)
- Only `.docx` input supported initially
- Output written to `<workspace>/out/` — same convention as `compile_typst`
- All shell-outs follow existing `std::process::Command` pattern from `workspace.rs`

---

### Task 1: Remove `extract_document` from MCP tool surface

**Files:**
- Modify: `scholarpress-backend/crates/sp-mcp/src/tools.rs:39-43,140-152`
- Modify: `scholarpress-backend/crates/sp-mcp/src/workspace.rs:377-410,610-640`
- Modify: `scholarpress-backend/crates/sp-mcp/Cargo.toml:19`

**Interfaces:**
- Consumes: nothing new
- Produces: clean removal — `extract_document` tool, params struct, workspace function, and test functions are gone

- [ ] **Step 1: Remove `ExtractDocumentParams` struct from tools.rs**

Delete lines 39-43:
```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractDocumentParams {
    pub file_path: PathBuf,
    pub format: Option<String>, // "json" (default) or "markdown"
}
```

- [ ] **Step 2: Remove `extract_document` tool method from tools.rs**

Delete lines 140-152 (the entire `extract_document` method including the `#[tool(...)]` attribute, function signature, and body).

- [ ] **Step 3: Remove `extract_document` function from workspace.rs**

Delete lines 377-410:
```rust
use sp_extract as extract;

pub fn extract_document(file_path: &Path, format: Option<&str>) -> Result<serde_json::Value, SpMcpError> {
    ...
}
```

- [ ] **Step 4: Remove `extract_document` tests from workspace.rs**

Delete lines 610-640 — the three test functions:
- `extract_document_on_garbage_returns_error`
- `extract_document_unsupported_extension_errors`
- `extract_document_format_param_routes_correctly`

- [ ] **Step 5: Remove `sp-extract` from Cargo.toml**

Delete line 19:
```toml
sp-extract = { path = "../sp-extract" }
```

Note: `sp-extract` stays in the workspace as a dependency of `sp-check`. We only remove it from `sp-mcp`.

- [ ] **Step 6: Build to verify clean removal**

```bash
cargo build -p sp-mcp 2>&1
```

Expected: `cargo build` succeeds with no unused-import or missing-dependency errors.

- [ ] **Step 7: Commit**

```bash
git add scholarpress-backend/crates/sp-mcp/src/tools.rs scholarpress-backend/crates/sp-mcp/src/workspace.rs scholarpress-backend/crates/sp-mcp/Cargo.toml
git commit -m "feat(sp-mcp): remove extract_document MCP tool, drop sp-extract dep"
```

---

### Task 2: Add `pandoc_convert` MCP tool

**Files:**
- Modify: `scholarpress-backend/crates/sp-mcp/src/error.rs:28-29`
- Modify: `scholarpress-backend/crates/sp-mcp/src/workspace.rs` (add function, add tests)
- Modify: `scholarpress-backend/crates/sp-mcp/src/tools.rs` (add params struct + tool method)

**Interfaces:**
- Consumes: nothing (new code)
- Produces: `pandoc_convert(file_path: PathBuf, format: String, workspace: PathBuf) -> Result<PathBuf, SpMcpError>` — writes output file to `<workspace>/out/<stem>.<ext>`, returns absolute path

- [ ] **Step 1: Add `Conversion` error variant to error.rs**

Insert after line 28 (`Extraction(String),`):
```rust
    #[error("conversion failed: {0}")]
    Conversion(String),
```

- [ ] **Step 2: Add `PandocConvertParams` struct to tools.rs**

Insert after the `CheckTypstParams` struct (after line 49):

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PandocConvertParams {
    pub file_path: PathBuf,
    pub format: String, // "typst" or "ast"
    pub workspace: PathBuf,
}
```

- [ ] **Step 3: Add `pandoc_convert` function to workspace.rs**

Insert before the `#[cfg(test)]` block (insert as a new section before line 412):

```rust
pub fn pandoc_convert(
    file_path: &Path,
    format: &str,
    workspace: &Path,
) -> Result<PathBuf, SpMcpError> {
    if !file_path.is_file() {
        return Err(SpMcpError::Conversion(format!(
            "file not found: {}",
            file_path.display()
        )));
    }

    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "docx" {
        return Err(SpMcpError::Conversion(format!(
            "unsupported extension: .{} (only .docx supported)",
            ext
        )));
    }

    let to_format = match format {
        "typst" => "typst",
        "ast" => "json",
        other => {
            return Err(SpMcpError::Conversion(format!(
                "unsupported format: {other} (use \"typst\" or \"ast\")"
            )));
        }
    };

    let stem = file_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "document".to_string());

    let out_ext = if format == "ast" { "json" } else { "typst" };
    let out_name = format!("{stem}.{out_ext}");

    std::fs::create_dir_all(workspace.join("out"))?;
    let out_path = workspace.join("out").join(&out_name);

    let output = std::process::Command::new("pandoc")
        .arg(file_path)
        .arg("--from")
        .arg("docx")
        .arg("--to")
        .arg(to_format)
        .arg("--output")
        .arg(&out_path)
        .output()
        .map_err(|e| {
            SpMcpError::Conversion(format!("failed to run pandoc: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SpMcpError::Conversion(format!(
            "pandoc failed: {stderr}"
        )));
    }

    if !out_path.is_file() {
        return Err(SpMcpError::Conversion(format!(
            "pandoc produced no output at {out_name}"
        )));
    }

    Ok(out_path)
}
```

- [ ] **Step 4: Add `pandoc_convert` tool method to tools.rs**

Insert before the `check_typst` method (insert at line 153, after the `check_pdf` method closing brace):

```rust
    #[tool(
        description = "Convert a DOCX file to Typst or pandoc JSON AST. Writes output to <workspace>/out/<stem>.typ (for format: \"typst\") or <workspace>/out/<stem>.json (for format: \"ast\"). Returns the absolute output path. Requires `pandoc` on PATH (already installed).\n\nAST headings are unreliable — many DOCX files use direct formatting instead of heading styles. Prefer TOC text over AST for section boundaries."
    )]
    async fn pandoc_convert(
        &self,
        params: Parameters<PandocConvertParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let out = workspace::pandoc_convert(&p.file_path, &p.format, &p.workspace)
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            out.display().to_string(),
        )]))
    }
```

- [ ] **Step 5: Add `pandoc_convert` tests to workspace.rs**

Insert after the `extract_document` tests are removed (inserting where they were deleted, in the `#[cfg(test)] mod tests` block):

```rust
    #[test]
    fn pandoc_convert_docx_to_typst_or_skips_if_no_pandoc() {
        let catalog = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.join("scholarpress-catalog"))
            .filter(|p| p.is_dir());
        let catalog = match catalog {
            Some(p) => p,
            None => {
                eprintln!("SKIP: scholarpress-catalog not found");
                return;
            }
        };
        let baseline = catalog.join("institutions/iu/tests/fixtures/baseline.docx");
        if !baseline.is_file() {
            eprintln!("SKIP: baseline.docx fixture not present");
            return;
        }

        let ws = local_tempdir();
        let result = pandoc_convert(&baseline, "typst", &ws);
        match result {
            Ok(out) => {
                assert!(out.is_file(), "typst output should exist");
                let content = fs::read_to_string(&out).unwrap();
                assert!(!content.is_empty(), "typst output should not be empty");
            }
            Err(SpMcpError::Conversion(msg))
                if msg.contains("pandoc") =>
            {
                eprintln!("SKIP: pandoc not on PATH ({})", msg);
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn pandoc_convert_unsupported_extension_errors() {
        let f = local_tempdir().join("foo.xyz");
        fs::write(&f, b"whatever").unwrap();
        let ws = local_tempdir();
        let result = pandoc_convert(&f, "typst", &ws);
        assert!(matches!(result, Err(SpMcpError::Conversion(_))));
    }

    #[test]
    fn pandoc_convert_unsupported_format_errors() {
        let f = local_tempdir().join("test.docx");
        // Not a real DOCX but the format check happens first
        fs::write(&f, b"fake").unwrap();
        let ws = local_tempdir();
        let result = pandoc_convert(&f, "xml", &ws);
        assert!(matches!(result, Err(SpMcpError::Conversion(_))));
    }

    #[test]
    fn pandoc_convert_missing_file_errors() {
        let ws = local_tempdir();
        let result = pandoc_convert(Path::new("/nonexistent/foo.docx"), "typst", &ws);
        assert!(matches!(result, Err(SpMcpError::Conversion(_))));
    }
```

- [ ] **Step 6: Build and run tests**

```bash
cargo build -p sp-mcp 2>&1
cargo test -p sp-mcp 2>&1
```

Expected: build succeeds, all new tests pass (or skip with reasonable messages for missing pandoc/fixtures). Existing tests continue to pass.

- [ ] **Step 7: Commit**

```bash
git add scholarpress-backend/crates/sp-mcp/src/error.rs scholarpress-backend/crates/sp-mcp/src/workspace.rs scholarpress-backend/crates/sp-mcp/src/tools.rs
git commit -m "feat(sp-mcp): add pandoc_convert tool for docx-to-typst conversion"
```

---

### Task 3: Update server instructions with Map–Scaffold–Migrate–Verify

**Files:**
- Modify: `scholarpress-backend/crates/sp-mcp/src/tools.rs:186-188`

**Interfaces:**
- Consumes: nothing
- Produces: updated `get_info()` instructions string

- [ ] **Step 1: Replace the instructions string in `get_info()`**

Change lines 186-188 from:
```rust
            .with_instructions(
                "ScholarPress: catalog + Typst template workspace tools. Use list_profiles to discover profiles, create_workspace to fork one into a scratch dir, then harness tools to edit, compile_typst + check_pdf to iterate.",
            )
```

To:
```rust
            .with_instructions(
                "ScholarPress: catalog + Typst template workspace tools. Use list_profiles to discover profiles, create_workspace to fork one into a scratch dir, then edit, compile_typst + check_pdf.\n\nWORKFLOW — Map–Scaffold–Migrate–Verify:\n\nMap — pandoc_convert(format: \"ast\") to survey structure, then scan pandoc_convert(format: \"typst\") output for Table of Contents. AST headings are unreliable (most DOCX uses direct formatting, not heading styles). The TOC is the source of truth for section count, order, and boundaries.\n\nScaffold — Create entry file with sections wired per template.typ comments (NAMED parameters, import pattern, chapter per-file convention).\n\nMigrate — One section at a time from the TOC: keyword-match the section title in pandoc typst output to find start boundary, keyword-match next section title for end boundary, slice the chunk, copy into the corresponding template section function. Run check_typst/format_typst on each section file, then compile_typst to catch errors early.\n\nVerify — compile_typst + check_pdf per milestone. Iterate incrementally.",
            )
```

- [ ] **Step 2: Build to verify**

```bash
cargo build -p sp-mcp 2>&1
```

Expected: build succeeds. String literals are well-formed.

- [ ] **Step 3: Commit**

```bash
git add scholarpress-backend/crates/sp-mcp/src/tools.rs
git commit -m "feat(sp-mcp): add Map-Scaffold-Migrate-Verify instructions to server info"
```

---

### Task 4: Update IU catalog template comments

**Files:**
- Modify: `scholarpress-catalog/institutions/iu/template/template.typ:7-10`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/toc.typ:1-3`

**Interfaces:**
- Consumes: nothing
- Produces: improved developer-facing comments in template files

- [ ] **Step 1: Add import/call example to template.typ**

Insert after line 10 (`//   Content blocks are passed via `body: [...]` parameters.`):

```typ
//
//   Example entry file wiring:
//     #import "template/template.typ": title-page, dedication-page, toc-page,
//       abstract-page, acknowledgements-page, preface-page, chapter,
//       references-page, curriculum-vitae
//
//     #title-page()
//     #dedication-page(body: [dedication text])
//     #toc-page(entries: toc_data)
//     #abstract-page(body: [abstract text])
//     ...
//
//   $ IN PROSE — $ starts Typst math mode. Dollar amounts, grant IDs, and
//   any prose containing $ must use \$ to escape (e.g., \$17 million).
```

- [ ] **Step 2: Document TOC entries struct shape in toc.typ**

Insert between lines 1 and 2 (before `#let toc-page(`):

```typ
// TOC ENTRIES — array of dictionaries used in #toc-page(entries: ...)
//   ((level: 1, title: "Introduction", page: 1),
//    (level: 2, title: "Background", page: 4), ...)
//   .level — integer heading level (1 = chapter, 2 = section)
//   .title — string heading text
//   .page  — integer page number
```

- [ ] **Step 3: Commit**

```bash
git add scholarpress-catalog/institutions/iu/template/template.typ scholarpress-catalog/institutions/iu/template/sections/toc.typ
git commit -m "docs(iu-catalog): add section wiring example and TOC struct docs"
```

---

### Task 5: Full build + manual verification

**Files:**
- None (verification only)

**Interfaces:**
- Consumes: completed Tasks 1-4
- Produces: verified working build

- [ ] **Step 1: Full release build**

```bash
cargo build --release -p sp-mcp 2>&1
```

Expected: build succeeds with no warnings.

- [ ] **Step 2: Run full test suite**

```bash
cargo test -p sp-mcp 2>&1
```

Expected: all tests pass (pandoc tests may skip if no pandoc on PATH, baseline tests may skip if no fixtures).

- [ ] **Step 3: Manual smoke test — list MCP tools**

After restarting the MCP server (copy the release binary or restart via OpenCode's MCP config), verify the tool list:

```bash
# Restart the MCP server, then ask "what tools does scholarpress provide?"
```

Expected: `list_profiles`, `list_workspaces`, `create_workspace`, `compile_typst`, `check_pdf`, `check_typst`, `format_typst`, `pandoc_convert` — 8 tools. `extract_document` is absent.

- [ ] **Step 4: Manual smoke test — pandoc_convert on dissertation**

```bash
# Via the MCP tool:
# pandoc_convert(file_path: "~/TRUNC - Hall dissertation 2026.docx", format: "typst", workspace: "<any workspace>")
```

Expected: returns path to `<workspace>/out/TRUNC - Hall dissertation 2026.typ`. File exists and contains valid Typst content.

- [ ] **Step 5: Manual smoke test — pandoc_convert AST mode**

```bash
# Via the MCP tool:
# pandoc_convert(file_path: "~/TRUNC - Hall dissertation 2026.docx", format: "ast", workspace: "<any workspace>")
```

Expected: returns path to `<workspace>/out/TRUNC - Hall dissertation 2026.json`. File exists and contains valid pandoc JSON AST.

- [ ] **Step 6: Run sp-check tests to confirm no regressions**

```bash
cargo test -p sp-check 2>&1
```

Expected: all tests pass. `sp-extract` removal from `sp-mcp` Cargo.toml does not affect `sp-check`.

- [ ] **Step 7: Commit (if any fixes from verification)**

No changes expected from verification. If fixes were needed, commit them.
