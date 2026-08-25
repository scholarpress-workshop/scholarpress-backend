# Goose Local `sp-mcp` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Package the existing stdio `sp-mcp` server for native Windows Goose Desktop and CLI use, with a safe PowerShell setup flow and isolated project workspaces.

**Architecture:** Goose launches the bundled `sp-mcp.exe` directly as a standard stdio extension. A PowerShell script validates the bundle and project, creates `.scholarpress\\workspaces`, then delegates YAML parsing and ScholarPress extension updates to a small `sp-mcp setup-goose` subcommand so arbitrary existing Goose configuration is not edited with regexes. Goose remains responsible for the server process lifetime.

**Tech Stack:** Rust 2021, Tokio, `serde_yaml`, PowerShell, Goose `config.yaml`, Windows x86_64 release bundles, existing Typst/Pandoc/catalog packaging.

## Global Constraints

- The implementation targets native Windows x86_64.
- Goose Desktop and Goose CLI use the shared `~/.config/goose/config.yaml` extension entry.
- `sp-mcp.exe` remains a standard stdio MCP server; no Goose SDK, Goose fork, or HTTP transport is added.
- The default project root is `<project>\\.scholarpress\\workspaces`.
- The bundled catalog is read-only by convention and can be replaced with `-CatalogPath`.
- Resolve tools in this order: explicit override, bundle-local `bin\\`, then `PATH`.
- Invoke external tools with `std::process::Command`, never through a shell.
- Reject workspace traversal, outside-root paths, output escapes, symlink escapes, junction escapes, and Windows reparse-point escapes.
- Preserve unrelated Goose extensions and back up `config.yaml` before updating it.
- Keep the existing OpenCode and other stdio workflows working.
- Do not add an installer, MSIX package, package-manager integration, HTTP transport, or automatic Goose directory publishing.

---

## File Map

- `crates/sp-mcp/Cargo.toml`: add YAML serialization support for the config updater.
- `crates/sp-mcp/src/main.rs`: dispatch the `setup-goose` utility before normal MCP startup.
- `crates/sp-mcp/src/goose_config.rs`: parse, validate, and update Goose's YAML extension entry.
- `crates/sp-mcp/src/lib.rs`: expose the setup command entry point without changing MCP service behavior.
- `crates/sp-mcp/tests/goose_config.rs`: config-preservation, backup, and invalid-YAML tests.
- `packaging/setup-goose.ps1`: validate bundle/project paths, create workspace root, and invoke the updater.
- `packaging/build-bundle.ps1`: package `setup-goose.ps1` instead of the OpenWork HTTP launcher in the Windows artifact.
- `packaging/README-WINDOWS.md`: document Goose Desktop/CLI setup and manual fallback.
- `packaging/start-scholarpress.ps1`: remove the obsolete OpenWork-specific Windows launcher.
- `.github/workflows/release-bundles.yml`: keep Windows bundle verification aligned with the new artifact.

## Task 1: Add A Tested Goose Config Updater

**Files:**
- Modify: `crates/sp-mcp/Cargo.toml`
- Create: `crates/sp-mcp/src/goose_config.rs`
- Modify: `crates/sp-mcp/src/lib.rs`
- Modify: `crates/sp-mcp/src/main.rs`
- Create: `crates/sp-mcp/tests/goose_config.rs`

**Interfaces:**
- `pub struct GooseSetup { pub config_path: PathBuf, pub command: PathBuf, pub catalog_path: PathBuf, pub workspace_root: PathBuf, pub typst_path: PathBuf, pub pandoc_path: PathBuf }`
- `pub fn update_config(setup: &GooseSetup) -> Result<PathBuf, GooseConfigError>` returns the backup path after atomically replacing the config.
- `sp-mcp setup-goose --config PATH --command PATH --catalog PATH --workspace-root PATH --typst PATH --pandoc PATH` updates the named config and prints the backup and extension paths.

