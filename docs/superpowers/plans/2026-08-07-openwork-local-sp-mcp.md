# OpenWork Local `sp-mcp` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a localhost Streamable HTTP mode and Windows/Linux archive launchers so OpenWork can run the existing ScholarPress MCP tools locally without breaking stdio clients.

**Architecture:** Keep one `ScholarPressService` and select either `rmcp::transport::stdio` or `rmcp` Streamable HTTP at startup. Put all filesystem and executable resolution behind the existing `Config`/workspace boundary, then package the same binary with platform-native Typst, Pandoc, catalog, and launcher files. The first OpenWork integration is manual URL registration; no OpenWork fork or installer is added.

**Tech Stack:** Rust 2021, Tokio, `rmcp` 3 Streamable HTTP server transport, `std::process::Command`, PowerShell, Bash, GitHub Actions, Typst, Pandoc.

## Global Constraints

- HTTP mode binds to `127.0.0.1` only.
- `sp-mcp --transport stdio` remains the default for existing clients.
- `sp-mcp --transport http` serves the same `ScholarPressService` through Streamable HTTP.
- The dedicated root is `<openwork-workspace>/.scholarpress/`; `SCHOLARPRESS_WORKSPACE_ROOT` points to `.scholarpress/workspaces`.
- Catalog access is a separate read-only configured root.
- Resolve executables in this order: explicit environment override, bundled `bin/`, then `PATH`.
- Invoke Typst and Pandoc with direct executable paths, never through a shell.
- Reject path traversal, outside-root paths, output escapes, symlink escapes, junction escapes, and Windows reparse-point escapes.
- Initial artifacts target `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`.
- The first release is a ZIP/tarball plus launcher; installer work is deferred.
- Generated bearer-token authentication is out of scope until OpenWork custom-app credentials are confirmed.

---

## File Map

- `crates/sp-mcp/Cargo.toml`: enable the rmcp Streamable HTTP server feature.
- `crates/sp-mcp/src/lib.rs`: select stdio or HTTP transport and expose the HTTP health route.
- `crates/sp-mcp/src/main.rs`: parse transport, bind, port, and runtime configuration.
- `crates/sp-mcp/src/config.rs`: add platform-neutral runtime configuration and bundle path resolution.
- `crates/sp-mcp/src/tools.rs`: preserve the existing tool surface and update runtime instructions.
- `crates/sp-mcp/src/workspace.rs`: enforce workspace-root containment and direct executable resolution.
- `crates/sp-mcp/src/error.rs`: add actionable path/tool errors if existing variants are insufficient.
- `crates/sp-mcp/tests/http_transport.rs`: MCP HTTP initialize/list/call smoke tests.
- `crates/sp-mcp/tests/path_boundary.rs`: traversal, output, symlink, junction, and reparse-point tests.
- `crates/sp-mcp/tests/executable_resolution.rs`: explicit/bundled/PATH/missing-tool resolution tests.
- `packaging/start-scholarpress.ps1`: Windows launcher.
- `packaging/start-scholarpress.sh`: Linux/WSL launcher.
- `packaging/build-bundle.ps1`: Windows archive assembly with pinned executable downloads.
- `packaging/build-bundle.sh`: Linux archive assembly with pinned executable downloads.
- `.github/workflows/release-bundles.yml`: build and publish Windows/Linux archives.
- `crates/sp-mcp/README.md`: local HTTP, Windows ZIP, Linux tarball, and OpenWork instructions.

## Task 1: Add Dual MCP Transports

**Files:**
- Modify: `crates/sp-mcp/Cargo.toml`
- Modify: `crates/sp-mcp/src/lib.rs`
- Modify: `crates/sp-mcp/src/main.rs`
- Modify: `crates/sp-mcp/src/config.rs`
- Create: `crates/sp-mcp/tests/http_transport.rs`

**Interfaces:**
- `sp-mcp --transport stdio` starts the existing stdio server.
- `sp-mcp --transport http --bind 127.0.0.1 --port 8765` starts the HTTP server.
- HTTP MCP endpoint is `/mcp`; health endpoint is `/health`.

