# ScholarPress: `check_typst` and `format_typst` MCP Tools (via typstyle)

## Context

During the first end-to-end MCP test, the agent spent 4 compile cycles
binary-searching text blocks to isolate a single `$17 million` in the CV
that broke the entire 25-page compilation. Common Typst gotchas — `$` in
prose, unclosed delimiters, wrong calling conventions — all require a full
`compile_typst` cycle to surface, even for trivial syntax errors.

[typstyle](https://github.com/typstyle-rs/typstyle) is a standalone CLI formatter
~5ms even for 3000-line files. It catches syntax errors without full
compilation and normalizes style. This design wraps two typstyle operations
as MCP tools: `check_typst` (syntax validation) and `format_typst`
(in-place formatting).

## Design

### Tool signatures

| Tool | typstyle command | Purpose |
|------|-----------------|---------|
| `check_typst(workspace, file_path)` | `typstyle --check <file>` | Validate syntax. Returns `"ok"` or `"needs_format"`. Catches `$` in prose, unclosed delimiters, calling convention errors without full compilation. |
| `format_typst(workspace, file_path)` | `typstyle -i <file>` | Format in-place. Normalizes indentation, whitespace, line width (80 cols). Modifies the file on disk. |

Both resolve `file_path` relative to the workspace. Both run `typstyle` from
the workspace root so relative imports resolve correctly.

### Agent workflow

```
edit file → check_typst → (ok? → compile_typst)
                         (needs_format → format_typst → compile_typst)
```

`check_typst` is a pre-flight: catch syntax errors in ~5ms instead of a
3-second `compile_typst` cycle. `format_typst` normalizes style before
compilation — recommended before every `compile_typst` call.

### Architecture

Same pattern as `compile_typst`: pure functions in `workspace.rs` that shell
out to the `typstyle` binary via `std::process::Command`. MCP tool handlers
in `tools.rs` wrap them.

No new Rust dependencies. `sp-mcp` already uses `Command` (via `sp-typst`
for the `typst` binary). The `typstyle` library crate is deliberately NOT
linked — the binary-on-PATH pattern is consistent with the `typst` dep and
the ponytail annotation from the sp-mcp design.

### Error responses

| Case | Response |
|------|----------|
| `typstyle` not on PATH | `"typstyle binary not found on PATH. Install with: cargo install typstyle --locked"` |
| File doesn't exist | `"file not found: <path>"` |
| Syntax error | `"syntax error at <file>:<line>:<col>: <message>"` (typstyle's native error output) |
| Check: file needs formatting | `"needs_format"` (NOT an error — this is a useful signal) |
| Check: file is properly formatted | `"ok"` |
| Format: success | Absolute path of the formatted file |

### File-level changes

| File | Change |
|------|--------|
| `crates/sp-mcp/src/workspace.rs` | Add `check_typst()` and `format_typst()` pure functions |
| `crates/sp-mcp/src/tools.rs` | Add `CheckTypstParams`, `FormatTypstParams` structs; two MCP tool handlers; `SpMcpError::Format` variant (or reuse `Compilation`) |
| `crates/sp-mcp/README.md` | Add `typstyle` install step alongside `typst`: `cargo install typstyle --locked` |

### Test plan

One test per tool. A small `.typ` file with `$17 million` in prose:

```rust
#[test]
fn check_typst_catches_dollar_in_prose() {
    let ws = local_tempdir();
    fs::write(ws.join("bad.typ"), "= Hello\n$17 million\n").unwrap();
    let result = check_typst(&ws, Path::new("bad.typ"));
    // typstyle --check exits non-zero for unparseable content
    assert!(result.is_ok()); // the function succeeds (typstyle ran)
    assert_eq!(result.unwrap(), "needs_format"); // because $ in prose is invalid syntax
}

#[test]
fn format_typst_modifies_file_or_skips_if_no_typstyle() {
    let ws = local_tempdir();
    fs::write(ws.join("input.typ"), "= Hello\n\n\n  world\n").unwrap();
    let result = format_typst(&ws, Path::new("input.typ"));
    match result {
        Ok(_) => {
            let formatted = fs::read_to_string(ws.join("input.typ")).unwrap();
            assert!(!formatted.contains("  world")); // indentation normalized
        }
        Err(SpMcpError::Compilation(msg)) if msg.contains("typstyle") => {
            // typstyle not on PATH — skip
        }
        Err(e) => panic!("unexpected error: {:?}", e),
    }
}
```

## Non-goals

- **`typstyle` library integration.** The `typstyle-core` crate exists but
  we use the CLI binary. Consistent with the `typst` binary pattern. Add
  library integration if binary startup overhead becomes measurable (it's
  ~5ms today).
- **Custom formatting configuration.** Default `typstyle` settings
  (80-column width, 2-space indent). Add configuration options if the IU
  spec requires a specific style that `typstyle` defaults don't produce.
- **`diff_typst` tool.** The `typstyle --diff` flag is useful for preview
  but not needed for the agent workflow. Add if the agent requests a
  "show me what changed" feature.
- **`tinymist` lint integration.** The LSP-level lint rules in tinymist
  are more powerful but require running a daemon. Revisit if `typstyle
  --check` misses a common lint that tinymist catches.

## Verification

- `cargo test -p sp-mcp --lib` — two new tests pass
- `typstyle` absent from PATH → both tools return clear "install typstyle"
  error, don't crash
- End-to-end: create a workspace, write a `.typ` file with `$17 million`,
  run `check_typst` → `"needs_format"`, run `format_typst` → file formatted,
  run `compile_typst` → PDF produced
- Existing tests unchanged (13 → 15)
