# Design: `scholarpress_interface_doc` Tool

Associates with: https://github.com/scholarpress-workshop/scholarpress-backend/issues/7
Date: 2026-08-01

## Problem

Agents waste 10+ tool calls reverse-engineering template function signatures and
entry data shapes because `template.typ` header comments only cover high-level
conventions, not per-function details. Three different entry data shapes exist
across list-like functions (dicts, positional tuples, raw content) with no
documentation of which function expects which shape.

## Solution

Add doc-comments (tidy format) to every section function in the catalog
template, generate a pre-rendered `REFERENCE.json` via CI, and expose it
through a new MCP tool that reads the file — zero compile cost at invocation.

### Architecture

```
scholarpress-catalog/institutions/<profile>/template/sections/*.typ
  author adds /// doc-comments here
        │
        ▼  CI: tidy.parse-module() → JSON
scholarpress-catalog/institutions/<profile>/template/REFERENCE.json
  pre-rendered artifact, committed to catalog
        │
        ▼  create_workspace() copies template/ tree
workspace/template/REFERENCE.json
        │
        ▼  scholarpress_interface_doc(workspace) tool reads and returns it
```

### Doc-comment convention

Each section function gets tidy `///` comments:

```typ
/// Renders the table of contents from a list of chapter entries.
/// Entries must be dicts with keys: `level`, `title`, `page`.
///
/// @example
/// ```typ
/// #toc-page(entries: ((level: 1, title: "Introduction", page: 1)))
/// ```
/// @endexample
///
/// -> none
#let toc-page(entries: (), cv-page: none) = { ... }
```

The comments must cover:
- What the function produces (description)
- Parameter names, types, and defaults
- Entry data shape when relevant (dict, tuple, raw content)
- A minimal calling example

Non-function globals (page numbering conventions, `data.json` usage) go in a
`REFERENCE.json` top-level `globals` section derived from `template.typ` comments.

### JSON output shape

```json
{
  "profile": "institutions/iu",
  "globals": [
    {"key": "page_numbering", "value": "i (front), 1 (body, from chapter())",
     "description": "Front matter uses Roman numerals; body chapters switch to Arabic via chapter()"},
    {"key": "data_json", "value": "data.json",
     "description": "Write structured data to data.json; templates read with json(\"data.json\")"}
  ],
  "functions": [
    {
      "name": "toc-page",
      "file": "sections/toc.typ",
      "signature": "toc-page(entries: (), cv-page: none)",
      "description": "Renders the table of contents.",
      "params": [
        {"name": "entries", "type": "array", "default": "()",
         "description": "Chapter entries as dicts with keys: level (int), title (str), page (int)"},
        {"name": "cv-page", "type": "int | none", "default": "none",
         "description": "Page number of CV entry. When set, suppresses leader dots for that entry."}
      ],
      "data_shape": "array of dicts: {level: int, title: str, page: int}",
      "example": "toc-page(entries: ((level: 1, title: \"Introduction\", page: 1)))"
    }
  ]
}
```

### CI generation

A script in the catalog repo runs on push to any `institutions/*/template/`
directory. For each profile:

```bash
typst compile generate-json-ref.typ template/REFERENCE.json
```

Where `generate-json-ref.typ`:
- Imports all section files
- Calls `tidy.parse-module()` to extract doc-comments and signatures
- Serializes to the JSON schema above

CI fails if:
- Any section function has no doc-comment
- A documented parameter is missing from the actual signature
- A documented parameter does not exist in the actual signature

### MCP tool

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct InterfaceDocParams {
    pub workspace: PathBuf,
}

#[tool(description = "Returns a structured reference for every section function in the workspace template: signatures, parameter types, entry data shapes, and calling examples. Reads a pre-rendered REFERENCE.json — no compilation needed.")]
async fn interface_doc(&self, params: Parameters<InterfaceDocParams>) -> Result<CallToolResult, McpError> {
    let path = params.0.workspace.join("template").join("REFERENCE.json");
    let text = std::fs::read_to_string(&path).map_err(|e| SpMcpError::Compilation(format!(
        "REFERENCE.json not found at {}: {}. Run CI in the catalog repo to generate it.",
        path.display(), e
    )))?;
    let json = serde_json::from_str::<serde_json::Value>(&text).map_err(|e| ...)?;
    // Return pretty-printed JSON as text so the agent can read it directly
    Ok(CallToolResult::success(vec![ContentBlock::text(serde_json::to_string_pretty(&json).unwrap())]))
}
```

### Workflow integration

The server instructions already include a WORKFLOW section. Add:

> After `create_workspace`, call `interface_doc` to see every section function's
> signature, parameter types, data shapes, and examples before writing content.

### Testing

**Catalog CI tests:**
- Tidy doc-tests (`#test(...)`) in doc-comments verify examples at generation time
- Generation script must produce valid JSON output

**sp-mcp integration test:**
- `create_workspace` for IU profile → verify `REFERENCE.json` exists and is valid JSON
- `interface_doc` call returns non-empty output with expected function names (title-page, toc-page, chapter, etc.)

### Non-goals

- Live Typst compile in the MCP tool — the pre-rendered file covers this
- Auto-detection of missing comments at tool invocation — CI catches that
- Replacing `template.typ` header comments — those remain for human readers of the entry file
