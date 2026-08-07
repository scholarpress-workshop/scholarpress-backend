# sp-mcp

Stdio MCP server for the ScholarPress ecosystem. Exposes workspace/profile
discovery, document conversion, Typst compilation, PDF checks, and template
interface documentation for use from any MCP-compliant agent harness.

## Requirements

- Rust 1.88+ (for building)
- The `typst` binary on `PATH` (for `compile_typst`):
  ```bash
  cargo install typst-cli
  # or download from https://github.com/typst/typst/releases
  ```
## Build

```bash
cargo build --release -p sp-mcp
```

Binary: `target/release/sp-mcp`

## Environment

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `SCHOLARPRESS_CATALOG_PATH` | yes | — | Path to a local `scholarpress-catalog` checkout |
| `SCHOLARPRESS_WORKSPACE_ROOT` | no | `~/.scholarpress/workspaces` | Root for per-job scratch directories |

## OpenCode configuration

`~/.config/opencode/opencode.json` (or equivalent):

```json
{
  "mcp": {
    "scholarpress": {
      "command": "/absolute/path/to/sp-mcp",
      "env": {
        "SCHOLARPRESS_CATALOG_PATH": "/absolute/path/to/scholarpress-catalog",
        "SCHOLARPRESS_WORKSPACE_ROOT": "/home/you/.scholarpress/workspaces"
      }
    }
  }
}
```

Restart OpenCode after editing. The server appears in the MCP panel as
"scholarpress" with six tools.

## Tools

| Tool | Args | Returns |
|------|------|---------|
| `list_profiles` | — | JSON array of `{id, scope, name}` |
| `list_workspaces` | — | JSON array of `{name, path, profile_id, mtime}` |
| `create_workspace` | `{name, profile_id}` | Absolute workspace path |
| `compile_typst` | `{workspace, entry_path, data?, out_name?}` | Absolute path to written PDF |
| `check_pdf` | `{workspace, pdf_path}` | JSON array of `{id, status, message, page}` |
| `extract_document` | `{file_path}` | JSON `ParsedDocument` |

## Workflow

1. Call `list_profiles` to discover available profiles
2. Call `create_workspace` with `{name, profile_id}` to fork a profile
3. Use the agent harness's built-in file/edit tools to modify templates
4. Call `compile_typst` to render → PDF
5. Call `check_pdf` to validate against the spec
6. Iterate (3-5) until `check_pdf` returns zero violations

## See also

- Spec: `docs/superpowers/specs/2026-07-29-scholarpress-mcp-design.md`
- Catalog: <https://github.com/scholarpress-workshop/scholarpress-catalog>