- [ ] **Step 1: Add the YAML dependency and failing preservation tests**

Add `serde_yaml = "0.9"` to `crates/sp-mcp/Cargo.toml`. Test that an existing config containing an unrelated extension is preserved and that the generated ScholarPress entry has `type: stdio`, an empty `args` array, `enabled: true`, `timeout: 300`, and all four environment variables.

```rust
#[test]
fn update_preserves_other_extensions_and_writes_scholarpress() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    std::fs::write(&config, "extensions:\n  other:\n    cmd: other-tool\n    enabled: true\n").unwrap();
    let setup = test_setup(&config);

    let backup = update_config(&setup).unwrap();
    assert!(backup.is_file());
    let value: serde_yaml::Value = serde_yaml::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(value["extensions"]["other"]["cmd"], "other-tool");
    assert_eq!(value["extensions"]["scholarpress"]["type"], "stdio");
    assert_eq!(value["extensions"]["scholarpress"]["envs"]["SCHOLARPRESS_WORKSPACE_ROOT"], setup.workspace_root.to_string_lossy().as_ref());
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `rtk cargo test -p sp-mcp --test goose_config update_preserves_other_extensions_and_writes_scholarpress`

Expected: FAIL because the updater module and command do not exist.

- [ ] **Step 3: Implement typed config update and atomic replacement**

Load the config as `serde_yaml::Value`, treat a missing file as an empty mapping, require the top-level value to be a mapping, and require `extensions` to be either absent or a mapping. Insert or replace only `extensions.scholarpress` with the values below:

```rust
fn extension(setup: &GooseSetup) -> serde_yaml::Value {
    serde_yaml::from_value(serde_json::json!({
        "name": "ScholarPress",
        "description": "Format and validate dissertation documents with ScholarPress",
        "cmd": setup.command,
        "args": [],
        "enabled": true,
        "type": "stdio",
        "timeout": 300,
        "envs": {
            "SCHOLARPRESS_CATALOG_PATH": setup.catalog_path,
            "SCHOLARPRESS_WORKSPACE_ROOT": setup.workspace_root,
            "SCHOLARPRESS_TYPST_PATH": setup.typst_path,
            "SCHOLARPRESS_PANDOC_PATH": setup.pandoc_path
        }
    })).expect("static extension shape is valid YAML")
}
```

Write a timestamped sibling backup before replacing the original. Write YAML to a temporary file in the same directory, flush it, then rename it over the config so a failed serialization or write cannot leave a partial config. Return actionable errors for invalid YAML, wrong top-level shapes, and filesystem failures.

- [ ] **Step 4: Add the `setup-goose` command dispatch**

In `main`, inspect the first argument before `Config::from_env()`. When it is `setup-goose`, parse the six required option values, call `sp_mcp::setup_goose`, print the updated config and backup paths, and exit without starting an MCP server. Preserve the existing argument path for `--transport`, `--bind`, and `--port`.

- [ ] **Step 5: Run config tests and the full crate suite**

Run: `rtk cargo test -p sp-mcp --test goose_config`

Run: `rtk cargo test -p sp-mcp`

Expected: all tests pass, including existing stdio and HTTP regression coverage.

- [ ] **Step 6: Commit the config updater**

```bash
rtk git add crates/sp-mcp/Cargo.toml crates/sp-mcp/src/goose_config.rs crates/sp-mcp/src/lib.rs crates/sp-mcp/src/main.rs crates/sp-mcp/tests/goose_config.rs
rtk git commit -m "feat: add safe Goose config setup"
```

## Task 2: Add The Native Windows Setup Script

**Files:**
- Create: `packaging/setup-goose.ps1`
- Test: `packaging/tests/setup-goose.Tests.ps1` or an equivalent Windows shell smoke test

**Interfaces:**
- `-ProjectPath` is required.
- `-BundlePath` defaults to the directory containing the script.
- `-CatalogPath` defaults to `<BundlePath>\\catalog`.
- `-GooseConfigPath` defaults to `$HOME\\.config\\goose\\config.yaml`.
- `-StartGoose` optionally invokes `goose` in the project directory after configuration.

- [ ] **Step 1: Add a failing script smoke test**

Run the script against a temporary bundle containing stub `sp-mcp.exe`, `bin\\typst.exe`, `bin\\pandoc.exe`, and `catalog\\institutions`. Assert that `.scholarpress\\workspaces` and the Goose config are created, and that the config contains the resolved absolute paths.

- [ ] **Step 2: Implement path validation and derived paths**

Use `Resolve-Path` for existing project, bundle, and catalog paths. Require `sp-mcp.exe`, `bin\\typst.exe`, `bin\\pandoc.exe`, and the catalog directory unless an explicit override replaces the corresponding bundled tool/catalog. Create `<ProjectPath>\\.scholarpress\\workspaces` with `New-Item -Force`.

```powershell
$ProjectRoot = (Resolve-Path -LiteralPath $ProjectPath).Path
$BundleRoot = (Resolve-Path -LiteralPath $BundlePath).Path
$WorkspaceRoot = Join-Path $ProjectRoot ".scholarpress\workspaces"
$CatalogRoot = if ($CatalogPath) { (Resolve-Path -LiteralPath $CatalogPath).Path } else { Join-Path $BundleRoot "catalog" }
New-Item -ItemType Directory -Force -Path $WorkspaceRoot | Out-Null
```

- [ ] **Step 3: Invoke the updater with explicit paths**

Resolve the Goose config parent, server, tools, and catalog before changing the config. Invoke `sp-mcp.exe setup-goose` with one argument per option using PowerShell's argument array, not `cmd.exe` or a shell string. Propagate a nonzero exit code and print the updater's actionable error.

- [ ] **Step 4: Add optional Goose startup and safe output**

With `-StartGoose`, run `goose` with `-WorkingDirectory $ProjectRoot`; otherwise exit after printing the configured project, server, catalog, workspace, Typst, Pandoc, config, and backup paths. Never start `sp-mcp.exe` directly.

- [ ] **Step 5: Run the Windows smoke test**

Run on Windows: `powershell -NoProfile -ExecutionPolicy Bypass -File packaging/setup-goose.ps1 -Help`

Run the temporary-bundle test and verify that a second run updates only `extensions.scholarpress` while retaining unrelated entries.

- [ ] **Step 6: Commit the setup script**

```bash
rtk git add packaging/setup-goose.ps1 packaging/tests
rtk git commit -m "feat: add Windows Goose setup script"
```

## Task 3: Replace OpenWork Windows Packaging And Documentation

**Files:**
- Modify: `packaging/build-bundle.ps1`
- Delete: `packaging/start-scholarpress.ps1`
- Modify: `packaging/README-WINDOWS.md`
- Modify: `crates/sp-mcp/README.md`

**Interfaces:**
- The Windows artifact contains `setup-goose.ps1`, not the OpenWork HTTP launcher.
- The documented command is `setup-goose.ps1 -ProjectPath PATH`.
- Manual Goose Desktop and `goose configure` setup remain available.

- [ ] **Step 1: Update bundle assembly**

Copy `packaging/setup-goose.ps1` into the bundle and stop copying `start-scholarpress.ps1`. Keep `sp-mcp.exe`, `bin\\`, `catalog\\`, and the Windows README unchanged in layout.

- [ ] **Step 2: Rewrite the Windows README around Goose**

Document extraction, execution policy, `-ProjectPath`, `-BundlePath`, `-CatalogPath`, `-GooseConfigPath`, `-StartGoose`, the `.scholarpress` layout, shared Desktop/CLI configuration, manual UI fallback, and explicit tool overrides. Remove OpenWork URL instructions.

- [ ] **Step 3: Update the MCP crate README**

Replace the OpenWork section with the Goose setup flow and retain the existing stdio/OpenCode configuration. Explicitly state that Goose owns the `sp-mcp.exe` process and that no HTTP mode is required for Goose.

- [ ] **Step 4: Build the Windows bundle and inspect its contents**

Run on Windows: `rtk powershell -NoProfile -ExecutionPolicy Bypass -File packaging/build-bundle.ps1`

Verify the ZIP contains `sp-mcp.exe`, `bin\\typst.exe`, `bin\\pandoc.exe`, `catalog\\`, `setup-goose.ps1`, and `README-WINDOWS.md`, and does not contain `start-scholarpress.ps1`.

- [ ] **Step 5: Commit packaging and docs**

```bash
rtk git add packaging/build-bundle.ps1 packaging/README-WINDOWS.md crates/sp-mcp/README.md
rtk git rm packaging/start-scholarpress.ps1
rtk git commit -m "docs: package ScholarPress for Goose on Windows"
```

## Task 4: Add Windows CI And End-To-End Acceptance Coverage

**Files:**
- Modify: `.github/workflows/release-bundles.yml`
- Modify: `crates/sp-mcp/tests/goose_config.rs`
- Modify: `packaging/tests/setup-goose.Tests.ps1`

**Interfaces:**
- CI builds the Windows MSVC release binary and assembles the Goose bundle.
- Acceptance tests exercise the same stdio MCP protocol Goose uses.

- [ ] **Step 1: Add config edge-case tests**

Cover a missing config, invalid YAML, a scalar top-level value, a non-map `extensions` value, a pre-existing ScholarPress entry, and preservation of unrelated nested values. Assert that invalid input leaves the original file unchanged and creates no backup.

- [ ] **Step 2: Add stdio MCP acceptance coverage**

Start the release binary with temporary catalog and workspace environment variables, send MCP initialize and `tools/list` over stdin, and assert that the response advertises the existing ScholarPress tool set. Use the current service tests as the protocol baseline; do not add Goose-specific protocol code.

- [ ] **Step 3: Run the Windows setup and bundle jobs in CI**

Keep the existing `x86_64-pc-windows-msvc` build. Run the config tests, PowerShell setup smoke test, `cargo test --workspace`, bundle assembly, and artifact upload. Ensure the Windows job installs the same pinned Typst and Pandoc inputs used by the bundle script.

- [ ] **Step 4: Run final verification**

```bash
rtk cargo fmt --all -- --check
rtk cargo test --workspace
rtk cargo build --release -p sp-mcp
rtk git diff --check
```

On Windows additionally run the setup smoke test, build the target release binary, inspect the ZIP, and manually activate the extension once in Goose Desktop and once in Goose CLI.

- [ ] **Step 5: Commit CI and acceptance coverage**

```bash
rtk git add .github/workflows/release-bundles.yml crates/sp-mcp/tests/goose_config.rs packaging/tests
rtk git commit -m "test: verify Goose Windows integration"
```

## Self-Review

- Standard Goose stdio configuration is implemented in Task 1 and documented in Task 3.
- Both Desktop and CLI are covered because both consume the shared config, with manual acceptance in Task 4.
- Project-local `.scholarpress\\workspaces` isolation is created by Task 2 and enforced by the existing server boundary tests.
- Bundled catalog and explicit catalog/tool overrides are handled by Tasks 2 and 3.
- Goose process ownership is preserved because the setup script never launches `sp-mcp.exe` as a daemon.
- Invalid YAML, unrelated extension preservation, backups, and atomic replacement are covered by Tasks 1 and 4.
- The existing OpenWork HTTP launcher is removed from the Windows artifact and documentation in Task 3.
- Installer, Goose-native extension code, HTTP transport, non-Windows packaging, and extension-directory publishing remain excluded.
- No unresolved implementation placeholders remain.
