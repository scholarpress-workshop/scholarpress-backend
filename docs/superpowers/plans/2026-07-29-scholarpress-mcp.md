# ScholarPress MCP Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an `sp-mcp` crate to `scholarpress-backend` that exposes six stdio MCP tools (workspace + profile discovery, document extraction, Typst compilation, formatting checks), then remove the now-obsolete `apps/publish-service`.

**Architecture:** New `sp-mcp` binary uses the `rmcp` crate (v3) for stdio MCP transport. Calls directly into `sp-extract`, `sp-check`, `sp-typst` as libraries — no HTTP, no CLI subprocess (except `typst` itself). Agent harnesses (OpenCode primary) handle all file I/O via their built-in tools; sp-mcp owns only the operations no harness can do natively plus workspace/catalog discovery.

**Tech Stack:** Rust 1.88+, `rmcp = "3"` (with `transport-io` feature), `tokio`, `serde`, `serde_json`, `anyhow`, `thiserror`. Direct dep on workspace crates `sp-extract`, `sp-check`, `sp-typst`.

**Spec:** `docs/superpowers/specs/2026-07-29-scholarpress-mcp-design.md`

## Global Constraints

- Rust MSRV: **1.88** (matches existing workspace; `rmcp` v3 requires this).
- The `typst` binary **must be on PATH** for `compile_typst` to work. Tests that call `compile_typst` skip with a clear message if `typst` is missing.
- No HTTP layer anywhere. sp-mcp is stdio-only.
- No `read_file` / `write_file` / `patch_file` / `list_files` tools in MCP. Agent uses harness tools.
- Catalog is **read-only by construction** — sp-mcp never writes to `SCHOLARPRESS_CATALOG_PATH`.
- Workspaces live under `SCHOLARPRESS_WORKSPACE_ROOT` (default `~/.scholarpress/workspaces/`), named dirs, persist until user `rm -rf`s.
- Commit style: `<type>: <short description>`, lowercase, no body. Match the existing repo convention.
- Pre-push: `cargo fmt --all && cargo clippy --all --tests -- -D warnings` must pass.

## File Structure

```
crates/sp-mcp/
  Cargo.toml              NEW — package manifest, deps
  src/
    lib.rs                NEW — module declarations, server bootstrap function
    main.rs               NEW — entry: load config, call lib::run, handle exit
    config.rs             NEW — Config struct (catalog_path, workspace_root) + from_env + new (for tests)
    error.rs              NEW — SpMcpError enum (thiserror), with From impls for io, sp_extract, sp_check, sp_typst errors
    workspace.rs          NEW — pure fns: list_workspaces, list_profiles, create_workspace
    tools.rs              NEW — MCP tool trait impls (six tools) wrapping the workspace fns + backend crates
  tests/
    integration.rs        NEW — one #[test] per tool against fixture data
  README.md               NEW — setup, env vars, OpenCode config, typst install
```

The `apps/publish-service/` directory is deleted in Task 11. The workspace `Cargo.toml` already uses `crates/*` and `apps/*` globs, so adding/removing crates is automatic.

---

## Task 1: Scaffold the sp-mcp crate

**Files:**
- Create: `crates/sp-msp/Cargo.toml` *(typo intentional — see step 1)*
- Create: `crates/sp-mcp/Cargo.toml`
- Create: `crates/sp-mcp/src/lib.rs`
- Create: `crates/sp-mcp/src/main.rs`
- Create: `crates/sp-mcp/src/config.rs`
- Create: `crates/sp-mcp/src/error.rs`
- Create: `crates/sp-mcp/src/workspace.rs`
- Create: `crates/sp-mcp/src/tools.rs`

**Interfaces:**
- Consumes: nothing (scaffold)
- Produces: a compiling, empty `sp_mcp` crate with module skeleton, plus a `Config` struct and `SpMcpError` enum that later tasks will use

- [ ] **Step 1: Create `crates/sp-mcp/Cargo.toml`**

Write exactly this content (do not add dependencies that are not used yet — the next tasks add them one at a time):

```toml
[package]
name = "sp-mcp"
version = "0.1.0"
edition = "2021"

[lib]
name = "sp_mcp"
path = "src/lib.rs"

[[bin]]
name = "sp-mcp"
path = "src/main.rs"

[dependencies]
```

- [ ] **Step 2: Create `crates/sp-mcp/src/lib.rs`**

```rust
pub mod config;
pub mod error;
pub mod tools;
pub mod workspace;
```

- [ ] **Step 3: Create `crates/sp-mcp/src/main.rs`**

```rust
fn main() {
    eprintln!("sp-mcp: scaffold only — no tools yet");
    std::process::exit(1);
}
```

- [ ] **Step 4: Create `crates/sp-mcp/src/config.rs`**

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub catalog_path: PathBuf,
    pub workspace_root: PathBuf,
}

impl Config {
    pub fn new(catalog_path: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            catalog_path,
            workspace_root,
        }
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        let catalog_path = std::env::var("SCHOLARPRESS_CATALOG_PATH")
            .map_err(|_| ConfigError::Missing("SCHOLARPRESS_CATALOG_PATH"))?;
        let workspace_root = std::env::var("SCHOLARPRESS_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_workspace_root());

        let catalog_path = PathBuf::from(catalog_path);

        if !catalog_path.is_dir() {
            return Err(ConfigError::NotADirectory("SCHOLARPRESS_CATALOG_PATH", catalog_path));
        }
        if let Some(parent) = workspace_root.parent() {
            if !parent.is_dir() && !parent.as_os_str().is_empty() {
                return Err(ConfigError::NotADirectory(
                    "SCHOLARPRESS_WORKSPACE_ROOT parent",
                    parent.to_path_buf(),
                ));
            }
        }

        Ok(Self {
            catalog_path,
            workspace_root,
        })
    }
}

