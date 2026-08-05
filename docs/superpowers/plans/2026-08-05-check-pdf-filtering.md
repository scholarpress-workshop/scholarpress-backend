# check_pdf check_id filtering — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional `check_ids` param to `check_pdf` MCP tool so agents can re-run specific checks against isolated section PDFs.

**Architecture:** Replace `CheckOptions.check_id: Option<String>` with `check_ids: Option<Vec<String>>` in the check engine; pass through from the MCP tool param; add a Debug subsection to `get_info()` instructions. No new types, no new dependencies.

**Tech Stack:** Rust (existing `sp-check`, `sp-mcp` crates), no new deps.

## Global Constraints

- `cargo fmt --all && cargo clippy --all --tests -- -D warnings` must pass before push
- All existing tests must pass unchanged
- Backward compatible: omitting `check_ids` = all checks run (current behavior)
- Unknown check IDs → empty results, not an error

---

### Task 1: Rename `CheckOptions.check_id` to `check_ids: Option<Vec<String>>`

**Files:**
- Modify: `crates/sp-check/src/engine.rs:1-68`
- Verify: `crates/sp-check/src/calibration.rs:108` (caller uses `::default()`)

**Interfaces:**
- Produces: `CheckOptions { check_ids: Option<Vec<String>>, category: Option<String> }`
- `run_checks` filters by set membership when `check_ids` is `Some` and non-empty

- [ ] **Step 1: Edit `CheckOptions` struct**

Change line 7-8 of `crates/sp-check/src/engine.rs` from:

```rust
#[derive(Default)]
pub struct CheckOptions {
    pub check_id: Option<String>,
    pub category: Option<String>,
}
```

to:

```rust
#[derive(Default)]
pub struct CheckOptions {
    pub check_ids: Option<Vec<String>>,
    pub category: Option<String>,
}
```

- [ ] **Step 2: Edit `run_checks` filter logic**

Change lines 21-26 of `crates/sp-check/src/engine.rs` from:

```rust
        if let Some(ref filter_id) = options.check_id {
            if check_def.id != *filter_id {
                continue;
            }
        }
```

to:

```rust
        if let Some(ref ids) = options.check_ids {
            if !ids.is_empty() && !ids.contains(&check_def.id) {
                continue;
            }
        }
```

- [ ] **Step 3: Verify calibration.rs needs no changes**

The only other caller of `CheckOptions` is `crates/sp-check/src/calibration.rs:108`:

```rust
let results = run_checks(&spec, &path, &CheckOptions::default())?;
```

Since `::default()` populates `check_ids: None` (same behavior as before), this line is unchanged.

- [ ] **Step 4: Build and test sp-check**

Run: `cargo test -p sp-check`
Expected: all tests pass, no compilation errors.

- [ ] **Step 5: Run fmt + clippy**

Run: `cargo fmt --all && cargo clippy --all --tests -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/sp-check/src/engine.rs
git commit -m "refactor: rename CheckOptions.check_id to check_ids (Vec) for multi-check filtering"
```

---

