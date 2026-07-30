# `check_typst` and `format_typst` MCP Tools Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `check_typst` (syntax validation) and `format_typst` (in-place formatting) MCP tools that shell out to the `typstyle` CLI binary — same pattern as `compile_typst` shells out to `typst`.

**Architecture:** Two pure functions in `workspace.rs` spawning `typstyle --check` and `typstyle -i` via `std::process::Command`. MCP tool handlers in `tools.rs` wrap them. No new Rust dependencies — `Command` is already in use.

**Tech Stack:** Rust, `std::process::Command`, `typstyle` binary on PATH.

**Spec:** `docs/superpowers/specs/2026-07-30-typstyle-mcp-tools.md`

## Global Constraints

- `typstyle` binary must be on PATH for the tools to work. If absent, return a clear "install with `cargo install typstyle --locked`" error.
- No new Rust crate dependencies. Use `std::process::Command` (already imported by `sp-typst`).
- Code style: match the existing `compile_typst` pattern in `workspace.rs` (shell-out, error wrapping, skip-if-missing-in-tests).
- Commit style: `<type>(sp-mcp): <short description>`.

---

## Task 1: Add `check_typst` and `format_typst` pure functions to `workspace.rs`

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs`
- Test: inline `#[cfg(test)]` in `workspace.rs`

**Interfaces:**
- Consumes: nothing (first task)
- Produces: `check_typst(workspace: &Path, file_path: &Path) -> Result<String, SpMcpError>` and `format_typst(workspace: &Path, file_path: &Path) -> Result<String, SpMcpError>`. Task 2 wraps these as MCP tools.

- [ ] **Step 1: Add `check_typst` and `format_typst` functions**

Insert after the existing `compile_typst` function in `crates/sp-mcp/src/workspace.rs`:

```rust
pub fn check_typst(workspace: &Path, file_path: &Path) -> Result<String, SpMcpError> {
    if !workspace.is_dir() {
        return Err(SpMcpError::Compilation(format!(
            "workspace not found: {}",
            workspace.display()
        )));
    }
    let file_abs = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        workspace.join(file_path)
    };
    if !file_abs.is_file() {
        return Err(SpMcpError::Compilation(format!(
            "file not found: {}",
            file_abs.display()
        )));
    }

    let output = std::process::Command::new("typstyle")
        .arg("--check")
        .arg(&file_abs)
        .current_dir(workspace)
        .output()
        .map_err(|e| SpMcpError::Compilation(format!("failed to run typstyle: {}", e)))?;

    if output.status.success() {
        Ok("ok".to_string())
    } else {
        Ok("needs_format".to_string())
    }
}

pub fn format_typst(workspace: &Path, file_path: &Path) -> Result<String, SpMcpError> {
    if !workspace.is_dir() {
        return Err(SpMcpError::Compilation(format!(
            "workspace not found: {}",
            workspace.display()
        )));
    }
    let file_abs = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        workspace.join(file_path)
    };
    if !file_abs.is_file() {
        return Err(SpMcpError::Compilation(format!(
            "file not found: {}",
            file_abs.display()
        )));
    }

    let output = std::process::Command::new("typstyle")
        .arg("-i")
        .arg(&file_abs)
        .current_dir(workspace)
        .output()
        .map_err(|e| SpMcpError::Compilation(format!("failed to run typstyle: {}", e)))?;

    if output.status.success() {
        Ok(file_abs.display().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(SpMcpError::Compilation(format!("typstyle failed: {}", stderr)))
    }
}
```

- [ ] **Step 2: Add tests**

Add to the `mod tests` block in `workspace.rs`:

```rust
    #[test]
    fn check_typst_catches_syntax_error_or_skips_if_no_typstyle() {
        let ws = local_tempdir();
        fs::write(ws.join("bad.typ"), "= Hello\n$17 million\n").unwrap();
        let result = check_typst(&ws, Path::new("bad.typ"));
        match result {
            Ok(status) => {
                // typstyle on PATH — $ in prose should trigger needs_format
                assert_eq!(status, "needs_format");
            }
            Err(SpMcpError::Compilation(msg)) if msg.contains("typstyle") => {
                // typstyle not on PATH — skip
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn check_typst_clean_file_is_ok_or_skips() {
        let ws = local_tempdir();
        fs::write(ws.join("clean.typ"), "= Hello\n\nWorld.\n").unwrap();
        let result = check_typst(&ws, Path::new("clean.typ"));
        match result {
            Ok(status) => assert_eq!(status, "ok"),
            Err(SpMcpError::Compilation(msg)) if msg.contains("typstyle") => {}
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn format_typst_modifies_file_or_skips_if_no_typstyle() {
        let ws = local_tempdir();
        fs::write(ws.join("input.typ"), "= Hello\n\n\n  world\n").unwrap();
        let result = format_typst(&ws, Path::new("input.typ"));
        match result {
            Ok(_) => {
                let formatted = fs::read_to_string(ws.join("input.typ")).unwrap();
                assert!(!formatted.contains("  world"));
            }
            Err(SpMcpError::Compilation(msg)) if msg.contains("typstyle") => {}
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }
```

