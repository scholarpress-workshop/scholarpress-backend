# sp-mcp Round 2: Pandoc Pipeline + Prompting Strategy

## Motivation

Testing feedback from 2026-07-30 surfaced three inefficiencies in the DOCX-to-Typst
formatting workflow:

1. `extract_document` returned massive JSON blobs that got truncated, forcing
   subagent re-reading and intermediate files.
2. Custom Python scripts for chapter parsing were fragile — broke on `$` and `*`
   escaping, needed post-hoc patching.
3. Slow compile-fix loops — each typo cost a full MCP roundtrip. Agent had no
   incremental workflow guidance.

Pandoc 3.7.0.2 is already on the system PATH and supports `docx` → `typst` and
`docx` → `ast` (JSON) natively. Adding a light prompting algorithm eliminates the
fragile parser and gives the agent a clear, incremental workflow.

## What Changes

### 1. MCP Tool Surface

**Remove:**
- `extract_document` — the problematic blob tool. `sp-extract` remains as a
  private dependency of `sp-check` for PDF validation but is no longer exposed
  to the agent.

**Add:**
- `pandoc_convert(file_path, format)` — shells out to `pandoc <file> --from docx --to <format>`:
  - `format: "typst"` → raw Typst string. Pandoc handles tables, basic formatting,
    smallcaps, italics, etc. natively. Eliminates the custom Python chapter parser.
  - `format: "ast"` → pandoc's native JSON AST tree. Structured tree of blocks,
    lighter than the old ParsedDocument blob. Tool description warns: **AST headings
    are unreliable. Many DOCX files use direct formatting instead of heading styles.
    Prefer the TOC text for section boundaries.**

### 2. Prompting Strategy: Map–Scaffold–Migrate–Verify

Updated `ServerInfo` instructions in `tools.rs:get_info()`:

```
Map   — Extract TOC text from the raw pandoc typst output to identify section
        count, order, and boundaries before processing content. If no TOC exists,
        use AST block structure as fallback. Goal: avoid context-window burnout
        from reading the entire document upfront.

Scaffold — Create a Typst entry file with sections wired using the template's
           named-parameter calling convention (see template.typ comments).

Migrate — For each section listed in the TOC:
          1. Keyword-match the section title in pandoc's raw Typst output to find
             the start boundary.
          2. Keyword-match the next section title to find the end boundary.
          3. Slice the content chunk between boundaries.
          4. Copy into the corresponding template section function.
          5. (Optional) Use check_typst/format_typst to validate formatting.
          6. compile_typst → fix errors → repeat for next section.
          Work one section at a time. Don't build a monolithic file.

Verify — compile_typst + check_pdf per milestone. Iterate incrementally.
```

### 3. Template/Catalog Comments

**`template.typ`** (IU catalog): Add import + call example below the existing
   CALLING CONVENTION header:

```typ
//   Example: agent wired a dissertation entry file as
//     #import "template/template.typ": title-page, dedication, toc, abstract,
//       acknowledgements, preface, chapter, references, cv
//
//     #title-page(title: "My Dissertation", author: "Jane Doe", ...)
//     #dedication(body: [dedication text])
//     #toc-page(entries: toc_data)
//     ...
//
//   Note: $ is Typst math mode. Dollar amounts, grant numbers, and similar
//         prose content must use \$ to escape.
```

**`sections/toc.typ`** (IU catalog): Document the expected entries struct:

```typ
//   TOC entries format: array of dictionaries with these fields:
//     .level — integer heading level (1 = chapter, 2 = section, etc.)
//     .title — string heading text
//     .page  — integer page number
//   Example: ((level: 1, title: "Introduction", page: 1), ...)
```

## What Stays Unchanged

- `sp-extract` crate — still built, still produces `ParsedDocument` for `sp-check`
- `sp-check` — all PDF validators (content, structure, sections, footnotes, etc.)
  consume `ParsedDocument` from `sp-extract` internally, unaffected
- `compile_typst`, `check_pdf`, `check_typst`, `format_typst` — no changes
- `list_profiles`, `list_workspaces`, `create_workspace` — no changes
- PDF extraction path — `extract_pdf` inside sp-extract is unchanged

## Implementation Notes

- Pandoc is discovered on PATH at runtime (same pattern as `typst` and `typstyle`
  in the existing `sp-typst` and `sp-mcp` code). No compile-time dependency.
- The `pandoc_convert` tool only supports `.docx` input initially. Add `.odt`,
  `.rtf`, and `.tex` if needed later.
- Pandoc AST output is the native JSON format from `pandoc --to json`, not a
  custom schema. The agent gets raw pandoc JSON — no serialization layer needed.
- Template comment changes go in `scholarpress-catalog/`, not in the MCP codebase.
  They take effect on the next `create_workspace` call.

## Testing

- Manual test: run `pandoc_convert` on `~/TRUNC - Hall dissertation 2026.docx`
  in both typst and ast modes, verify output is valid
- Confirm the updated instructions appear in MCP server info response
- Confirm `extract_document` is no longer listed in MCP tools
- Sp-check tests should continue to pass (sp-extract unchanged)
