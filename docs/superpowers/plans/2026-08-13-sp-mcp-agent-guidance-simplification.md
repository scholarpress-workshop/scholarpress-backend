# `sp-mcp` Agent Guidance Simplification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Make the existing `sp-mcp` tools easier for agents to use by correcting their schemas, shortening server instructions, and fixing stale diagnostic paths.

**Architecture:** Preserve all existing MCP tool names, parameter names, return shapes, path-boundary checks, and document-processing behavior. Change only Rust doc comments/schema descriptions, the server instruction text, source-hint mappings, and the unused workspace directory creation after confirming it has no active consumers.

**Tech Stack:** Rust, `rmcp` tool schemas, `schemars::JsonSchema`, `serde_json`, `cargo test`, `cargo fmt`, and `cargo clippy`.

## Global Constraints

- Do not add JSON metadata transport to `compile_typst`.
- Do not add combined or stateful tools such as `compile_and_check` or `scaffold_entry`.
- Do not rename existing MCP tools or public parameters.
- Do not replace Typst body content with JSON.
- Do not change workspace path-boundary behavior.
- Do not change PDF checking behavior or the `CheckOutcome` output shape.
- Keep source hints advisory and preserve their `Vec<String>` output.
- Do not modify catalog files in this backend-only change.

---

## File Map

- Modify: `crates/sp-mcp/src/tools.rs` — input-field schema descriptions, tool descriptions, and concise server instructions.
- Modify: `crates/sp-mcp/src/workspace.rs` — source-hint paths and, if unused, removal of `data/` creation.
- Modify: `crates/sp-mcp/tests/http_transport.rs` — inspect the advertised tool list and server-facing descriptions if the existing transport test is the smallest stable seam.
- Modify: `crates/sp-mcp/src/workspace.rs` tests — assert corrected source hints and workspace directory layout.
- Modify: `README.md` only if it repeats the removed `data.json` workflow or stale tool guidance found during the implementation search.

## Task 1: Document Existing Tool Inputs

**Files:**
- Modify: `crates/sp-mcp/src/tools.rs:20-50`
- Test: `crates/sp-mcp/tests/http_transport.rs`

**Interfaces:**
- Consumes: existing `CreateWorkspaceParams`, `CompileTypstParams`, `CheckPdfParams`, `PandocConvertParams`, and `InterfaceDocParams` structures.
- Produces: unchanged Rust parameter types with MCP-visible field descriptions.

- [ ] **Step 1: Add field documentation to the existing parameter structs**

Add these exact doc comments without changing field types or names:

```rust
/// Workspace name to create under SCHOLARPRESS_WORKSPACE_ROOT.
pub name: String,
/// Catalog profile ID returned by list_profiles, such as institutions/iu-indianapolis.
pub profile_id: String,
```

```rust
/// Absolute workspace path returned by create_workspace.
pub workspace: PathBuf,
/// Entry file path relative to the workspace, normally entry.typ.
pub entry_path: PathBuf,
/// Output filename only; written below the workspace's out/ directory.
pub out_name: Option<String>,
```

```rust
/// Absolute workspace path returned by create_workspace.
pub workspace: PathBuf,
/// PDF path relative to the workspace, normally out/entry.pdf.
pub pdf_path: PathBuf,
/// Optional check IDs to run; omit to run every check.
pub check_ids: Option<Vec<String>>,
```

```rust
/// DOCX input path relative to the workspace.
pub file_path: PathBuf,
/// Conversion format: typst or ast.
pub format: String,
/// Absolute workspace path returned by create_workspace.
pub workspace: PathBuf,
```

```rust
/// Absolute workspace path returned by create_workspace.
pub workspace: PathBuf,
```

- [ ] **Step 2: Add an MCP schema regression assertion**

Extend the existing `http_transport_accepts_mcp_initialize` test or add a focused test in `http_transport.rs` that calls `tools/list`, parses the JSON-RPC response, and asserts the tool schema contains these descriptions:

```text
"Entry file path relative to the workspace, normally entry.typ."
"PDF path relative to the workspace, normally out/entry.pdf."
"Conversion format: typst or ast."
```

