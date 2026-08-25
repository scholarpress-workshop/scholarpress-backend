# sp-mcp

MCP server for the ScholarPress ecosystem. Exposes workspace/profile discovery,
document conversion, Typst compilation, PDF checks, and template interface
documentation over stdio or localhost Streamable HTTP.

## Requirements

- Rust 1.88+ (for building)
- The `typst` binary on `PATH` (for `compile_typst`) when using a source checkout:
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
| `SCHOLARPRESS_TRANSPORT` | no | `stdio` | `stdio` or `http` |
| `SCHOLARPRESS_BIND` | no | `127.0.0.1` | HTTP bind address |
| `SCHOLARPRESS_PORT` | no | `8765` | HTTP port |
| `SCHOLARPRESS_TYPST_PATH` | no | bundle, then PATH | Explicit Typst executable |
| `SCHOLARPRESS_PANDOC_PATH` | no | bundle, then PATH | Explicit Pandoc executable |

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
"scholarpress" with the current tool set.

## Goose on Windows

For native Windows, use the platform archive and setup script under
`packaging/`. The script creates a dedicated `.scholarpress` directory inside
the selected project, then adds `sp-mcp.exe` as a standard Goose command-line
extension. Goose Desktop and Goose CLI share the resulting configuration.

```powershell
.\setup-goose.ps1 -ProjectPath "C:\Projects\dissertation"
```

Use `-CatalogPath`, `-TypstPath`, or `-PandocPath` for development overrides.
The setup script does not run a long-lived server; Goose owns the `sp-mcp.exe`
process. Manual configuration through Goose's custom-extension UI or
`goose configure` is also supported. WSL uses the Linux stdio workflow; native
Windows uses the Windows archive and does not require WSL.

## Tools

| Tool | Args | Returns |
|------|------|---------|
| `list_profiles` | — | JSON array of `{id, scope, name}` |
| `list_workspaces` | — | JSON array of `{name, path, profile_id, mtime}` |
| `create_workspace` | `{name, profile_id}` | Absolute workspace path |
| `compile_typst` | `{workspace, entry_path, out_name?}` | Absolute path to written PDF |
| `check_pdf` | `{workspace, pdf_path}` | JSON array of `{id, status, message, page}` |
| `pandoc_convert` | `{file_path, format, workspace}` | Absolute path to Typst or AST output |
| `interface_doc` | `{workspace}` | Pretty-printed template reference JSON |

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