- [ ] **Step 1: Add an HTTP startup test that expects the transport to exist**

Add a test helper that starts the service on an ephemeral loopback port and sends an HTTP MCP initialize request. Assert the response is successful and the server advertises `scholarpress-mcp`.

```rust
#[tokio::test]
async fn http_transport_accepts_mcp_initialize() {
    let server = spawn_http_for_test().await;
    let response = reqwest::Client::new()
        .post(server.url("/mcp"))
        .header("content-type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#)
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
}
```

- [ ] **Step 2: Run the new test and verify the expected failure**

Run:

```bash
rtk cargo test -p sp-mcp --test http_transport http_transport_accepts_mcp_initialize
```

Expected: FAIL because only stdio transport exists and the test helper is not implemented.

- [ ] **Step 3: Enable rmcp HTTP transport and add explicit startup options**

Add the rmcp Streamable HTTP server feature alongside `transport-io`. Add a small transport enum and startup configuration with these defaults:

```rust
enum TransportMode {
    Stdio,
    Http,
}

struct ServerOptions {
    transport: TransportMode,
    bind: std::net::IpAddr,
    port: u16,
}
```

Use `127.0.0.1` and port `8765` for HTTP defaults. Keep stdio as the default when no flag is supplied. Read `SCHOLARPRESS_TRANSPORT`, `SCHOLARPRESS_BIND`, and `SCHOLARPRESS_PORT` only as optional overrides; command-line values win.

- [ ] **Step 4: Serve the same service over both transports**

Keep the existing stdio path:

```rust
let service = ScholarPressService::new(config).serve(stdio()).await?;
service.waiting().await?;
```

Add an HTTP path using `StreamableHttpService` with an Axum/Tower router at `/mcp`, a `/health` response containing `{"status":"ok"}`, and loopback binding. Do not create a second service implementation or duplicate handlers.

- [ ] **Step 5: Run transport tests and the full crate suite**

```bash
rtk cargo test -p sp-mcp --test http_transport
rtk cargo test -p sp-mcp
```

Expected: HTTP initialize/list/call tests pass and all existing sp-mcp tests pass.

- [ ] **Step 6: Commit the transport boundary**

```bash
rtk git add crates/sp-mcp/Cargo.toml crates/sp-mcp/src/lib.rs crates/sp-mcp/src/main.rs crates/sp-mcp/src/config.rs crates/sp-mcp/tests/http_transport.rs
rtk git commit -m "feat: add local HTTP MCP transport"
```

## Task 2: Enforce The `.scholarpress` Filesystem Boundary

**Files:**
- Modify: `crates/sp-mcp/src/config.rs`
- Modify: `crates/sp-mcp/src/workspace.rs`
- Modify: `crates/sp-mcp/src/error.rs`
- Create: `crates/sp-mcp/tests/path_boundary.rs`

**Interfaces:**
- All workspace paths are validated against `Config.workspace_root` before use.
- Catalog paths remain under `Config.catalog_path` and are read/copy-only.
- Existing MCP parameter shapes remain unchanged in this task.

- [ ] **Step 1: Add failing boundary tests**

Cover these cases:

```rust
#[test]
fn rejects_workspace_outside_root() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let config = Config::new(root.path().to_path_buf(), root.path().join("workspaces"));
    std::fs::write(outside.path().join("entry.typ"), "= Outside").unwrap();
    assert!(compile_typst(&config, outside.path(), Path::new("entry.typ"), None).is_err());
}

#[test]
fn rejects_entry_path_traversal() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspaces/job");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(root.path().join("outside.typ"), "= Outside").unwrap();
    let config = Config::new(root.path().to_path_buf(), root.path().join("workspaces"));
    assert!(compile_typst(&config, &workspace, Path::new("../../outside.typ"), None).is_err());
}

#[test]
fn rejects_output_name_escape() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspaces/job");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(workspace.join("entry.typ"), "= Job").unwrap();
    let config = Config::new(root.path().to_path_buf(), root.path().join("workspaces"));
    assert!(compile_typst(&config, &workspace, Path::new("entry.typ"), Some("../outside")).is_err());
}

#[cfg(unix)]
#[test]
fn rejects_symlink_escape() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspaces/job");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(outside.path().join("entry.typ"), "= Outside").unwrap();
    symlink(outside.path().join("entry.typ"), workspace.join("entry.typ")).unwrap();
    let config = Config::new(root.path().to_path_buf(), root.path().join("workspaces"));
    assert!(compile_typst(&config, &workspace, Path::new("entry.typ"), None).is_err());
}

#[cfg(windows)]
#[test]
fn rejects_junction_escape() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspaces/job");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(outside.path().join("entry.typ"), "= Outside").unwrap();
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(workspace.join("linked"))
        .arg(outside.path())
        .status()
        .unwrap();
    let config = Config::new(root.path().to_path_buf(), root.path().join("workspaces"));
    assert!(compile_typst(&config, &workspace.join("linked"), Path::new("entry.typ"), None).is_err());
}
```

Each test must assert a path-boundary error before any process is spawned or file is written.

- [ ] **Step 2: Run boundary tests and verify failure**

```bash
rtk cargo test -p sp-mcp --test path_boundary
```

Expected: FAIL because current tools accept arbitrary workspace/file paths.

- [ ] **Step 3: Implement canonical containment helpers**

Add helpers with explicit responsibilities:

```rust
fn canonical_root(path: &Path) -> Result<PathBuf, SpMcpError>;
fn require_existing_under(root: &Path, path: &Path) -> Result<PathBuf, SpMcpError>;
fn require_output_under(root: &Path, path: &Path) -> Result<PathBuf, SpMcpError>;
```

Use `canonicalize`, `Path::starts_with`, and component checks. For a new output, canonicalize the nearest existing parent before appending the final filename. Reject symlink/junction/reparse-point components that resolve outside the allowed root.

- [ ] **Step 4: Apply the helpers to every path-taking tool**

Validate:

- `workspace` in `compile_typst`, `check_pdf`, `interface_doc`, and `pandoc_convert`.
- `entry_path` beneath the validated workspace.
- `pdf_path` beneath the validated workspace.
- `file_path` for Pandoc beneath the validated workspace.
- `out_name` beneath `<workspace>/out`.

Retain `create_workspace` name validation and ensure the root is created before canonicalization. Leave catalog profile copying limited to `Config.catalog_path`.

- [ ] **Step 5: Run boundary and regression tests**

```bash
rtk cargo test -p sp-mcp --test path_boundary
rtk cargo test -p sp-mcp
```

Expected: all boundary cases reject escapes and existing workspace/profile tests pass.

- [ ] **Step 6: Commit the boundary**

```bash
rtk git add crates/sp-mcp/src/config.rs crates/sp-mcp/src/workspace.rs crates/sp-mcp/src/error.rs crates/sp-mcp/tests/path_boundary.rs
rtk git commit -m "feat: sandbox MCP workspace paths"
```

## Task 3: Resolve Bundled And Override Executables

**Files:**
- Modify: `crates/sp-mcp/src/config.rs`
- Modify: `crates/sp-mcp/src/workspace.rs`
- Modify: `crates/sp-mcp/src/error.rs`
- Create: `crates/sp-mcp/tests/executable_resolution.rs`

**Interfaces:**
- `SCHOLARPRESS_TYPST_PATH` and `SCHOLARPRESS_PANDOC_PATH` override all discovery.
- Without overrides, the resolver checks `<executable_dir>/bin/<name>` before PATH.
- Process invocation receives an absolute executable path and never uses a shell.
- `ToolResolver::new(executable_dir: PathBuf, path_dirs: Vec<PathBuf>)` constructs a testable resolver.
- `ToolResolver::resolve(name: &str, override_path: Option<&Path>) -> Result<PathBuf, SpMcpError>` returns an absolute executable path.

- [ ] **Step 1: Add resolver tests**