The test must also assert the existing tool names `compile_typst`, `check_pdf`, and `pandoc_convert` remain present.

- [ ] **Step 3: Run the focused transport test**

Run:

```bash
rtk cargo test -p sp-mcp --test http_transport
```

Expected: all transport tests pass and the schema descriptions are present.

## Task 2: Simplify Tool And Server Descriptions

**Files:**
- Modify: `crates/sp-mcp/src/tools.rs:65-176`
- Test: `crates/sp-mcp/tests/http_transport.rs`

**Interfaces:**
- Consumes: existing tool handlers and parameter types.
- Produces: unchanged tool behavior with shorter, accurate descriptions.

- [ ] **Step 1: Remove the unsupported `data.json` claim**

Change the `compile_typst` description to state only:

```text
Compile a Typst entry file within a workspace. Writes the PDF below <workspace>/out/ and returns its absolute path. The entry path is relative to the workspace. Requires the typst executable.
```

Do not add a replacement data parameter.

- [ ] **Step 2: Shorten the remaining tool descriptions**

Use these descriptions:

```text
List existing workspaces under SCHOLARPRESS_WORKSPACE_ROOT.
```

```text
List catalog profiles available to create_workspace.
```

```text
Create a workspace by copying a catalog profile into SCHOLARPRESS_WORKSPACE_ROOT. Returns the absolute workspace path.
```

```text
Run the workspace spec's formatting checks against a PDF. Optionally limit execution with check_ids.
```

```text
Convert a workspace DOCX to Typst source or pandoc JSON AST. Use typst or ast as the format. TOC text is more reliable than AST headings for section mapping.
```

```text
Return the template's generated REFERENCE.json with section function signatures, parameter types, defaults, and examples.
```

- [ ] **Step 3: Replace `get_info` instructions**

Replace the current long string with this concise instruction text:

```text
ScholarPress workflow: call list_profiles, create_workspace, and interface_doc first. Convert the DOCX with pandoc_convert(format: "ast") and pandoc_convert(format: "typst") as needed. Write entry.typ and chapter files, then call compile_typst and check_pdf. Use check_ids to isolate PDF failures. Pandoc output is best effort; use TOC text to map sections and clean Typst artifacts such as #underline[...] and #strong[...].
```

- [ ] **Step 4: Assert descriptions do not advertise removed behavior**

Extend the `tools/list` test to assert the returned descriptions do not contain `data.json` and the server instructions do not contain the old `Map–Scaffold–Migrate–Verify` block.

- [ ] **Step 5: Run the focused transport test**

Run:

```bash
rtk cargo test -p sp-mcp --test http_transport
```

Expected: all tests pass with the shortened descriptions.