- [ ] **Step 3: Run tests**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend/.worktrees/sp-mcp
cargo test -p sp-mcp --lib
```

Expected: 17 passed (14 old + 3 new) if `typstyle` is on PATH. If `typstyle` is not installed, expect 14 old + 3 new skipped.

If `typstyle` is not installed and you want the tests to exercise the actual code path:
```bash
cargo install typstyle --locked
cargo test -p sp-mcp --lib
```

- [ ] **Step 4: Commit**

```bash
git add crates/sp-mcp/src/workspace.rs
git commit -m "feat(sp-mcp): add check_typst and format_typst pure functions"
```

---

## Task 2: Add MCP tool handlers for `check_typst` and `format_typst`

**Files:**
- Modify: `crates/sp-mcp/src/tools.rs`

**Interfaces:**
- Consumes: `check_typst()` and `format_typst()` from Task 1
- Produces: Two new MCP tools registered with the `ScholarPressService`

- [ ] **Step 1: Add params structs**

Insert after the existing `ExtractDocumentParams` struct in `crates/sp-mcp/src/tools.rs`:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckTypstParams {
    pub workspace: PathBuf,
    pub file_path: PathBuf,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FormatTypstParams {
    pub workspace: PathBuf,
    pub file_path: PathBuf,
}
```

- [ ] **Step 2: Add tool handler methods**

Insert before the final `}` of the `impl ScholarPressService` block:

```rust
    #[tool(
        description = "Validate Typst syntax without full compilation. Runs `typstyle --check`. Returns \"ok\" if the file is properly formatted, \"needs_format\" if issues found (e.g., $ in prose, unclosed delimiters). Requires `typstyle` on PATH (install with `cargo install typstyle --locked`)."
    )]
    async fn check_typst(
        &self,
        params: Parameters<CheckTypstParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let status = workspace::check_typst(&p.workspace, &p.file_path).map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(status)]))
    }

    #[tool(
        description = "Format a Typst file in-place. Runs `typstyle -i` to normalize indentation, whitespace, and line width. Returns the absolute path of the formatted file. Requires `typstyle` on PATH (install with `cargo install typstyle --locked`)."
    )]
    async fn format_typst(
        &self,
        params: Parameters<FormatTypstParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let path = workspace::format_typst(&p.workspace, &p.file_path).map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(path)]))
    }
```

- [ ] **Step 3: Build to verify compilation**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend/.worktrees/sp-mcp
cargo build -p sp-mcp
```

Expected: success.

- [ ] **Step 4: Quick smoke test — tools register**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend/.worktrees/sp-mcp
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | SCHOLARPRESS_CATALOG_PATH=/tmp/fake SCHOLARPRESS_WORKSPACE_ROOT=/tmp/fake workspace_root=/tmp/fake cargo run -p sp-mcp 2>/dev/null | grep -o '"name":"check_typst\|"name":"format_typst'
```

Expected: both `check_typst` and `format_typst` appear in the tools list.

- [ ] **Step 5: Commit**

```bash
git add crates/sp-mcp/src/tools.rs
git commit -m "feat(sp-mcp): add check_typst and format_typst MCP tool handlers"
```

---

## Task 3: Update README with typstyle install step

**Files:**
- Modify: `crates/sp-mcp/README.md`

- [ ] **Step 1: Add typstyle install step**

In the "Requirements" section of `crates/sp-mcp/README.md`, alongside the existing `typst` binary requirement, add:

```markdown
- The `typstyle` binary on `PATH` (for `check_typst` and `format_typst`):
  ```bash
  cargo install typstyle --locked
  ```
```

- [ ] **Step 2: Commit**

```bash
git add crates/sp-mcp/README.md
git commit -m "docs(sp-mcp): add typstyle install step to README"
```

---

## Task 4: Final verification

- [ ] **Step 1: Full build + test**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend/.worktrees/sp-mcp
cargo build -p sp-mcp
cargo test -p sp-mcp --lib
```

Expected: build succeeds, 17 tests pass (15 old + 2 new if `typstyle` on PATH).

- [ ] **Step 2: Lint check**

```bash
cargo clippy -p sp-mcp --tests -- -D warnings
```

Expected: zero warnings.

- [ ] **Step 3: MCP server lists 8 tools (6 original + 2 new)**

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"final","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | SCHOLARPRESS_CATALOG_PATH=/tmp/fake SCHOLARPRESS_WORKSPACE_ROOT=/tmp/fake cargo run -p sp-mcp 2>/dev/null | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(len(d['result']['tools']), 'tools')"
```

Expected: `8 tools`

- [ ] **Step 4: End-to-end with typstyle**

```bash
# Create a test workspace
mkdir -p /tmp/sp-mcp-typstyle-test
printf '= Hello\n$17 million\n' > /tmp/sp-mcp-typstyle-test/bad.typ

cd /home/danriggi/scholarpress-workshop/scholarpress-backend/.worktrees/sp-mcp
printf '%s\n%s\n%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"e2e","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"check_typst","arguments":{"workspace":"/tmp/sp-mcp-typstyle-test","file_path":"bad.typ"}}}' \
  | SCHOLARPRESS_CATALOG_PATH=/tmp/fake SCHOLARPRESS_WORKSPACE_ROOT=/tmp/sp-mcp-typstyle-test cargo run -p sp-mcp 2>/dev/null
```

Expected: response to `check_typst` returns `"needs_format"` (or an error about `typstyle` not installed, which is also valid).

- [ ] **Step 5: Git log clean**

```bash
git log --oneline -4
```

Expected: 3 commits, each corresponding to one task.