Test explicit override, bundled sibling executable, PATH fallback, and missing executable diagnostics using temporary executable stubs. On Windows, use `.exe`; on Unix, set executable permissions on the stub.

```rust
#[test]
fn explicit_tool_override_wins_over_bundle_and_path() {
    let bundle = tempdir_with_executable("bin/typst");
    let path_dir = tempdir_with_executable("typst");
    let override_path = tempdir_with_executable("custom/typst");
    let resolver = ToolResolver::new(bundle.path().to_path_buf(), vec![path_dir.path().to_path_buf()]);
    assert_eq!(resolver.resolve("typst", Some(override_path.path().join("custom/typst").as_path())).unwrap(), override_path.path().join("custom/typst"));
}

#[test]
fn bundle_tool_is_used_before_path() {
    let bundle = tempdir_with_executable("bin/typst");
    let path_dir = tempdir_with_executable("typst");
    let resolver = ToolResolver::new(bundle.path().to_path_buf(), vec![path_dir.path().to_path_buf()]);
    assert_eq!(resolver.resolve("typst", None).unwrap(), bundle.path().join("bin/typst"));
}

#[test]
fn missing_tool_error_lists_override_and_bundle_locations() {
    let bundle = tempfile::tempdir().unwrap();
    let resolver = ToolResolver::new(bundle.path().to_path_buf(), Vec::new());
    let error = resolver.resolve("pandoc", None).unwrap_err().to_string();
    assert!(error.contains("SCHOLARPRESS_PANDOC_PATH"));
    assert!(error.contains("bin/pandoc"));
}
```

- [ ] **Step 2: Run resolver tests and verify failure**

```bash
rtk cargo test -p sp-mcp --test executable_resolution
```

Expected: FAIL because `compile_typst` and `pandoc_convert` currently call bare command names.

- [ ] **Step 3: Implement executable discovery**

Add `tool_dir`/executable-directory configuration derived from `std::env::current_exe()`. Resolve platform names with `.exe` on Windows. Use `std::process::Command::new(&resolved_path)` for both tools and include the resolved path in process errors.

- [ ] **Step 4: Use the resolver in Typst and Pandoc operations**

Replace `Command::new("typst")` and `Command::new("pandoc")` with resolved paths. Preserve existing argument order and output behavior. Add startup diagnostics for resolved paths without failing startup when an optional tool is absent.

- [ ] **Step 5: Run resolver and integration tests**

```bash
rtk cargo test -p sp-mcp --test executable_resolution
rtk cargo test -p sp-mcp
```

Expected: all resolver, compile, conversion, and workspace tests pass.

- [ ] **Step 6: Commit executable resolution**

```bash
rtk git add crates/sp-mcp/src/config.rs crates/sp-mcp/src/workspace.rs crates/sp-mcp/src/error.rs crates/sp-mcp/tests/executable_resolution.rs
rtk git commit -m "feat: resolve bundled Typst and Pandoc binaries"
```

## Task 4: Add Platform Launchers And Archive Assembly

**Files:**
- Create: `packaging/start-scholarpress.ps1`
- Create: `packaging/start-scholarpress.sh`
- Create: `packaging/build-bundle.ps1`
- Create: `packaging/build-bundle.sh`
- Modify: `crates/sp-mcp/README.md`

**Interfaces:**
- Windows launcher accepts `-OpenWorkWorkspace` and optional `-Port`.
- Linux launcher accepts the workspace path as its first argument and optional `SCHOLARPRESS_PORT`.
- Both launchers set catalog/workspace/tool overrides and print the MCP URL.

- [ ] **Step 1: Add launcher smoke tests**

Create shell-level checks that invoke each launcher with a fake bundle and assert:

- `.scholarpress/workspaces` is created.
- The server receives the catalog and workspace environment variables.
- The printed endpoint contains `127.0.0.1`, the selected port, and `/mcp`.
- Terminating the launcher terminates the child server.

- [ ] **Step 2: Implement the Windows launcher**