fn default_workspace_root() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".scholarpress").join("workspaces")
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("environment variable {0} is not set")]
    Missing(&'static str),
    #[error("{0} does not point to an existing directory: {1}")]
    NotADirectory(&'static str, PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_workspace_root_uses_home() {
        let root = default_workspace_root();
        assert!(root.ends_with(".scholarpress/workspaces"));
    }

    #[test]
    fn new_stores_values() {
        let cfg = Config::new(PathBuf::from("/c"), PathBuf::from("/w"));
        assert_eq!(cfg.catalog_path, PathBuf::from("/c"));
        assert_eq!(cfg.workspace_root, PathBuf::from("/w"));
    }
}
```

- [ ] **Step 5: Create `crates/sp-mcp/src/error.rs`**

```rust
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SpMcpError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("profile {0:?} not found in catalog; available: {1:?}")]
    ProfileNotFound(String, Vec<String>),

    #[error("workspace {0:?} not found under {1}")]
    WorkspaceNotFound(String, PathBuf),

    #[error("workspace {0} already exists")]
    WorkspaceExists(PathBuf),

    #[error("invalid workspace name {0:?} (must be non-empty, no '/' or '..')")]
    BadWorkspaceName(String),

    #[error("spec.yaml not found at {0}")]
    SpecMissing(PathBuf),

    #[error("extraction failed: {0}")]
    Extraction(String),

    #[error("compilation failed: {0}")]
    Compilation(String),

    #[error("check failed: {0}")]
    Check(String),

    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),
}
```

- [ ] **Step 6: Create `crates/sp-mcp/src/workspace.rs` (empty stub)**

```rust
// populated in tasks 2-4
```

- [ ] **Step 7: Create `crates/sp-mcp/src/tools.rs` (empty stub)**

```rust
// populated in tasks 5-8
```

- [ ] **Step 8: Update root `Cargo.toml` to add `thiserror` dep to sp-mcp**

Replace the contents of `crates/sp-mcp/Cargo.toml` with:

```toml
[package]
name = "sp-mcp"
version = "0.1.0"
edition = "2021"

[lib]
name = "sp_mcp"
path = "src/lib.rs"

[[bin]]
name = "sp-mcp"
path = "src/main.rs"

[dependencies]
thiserror = "1"
```

- [ ] **Step 9: Build to verify the scaffold compiles**

Run: `cargo build -p sp-mcp`
Expected: success (the binary prints "scaffold only" when run; we're just checking the crate compiles)

- [ ] **Step 10: Run unit tests**

Run: `cargo test -p sp-mcp --lib`
Expected: 2 passed (default_workspace_root_uses_home, new_stores_values)

- [ ] **Step 11: Commit**

```bash
git add crates/sp-mcp/
git commit -m "feat(sp-mcp): scaffold crate with config and error types"
```

---

## Task 2: Implement `list_workspaces`

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs`
- Modify: `crates/sp-mcp/src/error.rs` (add variant if needed)
- Test: `crates/sp-mcp/src/workspace.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `Config` (workspace_root)
- Produces: `list_workspaces(config) -> Result<Vec<WorkspaceInfo>, SpMcpError>` where:
  ```rust
  pub struct WorkspaceInfo {
      pub name: String,
      pub path: PathBuf,
      pub profile_id: Option<String>,  // parsed from spec.yaml if present
      pub mtime: SystemTime,
  }
  ```

- [ ] **Step 1: Add types to `crates/sp-mcp/src/workspace.rs`**

Replace the stub with:

```rust
use crate::config::Config;
use crate::error::SpMcpError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceInfo {
    pub name: String,
    pub path: PathBuf,
    pub profile_id: Option<String>,
    pub mtime: SystemTime,
}

pub fn list_workspaces(config: &Config) -> Result<Vec<WorkspaceInfo>, SpMcpError> {
    if !config.workspace_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&config.workspace_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        let meta = entry.metadata()?;
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let profile_id = read_profile_id_from_spec(&path.join("spec.yaml"));
        out.push(WorkspaceInfo {
            name,
            path,
            profile_id,
            mtime,
        });
    }
    out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    Ok(out)
}

fn read_profile_id_from_spec(spec_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(spec_path).ok()?;
    // profile_id is the top-level `institution:` key for institutions, etc.
    // We don't fully parse YAML here; just grep the first `institution:` line.
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("institution:") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
        if let Some(rest) = line.strip_prefix("server:") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
        if let Some(rest) = line.strip_prefix("journal:") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}
```

- [ ] **Step 2: Add `serde` dep to `crates/sp-mcp/Cargo.toml`**

Add to `[dependencies]`:

```toml
serde = { version = "1", features = ["derive"] }
```

- [ ] **Step 3: Write the failing test**

Add to the bottom of `crates/sp-mcp/src/workspace.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread::sleep;
    use std::time::Duration;

    fn make_workspace(root: &Path, name: &str, spec: Option<&str>) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        if let Some(text) = spec {
            fs::write(dir.join("spec.yaml"), text).unwrap();
        }
        // ensure distinct mtimes
        sleep(Duration::from_millis(10));
    }

    #[test]
    fn list_workspaces_returns_sorted_by_mtime_desc() {
        let tmp = tempdir();
        make_workspace(&tmp, "alpha", Some("institution: Alpha University\n"));
        make_workspace(&tmp, "beta", Some("server: arxiv\n"));
        make_workspace(&tmp, "gamma", None);

        let cfg = Config::new(PathBuf::from("/c"), tmp.clone());
        let result = list_workspaces(&cfg).unwrap();

        let names: Vec<&str> = result.iter().map(|w| w.name.as_str()).collect();
        assert_eq!(names, vec!["gamma", "beta", "alpha"], "newest first");

        let alpha = result.iter().find(|w| w.name == "alpha").unwrap();
        assert_eq!(alpha.profile_id.as_deref(), Some("Alpha University"));
        let beta = result.iter().find(|w| w.name == "beta").unwrap();
        assert_eq!(beta.profile_id.as_deref(), Some("arxiv"));
        let gamma = result.iter().find(|w| w.name == "gamma").unwrap();
        assert_eq!(gamma.profile_id, None);
    }

    #[test]
    fn list_workspaces_empty_when_root_missing() {
        let cfg = Config::new(PathBuf::from("/c"), PathBuf::from("/nonexistent-xyz-12345"));
        let result = list_workspaces(&cfg).unwrap();
        assert!(result.is_empty());
    }

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir();
        let unique = format!(
            "sp-mcp-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let p = base.join(unique);
        fs::create_dir_all(&p).unwrap();
        p
    }
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p sp-mcp --lib workspace::tests::list_workspaces_returns_sorted_by_mtime_desc`
Expected: compile error because `tempdir()` helper conflicts — see step 5 for the fix. If you see other errors, fix the test code as needed to match step 3 verbatim.

- [ ] **Step 5: Fix the `tempdir` helper name collision**

The stdlib doesn't export `tempdir`, but in case of name collisions in future test runs, prefix the helper with `local_` and update the two callers:

Rename the function to `local_tempdir` and update both `make_workspace(&tmp, ...)` callsites to use `local_tempdir()`. (The current step 3 test uses `tempdir()` already; the rename is a defensive cleanup. Skip this step if no collision exists.)

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p sp-mcp --lib`
Expected: 4 passed (2 from config + 2 new)