## Task 3: Correct Source Hints

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs:399-447`
- Test: `crates/sp-mcp/src/workspace.rs` test module

**Interfaces:**
- Consumes: existing `source_hints(check_id: &str) -> Vec<String>` behavior.
- Produces: same return type and check-ID mapping with current catalog paths.

- [ ] **Step 1: Update stale template paths**

Use these mappings for the affected checks:

```rust
"title_clause_wording" => hint("template/sections/front-matter.typ"),
"title_page_no_bold" => hint("template/sections/front-matter.typ"),
"title_page_no_page_number" => hint("template/template.typ"),
"title_page_all_caps" => hints(&["template/sections/front-matter.typ", "entry.typ"]),
"clause_spacing" => hint("template/sections/front-matter.typ"),
"title_page_clause_centered" => hint("template/sections/front-matter.typ"),
"title_page_clause_spacing" => hint("template/sections/front-matter.typ"),
"abstract_text_centered" => hint("template/sections/front-matter.typ"),
"abstract_word_count" => hint("template/sections/front-matter.typ"),
"abstract_title_format" => hint("template/sections/front-matter.typ"),
"cv_no_credentials" => hint("template/sections/back-matter.typ"),
```

Update the other stale `chapters/` references as follows:

```rust
"headings_consistent" => hints(&["template/sections/chapter.typ", "chapters/"]),
"new_chapters_new_pages" => hints(&["template/sections/chapter.typ", "chapters/"]),
"references_font_consistent" => hint("template/sections/back-matter.typ"),
"references_heading_format" => hint("template/sections/back-matter.typ"),
"references_spacing" => hint("template/sections/back-matter.typ"),
"cv_heading_format" => hint("template/sections/back-matter.typ"),
"cv_no_page_number" => hint("template/sections/back-matter.typ"),
```

Keep `entry.typ`, `chapters/`, `template/styles.typ`, and `template/template.typ` hints where they remain accurate.

- [ ] **Step 2: Add a source-hint regression test**

Add a unit test in the existing `workspace.rs` test module:

```rust
#[test]
fn source_hints_use_current_template_paths() {
    let ids = [
        "title_clause_wording",
        "abstract_title_format",
        "headings_consistent",
        "references_heading_format",
        "cv_heading_format",
    ];
    for id in ids {
        let hints = source_hints(id);
        assert!(hints.iter().all(|hint| !hint.contains("chapters/title-page.typ")));
        assert!(hints.iter().all(|hint| !hint.contains("chapters/abstract.typ")));
        assert!(hints.iter().all(|hint| !hint.contains("chapters/cv.typ")));
    }
}
```

- [ ] **Step 3: Run the focused workspace tests**

Run:

```bash
rtk cargo test -p sp-mcp source_hints_use_current_template_paths
```

Expected: PASS.

## Task 4: Remove Unused `data/` Workspace Creation

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs:163-170`
- Modify: `crates/sp-mcp/src/workspace.rs` workspace creation tests

**Interfaces:**
- Consumes: `create_workspace` behavior.
- Produces: workspaces containing `spec.yaml`, `template/`, and `out/`, without an unused `data/` directory.

- [ ] **Step 1: Confirm no active `data/` consumers**

Run from the backend repository:

```bash
rtk grep 'workspace.*data|join\("data"\)|data\.json' crates docs README.md tests
```

Only historical plans or unsupported documentation may match. If active Rust code or tests match, retain the directory and document that no removal is safe.

- [ ] **Step 2: Add the minimal workspace-layout assertion**

In the existing `create_workspace` test, assert the result contains `out/` and does not contain `data/`:

```rust
assert!(created.join("out").is_dir());
assert!(!created.join("data").exists());
```

- [ ] **Step 3: Remove the unused directory creation**

Delete only this operation from `create_workspace`:

```rust
std::fs::create_dir_all(target.join("data"))?;
```

Keep `std::fs::create_dir_all(target.join("out"))?;` unchanged.

- [ ] **Step 4: Run workspace tests**

Run:

```bash
rtk cargo test -p sp-mcp workspace::tests
```

Expected: all workspace tests pass.

## Task 5: Full Verification

**Files:**
- Modify: only files listed in Tasks 1-4, plus `README.md` if the implementation search finds active duplicate guidance.

**Interfaces:**
- Consumes: corrected MCP schemas, descriptions, instructions, hints, and workspace layout.
- Produces: a tested `sp-mcp` build with no stale agent guidance.

- [ ] **Step 1: Run the complete `sp-mcp` test suite**

Run:

```bash
rtk cargo test -p sp-mcp
```

Expected: PASS.

- [ ] **Step 2: Run formatting and lint checks**

Run:

```bash
cargo fmt --all
rtk cargo clippy --all --tests -- -D warnings
```

Expected: clean formatting and no Clippy warnings.

- [ ] **Step 3: Search for stale guidance**

Run:

```bash
rtk grep 'data\.json|chapters/title-page\.typ|chapters/abstract\.typ|chapters/cv\.typ|Map–Scaffold–Migrate–Verify' crates/sp-mcp README.md
```

Expected: no active `sp-mcp` source or README matches. Historical design documents may retain old implementation notes.

- [ ] **Step 4: Review the final diff**

Run:

```bash
rtk git diff --check
rtk git status --short
rtk git diff --stat
```

Confirm the diff contains only agent guidance, schema descriptions, source hints, workspace directory cleanup, and tests.