Use PowerShell process APIs, not `cmd /c`, to start `sp-mcp.exe`. Resolve `$PSScriptRoot`, create `<OpenWorkWorkspace>/.scholarpress/{workspaces,catalog}`, set `SCHOLARPRESS_CATALOG_PATH`, `SCHOLARPRESS_WORKSPACE_ROOT`, `SCHOLARPRESS_TYPST_PATH`, and `SCHOLARPRESS_PANDOC_PATH`, then start HTTP mode and print the endpoint.

- [ ] **Step 3: Implement the Linux/WSL launcher**

Use the script directory as the bundle root, create the same `.scholarpress` layout, export the four runtime variables, and `exec` the Linux `sp-mcp` binary in HTTP mode so signals propagate directly.

- [ ] **Step 4: Implement archive assembly**

Pin the Typst and Pandoc versions in each build script. Download the official platform assets, verify their checksums, copy the matching `sp-mcp` release binary and catalog, and create:

```text
scholarpress-windows-x86_64.zip
scholarpress-linux-x86_64.tar.gz
```

Do not create an installer or mutate the OpenWork application directory.

- [ ] **Step 5: Document manual OpenWork setup**

Update `crates/sp-mcp/README.md` with archive extraction, launcher invocation, `.scholarpress` layout, executable override variables, and the exact OpenWork custom-app URL flow.

- [ ] **Step 6: Run launcher checks and commit packaging**

```bash
rtk bash packaging/start-scholarpress.sh --help
rtk powershell -NoProfile -ExecutionPolicy Bypass -File packaging/start-scholarpress.ps1 -Help
rtk git add packaging crates/sp-mcp/README.md
rtk git commit -m "feat: add local ScholarPress platform bundles"
```

Expected: scripts parse successfully; full launcher execution is covered on the matching OS in CI.

## Task 5: Add Cross-Platform CI And OpenWork Acceptance Checks

**Files:**
- Create: `.github/workflows/release-bundles.yml`
- Modify: `crates/sp-mcp/tests/http_transport.rs`
- Modify: `crates/sp-mcp/README.md`

**Interfaces:**
- CI builds Windows MSVC and Linux GNU release binaries and uploads the two archive artifacts.
- Acceptance documentation uses the printed localhost URL and does not require WSL on Windows.

- [ ] **Step 1: Add CI matrix configuration**

Define jobs for `windows-latest`/`x86_64-pc-windows-msvc` and `ubuntu-latest`/`x86_64-unknown-linux-gnu`. Run formatting, workspace tests, target builds, bundle assembly, and artifact upload for each target.

- [ ] **Step 2: Add HTTP tool-call coverage**

Extend the HTTP test to call `tools/list`, `list_profiles`, and `health` through loopback. Use a temporary catalog and workspace root so the test does not touch a user directory.

- [ ] **Step 3: Run final verification locally**

```bash
rtk cargo fmt --all -- --check
rtk cargo test --workspace
rtk cargo build --release -p sp-mcp
rtk git diff --check
```

On Windows, additionally run the PowerShell launcher and bundled `typst.exe`/`pandoc.exe` smoke tests. On Linux/WSL, run the shell launcher and native binaries.

- [ ] **Step 4: Commit CI and acceptance coverage**

```bash
rtk git add .github/workflows/release-bundles.yml crates/sp-mcp/tests/http_transport.rs crates/sp-mcp/README.md
rtk git commit -m "ci: build ScholarPress OpenWork bundles"
```

## Self-Review

- Dual stdio/HTTP transport is covered by Task 1.
- Loopback binding and health endpoint are covered by Task 1 and Task 5.
- `.scholarpress` layout and complete path containment are covered by Task 2 and Task 4.
- Explicit/bundled/PATH executable resolution is covered by Task 3.
- Windows and Linux native binaries plus launchers are covered by Task 4 and Task 5.
- OpenWork manual URL registration is documented in Task 4 and tested in Task 5.
- Installer, authentication, macOS, ARM64, OpenWork fork, and API-ID cleanup remain explicitly out of scope.
- No unresolved placeholders or speculative new service layers are included.
