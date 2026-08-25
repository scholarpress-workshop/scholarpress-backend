# Goose Local `sp-mcp` Design

## Goal

Make ScholarPress usable from Goose on native Windows through Goose's standard command-line MCP extension configuration. Support both Goose Desktop and Goose CLI without implementing a Goose-native extension or coupling ScholarPress to Goose internals.

## Context and constraints

Goose extensions are MCP servers. Goose can launch local command-line extensions over stdio and stores shared extension configuration for Desktop and CLI in `~/.config/goose/config.yaml`. The existing `sp-mcp` server already uses stdio, so this integration does not require a new transport.

The first distribution is a developer-oriented Windows ZIP plus a PowerShell configurator. A full installer is deferred:

```text
# ponytail: ship an archive and configurator first; add an installer after the workflow is proven.
```

The implementation targets native Windows x86_64. Linux/WSL and other platforms remain compatible with the existing stdio server but are not part of this integration's packaging work.

## Architecture

Keep `sp-mcp.exe` as the single MCP implementation and run it as a Goose command-line extension:

```yaml
extensions:
  scholarpress:
    name: ScholarPress
    description: Format and validate dissertation documents with ScholarPress
    cmd: C:\\Tools\\scholarpress\\sp-mcp.exe
    args: []
    enabled: true
    type: stdio
    timeout: 300
    envs:
      SCHOLARPRESS_CATALOG_PATH: C:\\Tools\\scholarpress\\catalog
      SCHOLARPRESS_WORKSPACE_ROOT: C:\\Projects\\dissertation\\.scholarpress\\workspaces
      SCHOLARPRESS_TYPST_PATH: C:\\Tools\\scholarpress\\bin\\typst.exe
      SCHOLARPRESS_PANDOC_PATH: C:\\Tools\\scholarpress\\bin\\pandoc.exe
```

The exact paths are resolved from setup-script arguments; the example is illustrative, not hard-coded.

The integration does not add a Goose SDK dependency, implement Goose's `Extension` trait, fork Goose, or add an HTTP listener. Goose owns the `sp-mcp.exe` child process and communicates through stdio. Existing OpenCode and other stdio clients remain unaffected.

## Windows artifact

The archive contains:

```text
scholarpress-windows-x86_64/
  sp-mcp.exe
  bin/
    typst.exe
    pandoc.exe
  catalog/
  setup-goose.ps1
  README-WINDOWS.md
```

The catalog is a pinned, read-only snapshot for normal use. Developers may select another catalog checkout through `-CatalogPath` or `SCHOLARPRESS_CATALOG_PATH`.

## Runtime and filesystem boundary

The setup script accepts a project path and derives the isolated workspace root:

```text
<project>\\.scholarpress\\
  workspaces\\
```

All workspaces, inputs, entry files, PDFs, and generated outputs must remain within the configured workspace root. Output names must remain beneath the selected workspace's `out` directory. Existing OpenWork boundary requirements carry forward, including rejection of traversal, outside-root paths, symlink escapes, junction escapes, and Windows reparse-point escapes.

Executable resolution remains:

1. Explicit `SCHOLARPRESS_TYPST_PATH` or `SCHOLARPRESS_PANDOC_PATH`.
2. Bundle-local `bin\\typst.exe` or `bin\\pandoc.exe`.
3. `PATH` fallback.

The server invokes resolved paths directly with `std::process::Command`, never through a shell. The catalog is a separate read-only configured root.

## Setup flow

The primary path is:

1. Extract the Windows archive.
2. Run `setup-goose.ps1 -ProjectPath <path>`.
3. Create `<project>\\.scholarpress\\workspaces`.
4. Resolve the bundled executables and catalog, unless explicit overrides are supplied.
5. Back up Goose's existing `config.yaml`.
6. Add or update only the `scholarpress` extension entry.
7. Optionally start Goose when `-StartGoose` is supplied.

Example invocation:

```powershell
.\\setup-goose.ps1 `
  -ProjectPath "C:\\Projects\\dissertation" `
  -BundlePath "C:\\Tools\\scholarpress"
```

For development catalog use:

```powershell
.\\setup-goose.ps1 `
  -ProjectPath "C:\\Projects\\dissertation" `
  -BundlePath "C:\\Tools\\scholarpress" `
  -CatalogPath "C:\\src\\scholarpress-catalog"
```

The script is a configurator, not a long-running server launcher. Goose remains responsible for starting, restarting, and stopping `sp-mcp.exe`.

Manual configuration through Goose Desktop's custom-extension UI or `goose configure` is supported and documented as a fallback. Both interfaces use the shared configuration.

## Setup safety and errors

The script must validate all required paths before changing Goose configuration. It must stop with an actionable error when:

- The project or bundle path does not exist.
- `sp-mcp.exe`, the catalog, or required bundled tools are missing.
- Goose's config directory cannot be created or read.
- The existing config is invalid YAML.
- The extension entry cannot be updated safely.

The script must preserve unrelated extensions, quote Windows paths correctly, and create a timestamped config backup before a successful update. It must not silently replace a user-supplied catalog, executable override, or unrelated configuration.

## Verification

- Build and test the Windows `sp-mcp.exe` release binary.
- Test bundled Typst and Pandoc resolution, explicit overrides, PATH fallback, and missing-tool errors.
- Test setup-script parameter validation and derived paths.
- Test config updates preserve unrelated extensions and correctly replace the ScholarPress entry.
- Test `.scholarpress\\workspaces` creation.
- Test MCP stdio initialize, tool listing, workspace creation, DOCX conversion, Typst compilation, and PDF checking.
- Test workspace, output, symlink, junction, and reparse-point boundaries.
- Manually verify activation in Goose Desktop and Goose CLI using the shared config.

## Scope exclusions

- Goose-native `Extension` trait implementation.
- Goose source changes or a Goose fork.
- Streamable HTTP transport.
- Installer, MSIX, or package-manager integration.
- macOS, Linux, or ARM64 release artifacts for this integration.
- Automatic publishing to the Goose extension directory.

## References

- Existing baseline: `docs/superpowers/specs/2026-08-07-openwork-local-sp-mcp-design.md`
- Goose extension architecture: <https://goose-docs.ai/docs/goose-architecture/extensions-design>
- Goose extension configuration: <https://goose-docs.ai/docs/getting-started/using-extensions>
