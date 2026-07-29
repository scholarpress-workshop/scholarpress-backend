# ScholarPress MCP Server: Backend Tool Surface for Agent Harnesses

## Context

The `scholarpress-publish` bespoke app (Next.js chat + Rust doc service) was
the first attempt at an AI-driven dissertation formatter. It did not work 
well. Also, the agentic ecosystem is moving fast: new harnesses
(OpenCode, Claude Code, Cursor, custom) appear every quarter with 
rapidly expanding capabilities.

This design pivots the product to **catalog + MCP server**: keep the
high-quality institution profiles and Typst templates in
`scholarpress-catalog`; expose the backend's capabilities (extract, check,
compile) as a small set of stdio MCP tools that any compliant harness can
drive. The harness handles file I/O, edit/granular operations, and chat
UX; the MCP server handles the operations no harness can do natively and
mediates access to the catalog and per-job workspaces.

## Design

### Architecture: one new crate, one process

Add `crates/sp-mcp` to `scholarpress-backend/`. Stdio MCP transport via the
`rmcp` crate. Direct library calls into `sp-extract`, `sp-check`, `sp-typst`
— no HTTP layer, no CLI subprocess for normal operation (except `typst`
itself, see [Known limitations](#known-limitations)).

```
scholarpress-backend/
  crates/
    sp-extract/    (unchanged)
    sp-check/      (unchanged)
    sp-typst/      (unchanged — see "Ponytail annotations" below)
    sp-mcp/        NEW — rmcp-based stdio server
  apps/
    scholarpress-cli/   (unchanged — still used by catalog fixture scripts)
    publish-service/    (REMOVED — no dependents remain)
```

`publish-service` is removed. Its only consumer was the deprecated
`publish-ui`; `validate_fixtures.sh` uses `scholarpress-cli`, not the HTTP
service. Nothing in the current ecosystem depends on the HTTP routes.

### Tool surface (v1, six tools)

| Tool | Purpose | Notes |
|------|---------|-------|
| `list_workspaces()` | Enumerate existing workspace dirs (returns absolute paths + profile id + mtime) | Agent needs the configured root — harness doesn't know it |
| `list_profiles()` | List profiles in the catalog (id + name + institution/scope) | Agent needs to discover what's available |
| `create_workspace(name, profile_id)` | Copy a profile from catalog into a new workspace dir, return its path | MCP owns the catalog source + workspace root |
| `compile_typst(workspace, entry_path, data_json?, out_name?)` | Run `typst compile` against workspace files, write to `<workspace>/out/<out_name>.pdf` (default = `entry_path` stem), return the absolute output path | Optional JSON data for `#let data = json.decode(...)` pattern |
| `check_pdf(workspace, pdf_path)` | Run formatting checks against the workspace's `spec.yaml`, return violations array | Always uses the workspace spec — the workspace IS the profile after creation |
| `extract_document(file_path)` | Parse PDF/DOCX, return structured `ParsedDocument` JSON | Reuses `sp-extract` types |

**Explicitly NOT in MCP** (agent uses harness tools instead):

- `read_file` / `write_file` / `patch_file` — read-only is safe; granular
  edits are what the harness's Edit tool exists for. Exposing them in MCP
  would duplicate that tool, force us to maintain context-window protection
  logic, and create two paths the agent has to choose between.
- `list_files` — agent uses harness `ls`/`glob` on the absolute paths
  `list_workspaces()` returns.
- `delete_workspace` / `delete_file` — agent uses harness `rm -rf`.
- `mkdir`, `mv`, `cp` — agent uses harness equivalents.

### Workspace model

Under a configurable root (default `~/.scholarpress/workspaces/`, env
`SCHOLARPRESS_WORKSPACE_ROOT`):

```
workspaces/<name>/
  spec.yaml          # forked from catalog, agent-editable
  template/          # forked from catalog, agent-editable
    template.typ
    styles.typ
    sections/...
  data/              # agent puts source PDFs, intermediate JSON here
  out/               # compiled PDFs go here
```

- `create_workspace` copies `spec.yaml` and `template/` from the catalog
  into a fresh dir. Excludes `tests/corpus/` (large calibration PDFs, not
  part of the template).
- Workspaces are **snapshots**, not live links. Catalog updates do not
  auto-resync. Want a new profile version? Create a new workspace.
- No `delete_workspace` tool. Agent uses harness `rm -rf`.
- No locking, no concurrency control. One agent per workspace; parallel
  jobs = two workspaces.

### Catalog integration

- Root: `SCHOLARPRESS_CATALOG_PATH` (default `../scholarpress-catalog`,
  existing sibling convention). v1 supports only a local directory
  path; remote catalog sources (e.g. a GitHub URL cloned on demand, a
  registry fetch) are a deliberate deferral — see Non-goals.
- `list_profiles()` scans every `<top>/<id>/` that has both `spec.yaml`
  and `template/template.typ`. Returns the id, display name, and scope
  (institution / journal / server / grant).
- The catalog is read-only **by construction**: MCP is the only code path
  that reads from `CATALOG_PATH`, and `create_workspace` copies out, never
  writes back. The agent's harness-level write tools can scribble all
  over the workspace, but cannot reach the catalog — they only have the
  absolute paths MCP returned.

### Distribution and setup

`sp-mcp` is a binary in the existing backend monorepo.

Build: `cargo build --release --bin sp-mcp`

OpenCode config:

```json
{
  "mcp": {
    "scholarpress": {
      "command": "/path/to/sp-mcp",
      "env": {
        "SCHOLARPRESS_CATALOG_PATH": "/path/to/scholarpress-catalog",
        "SCHOLARPRESS_WORKSPACE_ROOT": "/home/me/.scholarpress/workspaces"
      }
    }
  }
}
```

Stdio MCP transport. Works in any stdio-MCP-compliant harness (OpenCode,
Claude Code, Cursor, custom) — the MCP spec is the API contract, no
per-harness code.

External dep: the `typst` binary on PATH (documented in `sp-mcp` README;
user installs via `cargo install typst-cli` or download from typst.app).
Bundling typst is deliberately deferred — see "Ponytail annotations"
below.

## File-level changes

| Path | Change |
|------|--------|
| `crates/sp-mcp/Cargo.toml` | NEW. Deps: `rmcp`, `tokio`, `sp-extract`, `sp-check`, `sp-typst`, `serde`, `serde_json`, `anyhow` |
| `crates/sp-mcp/src/main.rs` | NEW. Stdio MCP server bootstrap, config loading (env vars) |
| `crates/sp-mcp/src/workspace.rs` | NEW. `list_workspaces`, `list_profiles`, `create_workspace` |
| `crates/sp-mcp/src/tools.rs` | NEW. MCP tool trait impls, request/response schemas |
| `crates/sp-mcp/src/error.rs` | NEW. Tool error type → MCP error responses |
| `crates/sp-mcp/README.md` | NEW. Setup, env vars, OpenCode config snippet, typst install step |
| `crates/sp-mcp/tests/integration.rs` | NEW. One runnable check per tool against fixture data |
| `Cargo.toml` (workspace root) | Add `crates/sp-mcp` to `members` |
| `apps/publish-service/` | DELETE entire directory |
| `Cargo.toml` (workspace root) | Remove `apps/publish-service` from `members` |
| `crates/sp-typst/src/lib.rs` | Add `ponytail:` comment at the `Command::new("typst")` site (see below) |
| `crates/sp-typst/src/lib.rs` | Add doc comment on `pub fn compile` referencing the same upgrade path |
| `scholarpress-publish/` | Repository becomes read-only / archived (separate housekeeping task, not in this design) |
| `scholarpress-deliver/` | Update README: remove references to `scholarpress-publish-ui` Docker image |

## Ponytail annotations on sp-typst (apply at implementation)

Two `ponytail:` markers to capture the deliberate deferral of embedding
the typst crate:

**1. At the shell-out site** in `crates/sp-typst/src/lib.rs`, adjacent to
`Command::new("typst")`:

```rust
// ponytail: shells out to typst binary (external dep on PATH).
// Upgrade: swap for `typst` crate once sp-typst API churn settles and
// per-iteration build cost matters less. See
// docs/superpowers/specs/2026-07-29-scholarpress-mcp-design.md
```

**2. Doc comment on the public `compile()` function**:

```rust
/// Compile Typst source to PDF.
///
/// Currently shells out to the external `typst` binary (must be on PATH).
/// Tracked for in-process embedding once the toolset stabilizes.
pub fn compile(source: &str) -> Result<Vec<u8>, TypstError> { ... }
```

The `ponytail:` marker means anyone hitting the shell-out code later
sees (a) this was deliberate, (b) the upgrade path, (c) where the
design rationale lives. Not a `TODO` (noise), not a `FIXME` (bug-shaped),
not a `NOTE` (too vague).

## Error handling

Every tool returns structured errors with enough context for the LLM to
act on:

| Tool | Failure mode | What the LLM gets |
|------|--------------|-------------------|
| `compile_typst` | Typst error (syntax, missing import) | `error: unclosed delimiter` + file/line/column from typst's native diagnostics |
| `check_pdf` | PDF has violations | **Success response**, body is the violations array (NOT an error — finding violations is the tool's job) |
| `check_pdf` | Spec file missing or invalid | "spec.yaml not found at `<path>`" or YAML parse error |
| `extract_document` | PDF/DOCX unparseable | Parser error with file path + reason |
| `create_workspace` | Profile id not in catalog | List of valid profile ids in the error message |
| `list_*` | Catalog/workspace root unset or unreadable | "`SCHOLARPRESS_CATALOG_PATH` is not set or unreadable: `<path>`" |

No silent fallbacks. No swallowed errors. The LLM needs to know what
went wrong to fix it. No retries inside the tool — the LLM decides
whether to retry (with edits) or give up.

## Non-goals (ponytail: deliberate deferrals)

The following are **consciously deferred**, not forgotten. Each is
parked here with its trigger condition for revisit.

- **Multi-harness documentation matrix** — works in any stdio-MCP client
  by spec, but tested in OpenCode only. Revisit when a second harness
  becomes a real target.
- **Cross-platform packaging** — Linux first. Revisit if Mac/Win needed.
- **Auth, multi-user, hosted MCP server** — local personal tool. Revisit
  if the user base grows beyond one.
- **Profile authoring tools** (LLM-assisted spec generation from real
  PDFs). Revisit when the catalog expands beyond hand-written profiles.
- **Catalog schema changes** — input is the current catalog as-is.
  Revisit when a profile needs to express something the current schema
  can't.
- **Removing `publish-service` is in this design**; **archiving the
  `scholarpress-publish` repo** is a separate housekeeping task. Land
  the sp-mcp crate working, then archive the publish repo.
- **Embedding the `typst` Rust crate** (drops the external `typst`
  binary dep; ~25-35 MB binary size increase; 3-5 min cold build).
  Revisit when sp-typst's public API has been stable across 2+ typst
  releases with no internal call sites breaking. Tracked inline via
  `ponytail:` comments in `sp-typst/src/lib.rs`.
- **Streaming progress for long operations** — compile/check are fast.
  Revisit if they become slow.
- **The 2 failing IU checks** (31/33 currently pass) — not investigated
  in this design. Revisit if the LLM workflows in sp-mcp surface them
  as a practical problem.
- **Remote catalog sources** (GitHub URL cloned on demand, registry
  fetch, etc.) — v1 supports only a local `SCHOLARPRESS_CATALOG_PATH`
  sibling directory. Revisit when distribution beyond the local user
  becomes real (multi-user, shared team catalog, or profile publishing
  workflow).

## Known limitations

**sp-mcp still depends on the `typst` binary on PATH.** This is a
conscious choice (see Ponytail annotations). The README documents the
install step. Forgetting the install produces a clear "binary not
found" error from the `compile_typst` tool — the LLM can surface this
to the user.

**Workspaces are unversioned.** A workspace is just a directory; the
agent can scribble freely. To roll back, the user `rm -rf`s and creates
a new one from the same profile. If this becomes painful, a future
iteration could back workspaces with git worktrees (deferred).

**Catalog is read-only by convention, not by enforcement at the
filesystem level.** MCP never writes to `CATALOG_PATH`, but a sufficiently
determined user could `chmod` the catalog and the agent would inherit
the permissions. This is acceptable for a personal tool; revisit if
multi-user is ever in scope.

## Breaking changes

- `apps/publish-service` is deleted. Any external code calling
  `http://localhost:4000/extract`, `/check`, `/compile`,
  `/institutions`, etc., will break. No backward-compatible aliases.
  Currently zero known external callers.
- `scholarpress-deliver`'s `docker-compose.yml` references the deleted
  `scholarpress-publish-ui` image. Update documented in file-level
  changes; users running `docker-compose up` will need to switch to
  configuring sp-mcp in their harness of choice.

## Verification

Per the "non-trivial logic leaves one runnable check behind" rule, one
minimal check per tool. No harness simulator, no JSON-RPC round-trip
test framework, no per-input fuzzing. The catalog's own
`validate_fixtures.sh` is the integration test.

- `compile_typst`: compile a 3-line `= Hello` typst file in a temp dir,
  assert non-empty PDF bytes
- `check_pdf`: run against `baseline.pdf` (known-good IU fixture), assert
  0 violations; against `left-narrow.pdf` (known-bad), assert
  `global_margins` in the violation list
- `extract_document`: run against a small known PDF, assert metadata
  present
- `create_workspace`: create from a fixture profile, assert files
  copied, assert `template/` and `spec.yaml` present
- `list_profiles` / `list_workspaces`: assert non-empty array against a
  fixture catalog

Required:

- `cargo build && cargo test` succeed workspace-wide
- All existing catalog fixture validation passes (`bash
  institutions/iu/tests/validate_fixtures.sh`)
- New `sp-mcp` integration tests pass (one per tool)
- End-to-end smoke: create workspace from `iu` profile, run
  `compile_typst` + `check_pdf` against the existing `template.typ`,
  observe zero violations
- Manual: configure sp-mcp in OpenCode, complete a real dissertation
  formatting session using only the six MCP tools + harness file tools