### Task 2: Wire `check_ids` through `check_pdf` in workspace.rs

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs:332-389`

**Interfaces:**
- Consumes: `CheckOptions { check_ids: Option<Vec<String>>, .. }` from Task 1
- Produces: `check_pdf` function gains `check_ids: Option<&[String]>` parameter

- [ ] **Step 1: Add `check_ids` parameter to `check_pdf` function signature**

Change line 332-336 of `crates/sp-mcp/src/workspace.rs` from:

```rust
pub fn check_pdf(
    _config: &Config,
    workspace: &Path,
    pdf_path: &Path,
) -> Result<Vec<CheckOutcome>, SpMcpError> {
```

to:

```rust
pub fn check_pdf(
    _config: &Config,
    workspace: &Path,
    pdf_path: &Path,
    check_ids: Option<&[String]>,
) -> Result<Vec<CheckOutcome>, SpMcpError> {
```

- [ ] **Step 2: Populate `CheckOptions` with `check_ids`**

Change line 361 from:

```rust
    let options = check::engine::CheckOptions::default();
```

to:

```rust
    let options = check::engine::CheckOptions {
        check_ids: check_ids.map(|ids| ids.to_vec()),
        ..Default::default()
    };
```

- [ ] **Step 3: Build sp-mcp**

Run: `cargo build -p sp-mcp`
Expected: no compilation errors.

- [ ] **Step 4: Run fmt + clippy**

Run: `cargo fmt --all && cargo clippy --all --tests -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/sp-mcp/src/workspace.rs
git commit -m "feat(sp-mcp): wire check_ids param through check_pdf to CheckOptions"
```

---

### Task 3: Add `check_ids` to MCP tool params and `get_info()` debug instructions

**Files:**
- Modify: `crates/sp-mcp/src/tools.rs:33-37` (CheckPdfParams), `tools.rs:131-144` (check_pdf handler), `tools.rs:205-208` (get_info instructions)

**Interfaces:**
- Consumes: `check_pdf(config, workspace, pdf_path, check_ids)` from Task 2
- Produces: `CheckPdfParams { workspace, pdf_path, check_ids: Option<Vec<String>> }`

- [ ] **Step 1: Add `check_ids` to `CheckPdfParams`**

Add `pub check_ids: Option<Vec<String>>` to the struct on line 33-37 of `crates/sp-mcp/src/tools.rs`:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckPdfParams {
    pub workspace: PathBuf,
    pub pdf_path: PathBuf,
    pub check_ids: Option<Vec<String>>,
}
```

- [ ] **Step 2: Pass `check_ids` to `workspace::check_pdf`**

Change line 139-140 of `crates/sp-mcp/src/tools.rs` from:

```rust
        let outcomes =
            workspace::check_pdf(&self.config, &p.workspace, &p.pdf_path).map_err(Self::err)?;
```

to:

```rust
        let outcomes =
            workspace::check_pdf(&self.config, &p.workspace, &p.pdf_path, p.check_ids.as_deref()).map_err(Self::err)?;
```

- [ ] **Step 3: Update tool description**

Change line 132 from:

```rust
        description = "Run formatting checks against the workspace's spec.yaml. Always uses the workspace spec. Returns a list of check outcomes (id, status, message, page)."
```

to:

```rust
        description = "Run formatting checks against the workspace's spec.yaml. Always uses the workspace spec. Returns a list of check outcomes (id, status, message, page, source_hints). Optionally filter to specific check IDs via check_ids param."
```

- [ ] **Step 4: Add Debug subsection to `get_info()` instructions**

In `get_info()` at `tools.rs:205-208`, insert the Debug subsection after the existing Verify text. The Verify text ends with `"Iterate incrementally."`. Add a newline and the Debug block:

The current instructions string ends with:

```
"Verify — compile_typst + check_pdf per milestone. Iterate incrementally."
```

Change it to end with:

```
"Verify — compile_typst + check_pdf per milestone. Iterate incrementally.\n\nDebug — When check_pdf reports failures, use check_ids to isolate: source_hints on each violation suggest likely culprit files. To confirm, create a minimal .typ file in the workspace that imports only the suspect section function from template.typ, sets #set page(...) matching the spec, and calls the section. Then: compile_typst(isolate.typ) -> check_pdf(isolate.pdf, check_ids: [\"the_failing_check_id\"]). If the check still fails -> issue is in the template section or styles. If it passes -> issue is in how entry.typ wires or composes the section."
```

- [ ] **Step 5: Build and test sp-mcp**

Run: `cargo test -p sp-mcp`
Expected: all 21 tests pass.

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: all tests pass across all crates.

- [ ] **Step 7: Run fmt + clippy**

Run: `cargo fmt --all && cargo clippy --all --tests -- -D warnings`
Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add crates/sp-mcp/src/tools.rs
git commit -m "feat(sp-mcp): add check_ids filter param and debug isolation instructions"
```

- [ ] **Step 9: Push all commits to origin**

```bash
git push
```
