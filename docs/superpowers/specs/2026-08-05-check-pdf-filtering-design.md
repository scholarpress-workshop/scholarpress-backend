# ScholarPress: `check_pdf` check_id filtering for failure isolation

## Context

Feedback from an end-to-end MCP test (2026-08-04) identified that when
`check_pdf` reports failures, the agent has no way to determine which source
file caused the violation. The agent iterated blindly — editing
`template.typ`, `styles.typ`, and `spec.yaml` — when the real fix was in
`entry.typ`.

Approach A (already shipped in `c86d55d`) added heuristic `source_hints` to
each check outcome, suggesting likely culprit files. This design (Approach C)
adds the mechanism to verify those hints: re-run a specific check against an
isolated section's PDF to prove whether the fault is in the section itself or
in how `entry.typ` composes it.

## Design

### Tool change

`check_pdf` gains an optional `check_ids` parameter:

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `workspace` | path | yes | Workspace root directory |
| `pdf_path` | path | yes | Path to PDF file |
| `check_ids` | `string[]` | no | If provided, only run these check IDs. If absent or empty, runs all checks (current behavior). |

### Internal changes

`CheckOptions` in `sp-check/src/engine.rs` already supports filtering to a
single `check_id: Option<String>`. Replace with `check_ids:
Option<Vec<String>>`. `run_checks` filters by set membership:

```rust
if let Some(ref ids) = options.check_ids {
    if !ids.is_empty() && !ids.contains(&check_def.id) {
        continue;
    }
}
```

Backward compatible: `None` / empty list means "run all checks" (identical to current behavior).

### Agent workflow addition

The `get_info()` instructions gain a "Debug" subsection under "Verify":

```
Debug — When check_pdf reports failures, use check_ids to isolate:
  source_hints on each violation suggest likely culprit files. To
  confirm, create a minimal .typ file in the workspace that imports
  only the suspect section function from template.typ, sets
  #set page(...) matching the spec, and calls the section. Then:
  compile_typst(isolate.typ) → check_pdf(isolate.pdf, check_ids: ["the_failing_check_id"])
  If the check still fails → issue is in the template section or styles.
  If it passes → issue is in how entry.typ wires or composes the section.
```

### File-level changes

| File | Change |
|------|--------|
| `crates/sp-check/src/engine.rs` | `CheckOptions.check_id` → `check_ids: Option<Vec<String>>`. Update `run_checks` filter and one caller (`calibration.rs`). |
| `crates/sp-mcp/src/tools.rs` | `CheckPdfParams` gains `check_ids: Option<Vec<String>>`. `get_info()` gains Debug subsection. |
| `crates/sp-mcp/src/workspace.rs` | `check_pdf` wires `check_ids` into `CheckOptions`. |

### Backward compatibility

- `check_ids` is optional. Omit it → all checks run (zero behavioral change).
- Existing `CheckOptions` callers: `calibration.rs` passes `CheckOptions::default()` and needs the field rename. All tests pass unchanged.
- Breaking changes: none.

## Non-goals

- **Category filtering.** `CheckOptions` already supports `category:
  Option<String>` but it's not exposed in the MCP tool. Add if agents request
  "re-run all layout checks" without listing individual IDs.
- **Automated section isolation.** The agent creates the minimal `.typ`
  wrapper itself — it already generates Typst files during migration.
  Automating this would require knowing each section function's argument
  types and default values, which `interface_doc` already provides to the
  agent. Not worth a new MCP tool.
- **Multi-ID compile isolation.** Compiling each suspect section separately
  and running filtered `check_pdf` against each is the agent's job, not a
  single compound tool. Keep the primitives small and composable.

## Verification

- `cargo test -p sp-check` — existing tests pass; `calibration.rs` updated.
- `cargo test -p sp-mcp` — existing tests pass (21 → unchanged).
- End-to-end: compile a full PDF, run `check_pdf(full.pdf)` → see failures,
  run `check_pdf(full.pdf, check_ids: ["global_margins"])` → see only that
  check.
- Empty `check_ids` → all checks run (same as omitting the field).
- Unknown check ID → no results (empty array), not an error.
- `cargo fmt --all && cargo clippy --all --tests -- -D warnings` clean.
