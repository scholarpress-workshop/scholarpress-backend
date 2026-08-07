# OpenWork Local `sp-mcp` Design

## Goal

Run ScholarPress locally from the OpenWork desktop app on native Windows while preserving existing OpenCode/stdio compatibility and supporting the current Linux/WSL workflow.

## Context and constraints

OpenWork's documented custom-app flow is URL-based: `Settings` -> `Extensions` -> `Add Custom App`. The first prototype therefore uses a localhost Streamable HTTP MCP endpoint rather than depending on undocumented local-stdio launching.

The current `sp-mcp` service is stdio-only. Its Typst and DOCX conversion tools launch external `typst` and `pandoc` processes by command name from `PATH`. Native Windows requires a Windows `sp-mcp.exe`, `typst.exe`, and `pandoc.exe`; WSL continues to use Linux binaries.

The first distribution is a developer-oriented ZIP/tarball plus launcher. A full installer is intentionally deferred:

```text
// ponytail: ship archives first; add an installer when manual extraction and launch are proven.
```

## Architecture

Keep one `sp-mcp` implementation with selectable transports:

```text
sp-mcp --transport stdio  # existing OpenCode and MCP clients
sp-mcp --transport http   # OpenWork custom app
```

HTTP mode uses `rmcp`'s Streamable HTTP server transport, binds to `127.0.0.1` only, and exposes the same `ScholarPressService` and tool handlers as stdio mode. The server provides a health endpoint for launcher diagnostics and uses a configured port with a documented default.

The OpenWork integration does not duplicate MCP tools. The first prototype is manual:

1. Extract the platform archive.
2. Run the platform launcher.
3. Paste the printed localhost MCP URL into OpenWork's custom-app dialog.
4. Use the same MCP tools as other clients.

A future OpenWork wrapper may automate steps 2-3, but only as a thin layer over the archive and launcher. It must not own catalog, process, or tool logic.

## Platform artifacts

Publish separate artifacts with shared source and platform-specific executables:

```text
scholarpress-windows-x86_64.zip
  scholarpress/sp-mcp.exe
  scholarpress/start-scholarpress.ps1
  scholarpress/catalog/
  scholarpress/bin/typst.exe
  scholarpress/bin/pandoc.exe
  scholarpress/README-WINDOWS.md

scholarpress-linux-x86_64.tar.gz
  scholarpress/sp-mcp
  scholarpress/start-scholarpress.sh
  scholarpress/catalog/
  scholarpress/bin/typst
  scholarpress/bin/pandoc
  scholarpress/README-LINUX.md
```

The initial build matrix is `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`. WSL uses the Linux artifact. Native OpenWork uses the Windows artifact and does not depend on WSL path or process translation. macOS and ARM64 artifacts are out of scope.

## Runtime layout and configuration

The launcher creates and configures a dedicated ScholarPress child directory inside the OpenWork workspace:

```text
<openwork-workspace>/.scholarpress/
  workspaces/
  catalog/
  bin/
```

`SCHOLARPRESS_WORKSPACE_ROOT` points to `.scholarpress/workspaces`, not the entire OpenWork workspace. The catalog is a separate read-only configured root. The launcher sets catalog, workspace, and tool paths before starting HTTP mode.

External executable resolution is deterministic:

1. `SCHOLARPRESS_TYPST_PATH` or `SCHOLARPRESS_PANDOC_PATH`.
2. The bundled executable under `bin/` beside `sp-mcp`.
3. `typst` or `pandoc` from `PATH`.

The server invokes resolved executable paths directly with `std::process::Command`; it does not invoke a shell. Startup reports the resolved catalog, workspace, Typst, and Pandoc paths. Missing external tools fail when the affected tool is called and return an actionable path/override error. Catalog and profile tools remain usable without Typst or Pandoc.

The launcher prints the endpoint and runtime information:

```text
ScholarPress MCP
Transport: streamable HTTP
Endpoint: http://127.0.0.1:8765/mcp
Catalog: ...
Typst: ...
Pandoc: ...
```

Manual local-machine trust is acceptable for the prototype because the server binds only to loopback. A generated bearer token is a future hardening step if OpenWork supports custom headers or URL credentials for custom apps.

## Filesystem boundary

The current implementation constrains `create_workspace` names beneath `SCHOLARPRESS_WORKSPACE_ROOT`, but other MCP parameters accept arbitrary paths. HTTP mode must enforce the complete boundary before release.

Every workspace path must canonicalize beneath the configured workspace root. File inputs, entry files, PDFs, and generated outputs must remain inside the selected workspace; output names must remain beneath its `out/` directory. DOCX conversion may read only a file inside the selected workspace and may write only inside its `out/` directory. Catalog access remains limited to profile discovery and copying.

Validation must reject:

- Absolute paths outside the configured root.
- `..` traversal after path resolution.
- Output paths escaping `workspace/out`.
- Symlink, junction, or Windows reparse-point escapes.

The first implementation keeps existing MCP parameter shapes and validates paths at the server boundary. Replacing path parameters with workspace names or opaque IDs is a later API cleanup, not a prerequisite for the OpenWork integration.

## Verification

- Stdio startup remains functional for existing clients.
- HTTP mode passes MCP initialize, `tools/list`, and a harmless tool call.
- Workspace, input, output, traversal, symlink, junction, and reparse-point boundary tests pass.
- Executable resolution covers explicit overrides, bundled binaries, PATH fallback, and missing-tool diagnostics.
- Windows smoke tests invoke bundled `typst.exe` and `pandoc.exe`; Linux tests invoke their platform equivalents.
- Launchers create the directory layout, start the server, print a usable URL, and forward clean shutdown.
- Manual OpenWork acceptance covers `list_profiles`, `create_workspace`, DOCX conversion, compilation, and PDF checks through the localhost URL.

## Scope exclusions

- Full installer or OS package manager integration.
- OpenWork source fork or duplicated tool implementation.
- Public/network HTTP binding.
- macOS or ARM64 release artifacts.
- Generated HTTP authentication token until OpenWork custom-app support can carry it.