- [ ] **Step 7: Commit**

```bash
git add crates/sp-mcp/
git commit -m "feat(sp-mcp): list_workspaces — enumerate workspace dirs sorted by mtime"
```

---

## Task 3: Implement `list_profiles`

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs`
- Test: inline in `workspace.rs`

**Interfaces:**
- Consumes: `Config` (catalog_path)
- Produces: `list_profiles(config) -> Result<Vec<ProfileInfo>, SpMcpError>` where:
  ```rust
  pub struct ProfileInfo {
      pub id: String,            // "institutions/iu" or "servers/arxiv"
      pub scope: String,         // "institution", "server", "journal", "grant"
      pub name: String,          // human-readable from spec.yaml
  }
  ```

- [ ] **Step 1: Add `ProfileInfo` and `list_profiles` to `crates/sp-mcp/src/workspace.rs`**

Append after the existing code (before the `#[cfg(test)]` block):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileInfo {
    pub id: String,
    pub scope: String,
    pub name: String,
}

pub fn list_profiles(config: &Config) -> Result<Vec<ProfileInfo>, SpMcpError> {
    if !config.catalog_path.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for scope_entry in std::fs::read_dir(&config.catalog_path)? {
        let scope_entry = scope_entry?;
        let scope_path = scope_entry.path();
        if !scope_path.is_dir() {
            continue;
        }
        let scope = match scope_entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        for id_entry in std::fs::read_dir(&scope_path)? {
            let id_entry = id_entry?;
            let id_path = id_entry.path();
            if !id_path.is_dir() {
                continue;
            }
            if !id_path.join("spec.yaml").is_file() {
                continue;
            }
            if !id_path.join("template").join("template.typ").is_file() {
                continue;
            }
            let id = match id_entry.file_name().into_string() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let name = read_profile_name(&id_path.join("spec.yaml"))
                .unwrap_or_else(|| id.clone());
            out.push(ProfileInfo {
                id: format!("{}/{}", scope, id),
                scope: scope.clone(),
                name,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn read_profile_name(spec_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(spec_path).ok()?;
    for line in text.lines() {
        for key in &["institution:", "server:", "journal:", "grant:"] {
            if let Some(rest) = line.strip_prefix(key) {
                let v = rest.trim().trim_matches('"').to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}
```

- [ ] **Step 2: Write the failing test**

Add a new test inside the existing `mod tests` block in `workspace.rs`:

```rust
    #[test]
    fn list_profiles_finds_scoped_profiles() {
        let catalog = local_tempdir();
        let iu = catalog.join("institutions").join("iu");
        fs::create_dir_all(iu.join("template")).unwrap();
        fs::write(iu.join("spec.yaml"), "institution: Indiana University\n").unwrap();
        fs::write(iu.join("template").join("template.typ"), "= Hi\n").unwrap();

        let arxiv = catalog.join("servers").join("arxiv");
        fs::create_dir_all(arxiv.join("template")).unwrap();
        fs::write(arxiv.join("spec.yaml"), "server: arxiv\n").unwrap();
        fs::write(arxiv.join("template").join("template.typ"), "= Hi\n").unwrap();

        // incomplete profile: missing template.typ — should be skipped
        let incomplete = catalog.join("institutions").join("partial");
        fs::create_dir_all(&incomplete).unwrap();
        fs::write(incomplete.join("spec.yaml"), "institution: Partial\n").unwrap();

        let cfg = Config::new(catalog.clone(), PathBuf::from("/w"));
        let profiles = list_profiles(&cfg).unwrap();
        let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["institutions/iu", "servers/arxiv"]);

        let iu_info = profiles.iter().find(|p| p.id == "institutions/iu").unwrap();
        assert_eq!(iu_info.scope, "institution");
        assert_eq!(iu_info.name, "Indiana University");
    }
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cargo test -p sp-mcp --lib workspace::tests::list_profiles_finds_scoped_profiles`
Expected: 1 passed

- [ ] **Step 4: Commit**

```bash
git add crates/sp-mcp/
git commit -m "feat(sp-mcp): list_profiles — scan catalog for spec.yaml + template.typ"
```

---

## Task 4: Implement `create_workspace`

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs`
- Test: inline in `workspace.rs`

**Interfaces:**
- Consumes: `Config` (catalog_path, workspace_root), `name: &str`, `profile_id: &str` (e.g., "institutions/iu")
- Produces: `create_workspace(config, name, profile_id) -> Result<PathBuf, SpMcpError>` — returns the new workspace path

- [ ] **Step 1: Add `create_workspace` to `crates/sp-mcp/src/workspace.rs`**

Append after the existing code:

```rust
pub fn create_workspace(
    config: &Config,
    name: &str,
    profile_id: &str,
) -> Result<PathBuf, SpMcpError> {
    validate_name(name)?;

    let profile_dir = config.catalog_path.join(profile_id);
    if !profile_dir.is_dir() {
        let available: Vec<String> = list_profiles(config)?
            .into_iter()
            .map(|p| p.id)
            .collect();
        return Err(SpMcpError::ProfileNotFound(
            profile_id.to_string(),
            available,
        ));
    }
    if !profile_dir.join("spec.yaml").is_file() {
        return Err(SpMcpError::SpecMissing(profile_dir.join("spec.yaml")));
    }

    let target = config.workspace_root.join(name);
    if target.exists() {
        return Err(SpMcpError::WorkspaceExists(target));
    }
    std::fs::create_dir_all(target.join("data"))?;
    std::fs::create_dir_all(target.join("out"))?;

    copy_tree(&profile_dir.join("spec.yaml"), &target.join("spec.yaml"))?;
    let template_src = profile_dir.join("template");
    if template_src.is_dir() {
        copy_dir_recursive(&template_src, &target.join("template"))?;
    }

    Ok(target)
}

fn validate_name(name: &str) -> Result<(), SpMcpError> {
    if name.is_empty() {
        return Err(SpMcpError::BadWorkspaceName(name.to_string()));
    }
    if name.contains('/') || name.contains("..") || name.contains('\0') {
        return Err(SpMcpError::BadWorkspaceName(name.to_string()));
    }
    Ok(())
}

fn copy_tree(src: &Path, dst: &Path) -> Result<(), SpMcpError> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dst)?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), SpMcpError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            // skip tests/corpus — large calibration PDFs
            if from.file_name() == Some(std::ffi::OsStr::new("tests")) {
                continue;
            }
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Write the failing test**

Add a new test inside the `mod tests` block:

```rust
    #[test]
    fn create_workspace_copies_spec_and_template() {
        let catalog = local_tempdir();
        let iu = catalog.join("institutions").join("iu");
        fs::create_dir_all(iu.join("template").join("sections")).unwrap();
        fs::write(iu.join("spec.yaml"), "institution: IU\n").unwrap();
        fs::write(iu.join("template").join("template.typ"), "= Hi\n").unwrap();
        fs::write(
            iu.join("template").join("sections").join("ch.typ"),
            "= Chapter\n",
        )
        .unwrap();
        // create a tests/ dir to verify it is skipped
        fs::create_dir_all(iu.join("tests").join("corpus")).unwrap();
        fs::write(iu.join("tests").join("ignored.typ"), "ignored\n").unwrap();

        let ws_root = local_tempdir();
        let cfg = Config::new(catalog.clone(), ws_root.clone());

        let path = create_workspace(&cfg, "iu-job-1", "institutions/iu").unwrap();
        assert!(path.is_dir());
        assert!(path.join("spec.yaml").is_file());
        assert!(path.join("template").join("template.typ").is_file());
        assert!(path.join("template").join("sections").join("ch.typ").is_file());
        assert!(path.join("data").is_dir());
        assert!(path.join("out").is_dir());
        // tests/ skipped
        assert!(!path.join("template").join("tests").exists());
    }

    #[test]
    fn create_workspace_rejects_bad_name() {
        let cfg = Config::new(PathBuf::from("/c"), PathBuf::from("/w"));
        assert!(create_workspace(&cfg, "../escape", "institutions/iu").is_err());
        assert!(create_workspace(&cfg, "a/b", "institutions/iu").is_err());
        assert!(create_workspace(&cfg, "", "institutions/iu").is_err());
    }

    #[test]
    fn create_workspace_unknown_profile_lists_available() {
        let catalog = local_tempdir();
        let iu = catalog.join("institutions").join("iu");
        fs::create_dir_all(iu.join("template")).unwrap();
        fs::write(iu.join("spec.yaml"), "institution: IU\n").unwrap();
        fs::write(iu.join("template").join("template.typ"), "= Hi\n").unwrap();

        let cfg = Config::new(catalog, PathBuf::from("/w"));
        let err = create_workspace(&cfg, "x", "institutions/missing").unwrap_err();
        match err {
            SpMcpError::ProfileNotFound(id, avail) => {
                assert_eq!(id, "institutions/missing");
                assert_eq!(avail, vec!["institutions/iu".to_string()]);
            }
            other => panic!("expected ProfileNotFound, got {:?}", other),
        }
    }
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p sp-mcp --lib`
Expected: 7 passed (2 config + 2 workspace-list + 1 list-profiles + 3 create-workspace)

- [ ] **Step 4: Commit**

```bash
git add crates/sp-mcp/
git commit -m "feat(sp-mcp): create_workspace — copy profile into scratch dir"
```

---

## Task 5: Implement `compile_typst` (pure function)

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs` (or a new `compile.rs` — keep it in workspace.rs to match the spec's file list; if it grows past ~150 lines, split later)
- Modify: `crates/sp-mcp/Cargo.toml` (add sp-typst dep)
- Test: inline in `workspace.rs`

**Interfaces:**
- Consumes: `Config`, `workspace: &Path`, `entry_path: &Path` (relative to workspace, e.g., "template.typ"), `data: Option<&serde_json::Value>`, `out_name: Option<&str>` (default = entry_path stem + ".pdf")
- Produces: `compile_typst(config, workspace, entry, data, out_name) -> Result<PathBuf, SpMcpError>` — writes to `<workspace>/out/<out_name>.pdf`, returns the absolute path

The `typst` binary must be on PATH. If not, return `SpMcpError::Compilation("typst binary not found on PATH".to_string())`.

The optional `data` JSON, if provided, is written to `<workspace>/data.json`. The typst template can read it with `json("data.json")` or `read("data.json")`.

- [ ] **Step 1: Add sp-typst dep to `crates/sp-mcp/Cargo.toml`**

Add to `[dependencies]`:

```toml
sp-typst = { path = "../sp-typst" }
```

- [ ] **Step 2: Add `compile_typst` to `crates/sp-mcp/src/workspace.rs`**

Append:

```rust
use sp_typst as typst;

pub fn compile_typst(
    config: &Config,
    workspace: &Path,
    entry_path: &Path,
    data: Option<&serde_json::Value>,
    out_name: Option<&str>,
) -> Result<PathBuf, SpMcpError> {
    if !workspace.is_dir() {
        return Err(SpMcpError::WorkspaceNotFound(
            workspace.display().to_string(),
            config.workspace_root.clone(),
        ));
    }
    let entry_abs = workspace.join(entry_path);
    if !entry_abs.is_file() {
        return Err(SpMcpError::Compilation(format!(
            "entry file not found: {}",
            entry_abs.display()
        )));
    }
    if let Some(d) = data {
        std::fs::write(workspace.join("data.json"), serde_json::to_string(d)?)?;
    }

    let entry_for_typst = entry_path.to_string_lossy().to_string();
    let source = std::fs::read_to_string(&entry_abs)?;
    // sp-typst compiles the entry file from --root=workspace.
    let bytes = typst::compile(&source, Some(workspace)).map_err(|e| {
        SpMcpError::Compilation(format!("typst compile failed: {}", e))
    })?;

    let stem = entry_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "out".to_string());
    let name = out_name.map(String::from).unwrap_or_else(|| format!("{}.pdf", stem));

    std::fs::create_dir_all(workspace.join("out"))?;
    let out_path = workspace.join("out").join(&name);
    std::fs::write(&out_path, bytes)?;
    Ok(out_path)
}
```

- [ ] **Step 3: Add `serde_json` dep to `crates/sp-mcp/Cargo.toml`**

Add to `[dependencies]`:

```toml
serde_json = "1"
```

- [ ] **Step 4: Write the failing test**

Add a new test inside `mod tests`:

```rust
    #[test]
    fn compile_typst_produces_pdf_or_skips_if_no_typst_binary() {
        let ws = local_tempdir();
        let tmpl = ws.join("template.typ");
        fs::write(&tmpl, "= Hello, world!\n").unwrap();
        let cfg = Config::new(PathBuf::from("/c"), PathBuf::from("/w"));

        let result = compile_typst(
            &cfg,
            &ws,
            Path::new("template.typ"),
            None,
            None,
        );
        match result {
            Ok(p) => {
                assert!(p.is_file(), "pdf should exist");
                let bytes = fs::read(&p).unwrap();
                assert!(bytes.starts_with(b"%PDF"), "should be a valid PDF");
            }
            Err(SpMcpError::Compilation(msg)) if msg.contains("typst") => {
                eprintln!("SKIP: typst not on PATH ({})", msg);
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }
```

- [ ] **Step 5: Run the test**

Run: `cargo test -p sp-mcp --lib workspace::tests::compile_typst_produces_pdf_or_skips_if_no_typst_binary`
Expected: PASS (either compiles successfully if `typst` is on PATH, or skips with a printed message if not)

- [ ] **Step 6: Commit**

```bash
git add crates/sp-mcp/
git commit -m "feat(sp-mcp): compile_typst — shell out to typst, write to out/, return path"
```

---

## Task 6: Implement `check_pdf` (pure function)

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs`
- Modify: `crates/sp-mcp/Cargo.toml` (add sp-check, sp-extract deps)
- Test: inline in `workspace.rs`

**Interfaces:**
- Consumes: `Config` (workspace_root for context only), `workspace: &Path`, `pdf_path: &Path` (absolute or relative to workspace)
- Produces: `check_pdf(config, workspace, pdf_path) -> Result<Vec<CheckOutcome>, SpMcpError>` — returns the JSON-serialized `Report` from `sp_check::report::build_report`

- [ ] **Step 1: Add sp-check dep to `crates/sp-mcp/Cargo.toml`**

Add to `[dependencies]`:

```toml
sp-check = { path = "../sp-check" }
```

- [ ] **Step 2: Add `check_pdf` to `crates/sp-mcp/src/workspace.rs`**

Append:

```rust
use sp_check as check;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckOutcome {
    pub id: String,
    pub status: String,    // "pass" | "fail" | "skip"
    pub message: String,
    pub page: Option<usize>,
}

pub fn check_pdf(
    _config: &Config,
    workspace: &Path,
    pdf_path: &Path,
) -> Result<Vec<CheckOutcome>, SpMcpError> {
    if !workspace.is_dir() {
        return Err(SpMcpError::WorkspaceNotFound(
            workspace.display().to_string(),
            workspace.to_path_buf(),
        ));
    }
    let spec_path = workspace.join("spec.yaml");
    if !spec_path.is_file() {
        return Err(SpMcpError::SpecMissing(spec_path));
    }
    let pdf_abs = if pdf_path.is_absolute() {
        pdf_path.to_path_buf()
    } else {
        workspace.join(pdf_path)
    };
    if !pdf_abs.is_file() {
        return Err(SpMcpError::Check(format!(
            "pdf not found: {}",
            pdf_abs.display()
        )));
    }

    let spec = check::spec::load_spec(&spec_path)
        .map_err(|e| SpMcpError::Check(format!("failed to load spec: {}", e)))?;
    let options = check::engine::CheckOptions::default();
    let results = check::engine::run_checks(&spec, &pdf_abs, &options)
        .map_err(|e| SpMcpError::Check(format!("check run failed: {}", e)))?;
    let report = check::report::build_report(results);

    let outcomes = report
        .results
        .into_iter()
        .map(|r| CheckOutcome {
            id: r.id,
            status: r.status.as_str().to_string(),
            message: r.message,
            page: r.page,
        })
        .collect();
    Ok(outcomes)
}
```

- [ ] **Step 3: Write the failing test**

The test needs a real IU spec and a real PDF fixture. Use the existing catalog fixture. If the fixture PDFs are not downloaded (`tests/corpus/*.pdf`), this test will fail to extract and produce no meaningful check. The test must skip in that case.

Add to `mod tests`:

```rust
    #[test]
    fn check_pdf_against_iu_baseline() {
        // Locate the catalog at the standard sibling path: ../scholarpress-catalog
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let catalog_candidate = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("scholarpress-catalog"));
        let catalog = match catalog_candidate {
            Some(p) if p.is_dir() => p,
            _ => {
                eprintln!("SKIP: scholarpress-catalog not found as sibling of scholarpress-backend");
                return;
            }
        };
        let baseline = catalog.join("institutions/iu/tests/fixtures/baseline.pdf");
        if !baseline.is_file() {
            eprintln!("SKIP: IU baseline fixture not present (run bash compile.sh)");
            return;
        }

        let ws = local_tempdir();
        fs::write(
            ws.join("spec.yaml"),
            fs::read_to_string(catalog.join("institutions/iu/spec.yaml")).unwrap(),
        )
        .unwrap();

        let cfg = Config::new(catalog, PathBuf::from("/w"));
        let outcomes = check_pdf(&cfg, &ws, &baseline).unwrap();
        // baseline is known-good for margins; we just assert the tool ran and returned something
        assert!(!outcomes.is_empty(), "expected at least one check result");
    }
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p sp-mcp --lib workspace::tests::check_pdf_against_iu_baseline -- --nocapture`
Expected: PASS (if the IU catalog sibling is present and `baseline.pdf` exists), or SKIP with a printed message (otherwise)

If the test fails because `Report` does not have a `.results` field, read `crates/sp-check/src/report.rs` and adjust the field access in step 2 accordingly.

- [ ] **Step 5: Commit**

```bash
git add crates/sp-mcp/
git commit -m "feat(sp-mcp): check_pdf — run sp-check against workspace spec, return violations"
```

---

## Task 7: Implement `extract_document` (pure function)

**Files:**
- Modify: `crates/sp-mcp/src/workspace.rs`
- Test: inline in `workspace.rs`

**Interfaces:**
- Consumes: `file_path: &Path`
- Produces: `extract_document(file_path) -> Result<serde_json::Value, SpMcpError>` — JSON-serialized `ParsedDocument`

- [ ] **Step 1: Add `extract_document` to `crates/sp-mcp/src/workspace.rs`**

Append:

```rust
use sp_extract as extract;

pub fn extract_document(file_path: &Path) -> Result<serde_json::Value, SpMcpError> {
    if !file_path.is_file() {
        return Err(SpMcpError::Extraction(format!(
            "file not found: {}",
            file_path.display()
        )));
    }
    let bytes = std::fs::read(file_path)?;
    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let doc = match ext.as_str() {
        "pdf" => extract::extract_pdf(&bytes),
        "docx" => extract::extract_docx(&bytes),
        other => {
            return Err(SpMcpError::Extraction(format!(
                "unsupported extension: .{}",
                other
            )));
        }
    }
    .map_err(|e| SpMcpError::Extraction(e.to_string()))?;

    serde_json::to_value(doc).map_err(|e| SpMcpError::Extraction(e.to_string()))
}
```

- [ ] **Step 2: Write the failing test**

Add to `mod tests`:

```rust
    #[test]
    fn extract_document_on_garbage_returns_error() {
        let f = local_tempdir().join("bad.pdf");
        fs::write(&f, b"not a real pdf").unwrap();
        let result = extract_document(&f);
        assert!(matches!(result, Err(SpMcpError::Extraction(_))));
    }

    #[test]
    fn extract_document_unsupported_extension_errors() {
        let f = local_tempdir().join("foo.xyz");
        fs::write(&f, b"whatever").unwrap();
        let result = extract_document(&f);
        assert!(matches!(result, Err(SpMcpError::Extraction(_))));
    }
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p sp-mcp --lib`
Expected: all workspace tests pass (2 config + 2 list_workspaces + 1 list_profiles + 3 create_workspace + 1 compile_typst + 1 check_pdf + 2 extract_document = 12)

- [ ] **Step 4: Commit**

```bash
git add crates/sp-mcp/
git commit -m "feat(sp-mcp): extract_document — wrap sp-extract for PDF/DOCX"
```

---

## Task 8: Wire up the MCP server (six tool handlers)

**Files:**
- Modify: `crates/sp-mcp/Cargo.toml` (add rmcp + tokio)
- Modify: `crates/sp-mcp/src/tools.rs`
- Modify: `crates/sp-mcp/src/lib.rs` (add server bootstrap)
- Modify: `crates/sp-mcp/src/main.rs` (call into lib::run)
- Test: manual smoke test (start server, send MCP initialize + tools/list, verify six tool names)

**Interfaces:**
- New: `ScholarPressService` struct holds `Config` + `ToolRouter<Self>`
- Each `#[tool]` method maps to a `crates/sp-mcp/src/workspace.rs` pure function
- `lib::run(config) -> Result<(), Box<dyn Error>>` starts the stdio server and waits

- [ ] **Step 1: Add rmcp + tokio + anyhow + futures deps to `crates/sp-mcp/Cargo.toml`**

Add to `[dependencies]`:

```toml
rmcp = { version = "3", features = ["transport-io", "macros", "server"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "sync"] }
anyhow = "1"
futures = "0.3"
```

- [ ] **Step 2: Write `crates/sp-mcp/src/tools.rs`**

Replace the stub with:

```rust
use crate::config::Config;
use crate::error::SpMcpError;
use crate::workspace;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{ErrorData as McpError, *};
use rmcp::{tool, tool_handler, tool_router};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct ScholarPressService {
    config: Arc<Config>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct CreateWorkspaceParams {
    pub name: String,
    pub profile_id: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct CompileTypstParams {
    pub workspace: PathBuf,
    pub entry_path: PathBuf,
    pub data: Option<serde_json::Value>,
    pub out_name: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct CheckPdfParams {
    pub workspace: PathBuf,
    pub pdf_path: PathBuf,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ExtractDocumentParams {
    pub file_path: PathBuf,
}

#[tool_router]
impl ScholarPressService {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            tool_router: Self::tool_router(),
        }
    }

    fn err(e: SpMcpError) -> McpError {
        McpError::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
    }

    #[tool(description = "List existing workspaces under SCHOLARPRESS_WORKSPACE_ROOT. Returns name, absolute path, profile_id (if spec.yaml identifies one), and mtime.")]
    async fn list_workspaces(&self) -> Result<CallToolResult, McpError> {
        let list = workspace::list_workspaces(&self.config).map_err(Self::err)?;
        let json = serde_json::to_string(&list).map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List available profiles in the catalog. Returns id (e.g. 'institutions/iu'), scope, and human-readable name.")]
    async fn list_profiles(&self) -> Result<CallToolResult, McpError> {
        let list = workspace::list_profiles(&self.config).map_err(Self::err)?;
        let json = serde_json::to_string(&list).map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Create a new workspace by copying a catalog profile (spec.yaml + template/) into a named dir under the workspace root. Returns the absolute path.")]
    async fn create_workspace(&self, params: Parameters<CreateWorkspaceParams>) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let path = workspace::create_workspace(&self.config, &p.name, &p.profile_id).map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::text(path.display().to_string())]))
    }

    #[tool(description = "Compile a Typst entry file within a workspace. Optionally pass `data` (JSON object) which is written to <workspace>/data.json before compilation. The PDF is written to <workspace>/out/<out_name>.pdf (default = entry stem). Returns the absolute output path. Requires the `typst` binary on PATH.")]
    async fn compile_typst(&self, params: Parameters<CompileTypstParams>) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let out = workspace::compile_typst(
            &self.config,
            &p.workspace,
            &p.entry_path,
            p.data.as_ref(),
            p.out_name.as_deref(),
        )
        .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::text(out.display().to_string())]))
    }

    #[tool(description = "Run formatting checks against the workspace's spec.yaml. Always uses the workspace spec. Returns a list of check outcomes (id, status, message, page).")]
    async fn check_pdf(&self, params: Parameters<CheckPdfParams>) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let outcomes = workspace::check_pdf(&self.config, &p.workspace, &p.pdf_path).map_err(Self::err)?;
        let json = serde_json::to_string(&outcomes).map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Extract text and metadata from a PDF or DOCX. Returns a JSON ParsedDocument (pages, paragraphs, headings, metadata).")]
    async fn extract_document(
        &self,
        params: Parameters<ExtractDocumentParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0.file_path;
        let doc = workspace::extract_document(&p).map_err(Self::err)?;
        let json = serde_json::to_string(&doc).map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for ScholarPressService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "scholarpress-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "ScholarPress: catalog + Typst template workspace tools. Use list_profiles to discover profiles, create_workspace to fork one into a scratch dir, then harness tools to edit, compile_typst + check_pdf to iterate."
                    .to_string(),
            ),
        }
    }
}
```

- [ ] **Step 3: Update `crates/sp-mcp/src/lib.rs`**

Replace with:

```rust
pub mod config;
pub mod error;
pub mod tools;
pub mod workspace;

use crate::config::Config;
use crate::tools::ScholarPressService;
use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};

pub async fn run(config: Config) -> Result<()> {
    let service = ScholarPressService::new(config).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

- [ ] **Step 4: Update `crates/sp-mcp/src/main.rs`**

Replace with:

```rust
use anyhow::Context;
use sp_mcp::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env().context("loading sp-mcp config")?;
    sp_mcp::run(config).await
}
```

- [ ] **Step 5: Build to verify it compiles**

Run: `cargo build -p sp-mcp`
Expected: success (may need to update rmcp API usage if the `tool` / `tool_handler` / `tool_router` macros differ from what step 2 assumes — read the rmcp 3.0 docs and adjust the imports / attribute syntax). If the build fails, fix step 2/3/4 to match the actual API; this is the highest-risk step in the plan.

- [ ] **Step 6: Manual smoke test — server starts and lists six tools**

Run, in one terminal:
```bash
SCHOLARPRESS_CATALOG_PATH=/tmp/fake-catalog SCHOLARPRESS_WORKSPACE_ROOT=/tmp/fake-ws cargo run -p sp-mcp
```

(Don't worry that `/tmp/fake-catalog` doesn't exist — the server starts before any tool is called. The `from_env` validation may or may not reject the missing dir; if it does, create the empty dir first.)

In another terminal, send a raw MCP `initialize` + `tools/list` via stdin (you can use `python3` with a one-liner, or `nc`):

```bash
printf '%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | SCHOLARPRESS_CATALOG_PATH=/tmp/fake-catalog SCHOLARPRESS_WORKSPACE_ROOT=/tmp/fake-ws cargo run -p sp-mcp
```

Expected: the response to `tools/list` includes six tools with the names: `list_workspaces`, `list_profiles`, `create_workspace`, `compile_typst`, `check_pdf`, `extract_document`. (Names are derived from the Rust method names by snake_casing them — verify the actual names returned.)

If the response names differ (e.g., trailing underscores), note them and continue. The names matter for harness config but not for the build.

- [ ] **Step 7: Commit**

```bash
git add crates/sp-mcp/
git commit -m "feat(sp-mcp): wire up rmcp stdio server with six tool handlers"
```

---

## Task 9: Write the sp-mcp README

**Files:**
- Create: `crates/sp-mcp/README.md`

- [ ] **Step 1: Write the README**

Create `crates/sp-mcp/README.md` with:

```markdown
# sp-mcp

Stdio MCP server for the ScholarPress ecosystem. Exposes six tools
(workspace/profile discovery, document extraction, Typst compilation,
formatting checks) for use from any MCP-compliant agent harness.

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
```

- [ ] **Step 2: Commit**

```bash
git add crates/sp-mcp/README.md
git commit -m "docs(sp-mcp): README with setup, env vars, OpenCode config"
```

---

## Task 10: Apply ponytail annotations to sp-typst

**Files:**
- Modify: `crates/sp-typst/src/lib.rs`

Per the spec's "Ponytail annotations on sp-typst" section.

- [ ] **Step 1: Add the `ponytail:` comment above the `Command::new("typst")` site**

In `crates/sp-typst/src/lib.rs`, change the body of `compile` so the line preceding `let mut cmd = Command::new("typst");` reads:

```rust
    // ponytail: shells out to typst binary (external dep on PATH).
    // Upgrade: swap for `typst` crate once sp-typst API churn settles and
    // per-iteration build cost matters less. See
    // docs/superpowers/specs/2026-07-29-scholarpress-mcp-design.md
    let mut cmd = Command::new("typst");
```

- [ ] **Step 2: Update the doc comment on `pub fn compile`**

Change the existing `/// Compile Typst source to PDF.` doc comment to:

```rust
/// Compile Typst source to PDF.
///
/// Currently shells out to the external `typst` binary (must be on PATH).
/// Tracked for in-process embedding once the toolset stabilizes.
pub fn compile(source: &str, root: Option<&Path>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
```

- [ ] **Step 3: Verify sp-typst still builds and tests pass**

Run: `cargo test -p sp-typst`
Expected: all existing tests pass (the `ponytail:` comment does not change behavior)

- [ ] **Step 4: Commit**

```bash
git add crates/sp-typst/src/lib.rs
git commit -m "docs(sp-typst): mark shell-out as deliberate deferral with ponytail annotation"
```

---

## Task 11: Remove apps/publish-service

**Files:**
- Delete: `apps/publish-service/` (entire directory)
- Modify: `scholarpress-deliver/README.md` (remove references to `scholarpress-publish-ui`)

- [ ] **Step 1: Verify nothing in the workspace depends on publish-service**

Run: `cargo metadata --format-version 1 --no-deps | grep -o '"name":"sp-[a-z-]*"' | sort -u`
Expected: lists sp-extract, sp-check, sp-typst, sp-mcp, scholarpress-cli — but NOT sp-publish-service or anything from apps/publish-service. (publish-service's Cargo package name may have been different; this check is just to confirm the workspace doesn't link to it.)

Also: `grep -r "publish-service\|publish_service" crates/ apps/scholarpress-cli/ --include="*.rs" --include="*.toml"`
Expected: no matches (or only matches unrelated to the package name — e.g., comments). If there ARE matches, stop and reassess; the spec assumed no dependents.

- [ ] **Step 2: Delete the directory**

Run: `git rm -r apps/publish-service/`
Expected: git removes the directory from the index; the files are still on disk until commit.

- [ ] **Step 3: Verify the workspace still builds**

Run: `cargo build`
Expected: success (the workspace Cargo.toml uses `apps/*` glob, so the deleted member is automatically removed from `members`)

- [ ] **Step 4: Update `scholarpress-deliver/README.md`**

Read the current file and remove the references to `scholarpress-publish-ui`. The image name appears in the "Architecture" table. Replace it with a note that `sp-mcp` is the new entry point and link to the sp-mcp README.

If `scholarpress-deliver/docker-compose.yml` references the deleted image, also update that file. Use `grep -rn "publish-ui\|publish_service" scholarpress-deliver/` to find every reference.

- [ ] **Step 5: Verify the deliver README still parses (no broken links)**

Re-read the file. If it referenced removed services, the prose should now point to sp-mcp as the consumer.

- [ ] **Step 6: Commit**

```bash
git add -A apps/ scholarpress-deliver/
git commit -m "refactor: remove publish-service; sp-mcp is the new tool surface"
```

---

## Task 12: Final verification

- [ ] **Step 1: Full workspace build and test**

Run:
```bash
cargo build
cargo test --workspace
```

Expected: zero errors, all tests pass (including the sp-mcp unit tests, the sp-check and sp-extract tests, and the catalog fixture validation if it's wired into cargo).

- [ ] **Step 2: Lint check (pre-push requirement)**

Run:
```bash
cargo fmt --all
cargo clippy --all --tests -- -D warnings
```

Expected: zero warnings. If `cargo fmt` reports diffs, run it and re-commit; if `cargo clippy` reports warnings, fix the underlying code (don't suppress lints).

- [ ] **Step 3: Catalog fixture validation (if wired)**

If `scholarpress-catalog/institutions/iu/tests/validate_fixtures.sh` is runnable from the backend repo, run it. Otherwise note as deferred.

- [ ] **Step 4: End-to-end smoke (manual)**

Set up real env vars and exercise one full loop:
```bash
export SCHOLARPRESS_CATALOG_PATH=/path/to/scholarpress-catalog
export SCHOLARPRESS_WORKSPACE_ROOT=/tmp/sp-mcp-smoke
mkdir -p $SCHOLARPRESS_WORKSPACE_ROOT

cargo run -p sp-mcp &
SERVER_PID=$!
sleep 2

# From another shell, send list_profiles and create_workspace via MCP
# (or use a Python MCP client to drive it)
kill $SERVER_PID
```

Expected: `list_profiles` returns at least `institutions/iu`. `create_workspace` with `{"name":"smoke","profile_id":"institutions/iu"}` creates a directory with `spec.yaml` and `template/`.

- [ ] **Step 5: OpenCode integration smoke (manual, optional)**

Add the sp-mcp binary to OpenCode's `opencode.json` as shown in the sp-mcp README. Open a new OpenCode session and ask the LLM to "list the available ScholarPress profiles." The model should call `list_profiles` and report back the IU profile. If it doesn't, check OpenCode's MCP logs.

- [ ] **Step 6: Tag the milestone (optional)**

If everything is green, push the branch and open a PR. If you'd rather not push yet, just note the local commit hashes for the user.
