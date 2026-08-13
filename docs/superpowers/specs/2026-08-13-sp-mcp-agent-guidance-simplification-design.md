# `sp-mcp` Agent Guidance Simplification Design

## Goal

Reduce agent confusion in `sp-mcp` by making the existing tool contracts accurate, concise, and aligned with the consolidated IU catalog template.

This is an agent-facing documentation and diagnostics cleanup. It does not add a new document-generation API.

## Non-Goals

- Do not add JSON metadata transport to `compile_typst`.
- Do not add combined or stateful tools such as `compile_and_check` or `scaffold_entry`.
- Do not rename existing MCP tools or public parameters.
- Do not replace Typst body content with JSON.
- Do not change workspace path-boundary behavior.
- Do not change PDF checking behavior or the `CheckOutcome` output shape.

## Tool Contract Changes

Keep the existing tool names and parameter structures:

- `list_workspaces`
- `list_profiles`
- `create_workspace`
- `compile_typst`
- `check_pdf`
- `pandoc_convert`
- `interface_doc`

Add concise schema descriptions for the existing path and format fields:

- `workspace`: absolute workspace path returned by `create_workspace`.
- `entry_path`: entry file path relative to the workspace, normally `entry.typ`.
- `out_name`: output filename only; the server writes it below `out/`.
- `pdf_path`: PDF path relative to the workspace, normally `out/entry.pdf`.
- `file_path`: DOCX path relative to the workspace.
- `format`: exactly `typst` or `ast`.

Remove the unsupported `data.json` claim from the `compile_typst` description. The current tool does not accept a data object or write `data.json`, so the description must not suggest that workflow.

## Server Instructions

Replace the large duplicated workflow string in `get_info` with a short sequence:

1. Call `list_profiles`.
2. Call `create_workspace`.
3. Call `interface_doc`.
4. Call `pandoc_convert` for the DOCX as `ast` and `typst` when needed.
5. Write the entry and chapter files through the workspace file tools.
6. Call `compile_typst`.
7. Call `check_pdf` and use `check_ids` to isolate failures.

Retain only the essential conversion guidance: Pandoc output is best effort, TOC text is more reliable than AST headings, and Typst content may require cleanup.

## Diagnostic Hint Changes

Update `workspace.rs::source_hints` to match the current template layout:

- Front matter checks point to `template/sections/front-matter.typ`.
- Chapter heading, figure, equation, and body checks point to `template/sections/chapter.typ` and `chapters/`.
- References, appendices, and CV checks point to `template/sections/back-matter.typ`.
- Shared typography checks point to `template/styles.typ` and `template/template.typ`.
- Entry wiring and ordering checks point to `entry.typ`.

Hints remain advisory. Preserve the current `Vec<String>` output and do not attempt source-map resolution in this change.

## Workspace Cleanup

Remove creation of the unused `<workspace>/data/` directory from `create_workspace` if repository-wide search confirms no active backend caller depends on it. The existing `out/` directory remains required.

If any active caller depends on `data/`, retain it and document that decision in the implementation plan instead of removing it.

## Verification

- Run the `sp-mcp` unit and integration tests.
- Verify generated MCP schemas/descriptions contain the corrected path and format guidance.
- Verify `compile_typst` no longer advertises `data.json`.
- Verify source hints contain no deleted paths such as `chapters/title-page.typ`, `chapters/abstract.typ`, or `chapters/cv.typ`.
- Run existing path-boundary tests unchanged.
- Run workspace compile, PDF check, and DOCX conversion tests where dependencies are available.
- Run `cargo fmt --all` and `cargo clippy --all --tests -- -D warnings` before completion.

## Future Work

Structured metadata input remains a separate design. If added later, it should introduce an explicit validated JSON contract rather than reusing the current unsupported `data.json` documentation.
