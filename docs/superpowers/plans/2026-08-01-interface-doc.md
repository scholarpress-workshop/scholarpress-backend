# Interface Doc Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add tidy doc-comments to all IU template section functions, auto-generate a `REFERENCE.json` via CI, and expose it through a new `scholarpress_interface_doc` MCP tool.

**Architecture:** Authors write `///` doc-comments in section files. A Typst script (`generate-json-ref.typ`) uses `tidy.parse-module()` to extract function signatures, parameter docs, and data shapes; serializes to JSON. CI runs this script on push and commits `REFERENCE.json`. The MCP tool reads the pre-rendered JSON — no Typst compile at invocation time.

**Tech Stack:** Typst (tidy 0.4.x), Rust (rmcp MCP framework), GitHub Actions CI

## Global Constraints

- tidy 0.4.x doc-comment syntax (`///` blocks, `/// -> type` for returns)
- JSON schema matches spec: `{profile, globals: [{key, description}], functions: [{name, file, signature, description, params: [{name, type, default, description}]}]}`
- CI must fail if any section function lacks a doc-comment
- MCP tool reads workspace/template/REFERENCE.json — no compilation at invocation

---

### Task 1: Doc-comments on template.typ (globals)

**Files:**
- Modify: `scholarpress-catalog/institutions/iu/template/template.typ`

**Interfaces:**
- Produces: `template.typ` has a top-level `///` module docstring that tidy picks up as the module description

- [ ] **Step 1: Add module-level doc-comment to template.typ**

Prepend this block before line 1 of `template.typ`:

```typst
/// IU Dissertation Template — Global Metadata and Conventions.
///
/// = Page Numbering
/// Front matter uses Roman numerals (i, ii, iii...) set at template level:
///   `#set page(numbering: "i")`
/// Chapter body switches to Arabic (1, 2, 3...) via `chapter(first: true)` which
/// calls `counter(page).update(1)` and `#set page(numbering: "1")`.
/// Back matter (references, appendices) must set their own numbering or they
/// inherit the top-level Roman numeral.
///
/// = Heading Hierarchy
/// Chapter title: rendered by `chapter()` as `=` (H1).
/// Inside chapter body write `==` (H2 — centered, underlined, numbered "1.1")
/// and `===` (H3 — left-aligned, underlined, numbered "1.1.1").
/// Front matter uses Typst defaults.
///
/// = Dollar Signs
/// `$` starts math mode in Typst prose. Escape with `\$` (e.g., `\$17 million`).
///
/// = data.json
/// Write structured data to `<workspace>/data.json` before running `compile_typst`.
/// The template reads it with `json("data.json")` or `read("data.json")`.
///
/// = Chapter Per-File Convention
/// Each chapter is one file in `chapters/`:
///   `ch01.typ`: `#let ch-name = [...]`
///   `template.typ` imports: `#import "chapters/ch01.typ": ch-name`
///   Then calls: `#chapter(number: "1", title: "Title", body: ch-name, first: true)`
```

- [ ] **Step 2: Verify doc-comment stays above the first `#let`/`#import`/`#set`**

The tidy parser expects module doc-comments to appear before the first code statement. Verify the doc-comment block is the very first thing in the file.

- [ ] **Step 3: Commit**

```bash
git add institutions/iu/template/template.typ
git commit -m "docs: add tidy module-level doc-comment to template.typ"
```

---

### Task 2: Doc-comments on list/collection section functions

**Files:**
- Modify: `scholarpress-catalog/institutions/iu/template/sections/toc.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/lot.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/lof.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/lop.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/loa.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/references.typ`

**Interfaces:**
- Consumes: None
- Produces: Each file has `///` doc-comments on its exported function

- [ ] **Step 1: Add doc-comment to toc.typ**

Replace line 1 (the `// TOC ENTRIES ...` comment block, lines 1-6) with:

```typst
/// Renders the Table of Contents.
/// Entries must be a list of dicts, each with named keys:
///   `level` (int) — heading level (1 = chapter, 2 = section)
///   `title` (str) — heading text
///   `page` (int) — page number
///
/// @example
/// ```typ
/// #toc-page(entries: (
///   (level: 1, title: "Introduction", page: 1),
///   (level: 2, title: "Background", page: 4),
/// ))
/// ```
/// @endexample
///
/// -> none
#let toc-page(
  /// Array of dicts {level: int, title: str, page: int}
  /// -> array
  entries: (),
  /// Page number for the Curriculum Vitae entry. When set, the CV entry
  /// appears with its page number (currently without leader dots).
  /// -> int | none
  cv-page: none,
) = {
```

- [ ] **Step 2: Add doc-comment to lot.typ**

Prepend before `#let list-of-tables(entries: ())`:

```typst
/// Renders the List of Tables.
/// Entries must be a list of **positional 2-tuples** `(title, page)` — NOT dicts.
///
/// @example
/// ```typ
/// #list-of-tables(entries: (
///   ("Table 1.1: Sample Results", 23),
///   ("Table 2.1: Summary Statistics", 47),
/// ))
/// ```
/// @endexample
///
/// -> none
#let list-of-tables(
  /// Array of 2-tuples: (title: str, page: int)
  /// -> array
  entries: (),
) = {
```

- [ ] **Step 3: Add doc-comment to lof.typ**

Prepend before `#let list-of-figures(entries: ())`:

```typst
/// Renders the List of Figures.
/// Entries must be a list of **positional 2-tuples** `(title, page)` — NOT dicts.
///
/// @example
/// ```typ
/// #list-of-figures(entries: (
///   ("Figure 1.1: System Architecture", 15),
/// ))
/// ```
/// @endexample
///
/// -> none
#let list-of-figures(
  /// Array of 2-tuples: (title: str, page: int)
  /// -> array
  entries: (),
) = {
```

- [ ] **Step 4: Add doc-comment to lop.typ**

Prepend before `#let list-of-pictures(entries: ())`:

```typst
/// Renders the List of Pictures.
/// Entries must be a list of **positional 2-tuples** `(title, page)` — NOT dicts.
///
/// @example
/// ```typ
/// #list-of-pictures(entries: (
///   ("Photo 1: Study Site", 30),
/// ))
/// ```
/// @endexample
///
/// -> none
#let list-of-pictures(
  /// Array of 2-tuples: (title: str, page: int)
  /// -> array
  entries: (),
) = {
```

- [ ] **Step 5: Add doc-comment to loa.typ**

Prepend before `#let list-of-abbreviations(entries: ())`:

```typst
/// Renders the List of Abbreviations.
/// Entries must be a list of **positional 2-tuples** `(abbreviation, meaning)` — NOT dicts.
///
/// @example
/// ```typ
/// #list-of-abbreviations(entries: (
///   ("API", "Application Programming Interface"),
///   ("DOI", "Digital Object Identifier"),
/// ))
/// ```
/// @endexample
///
/// -> none
#let list-of-abbreviations(
  /// Array of 2-tuples: (abbreviation: str, meaning: str)
  /// -> array
  entries: (),
) = {
```

- [ ] **Step 6: Add doc-comment to references.typ**

Prepend before `#let references-page(entries: [])`:

```typst
/// Renders the References page.
/// Entries are **raw content** — NOT tuples or dicts.
/// Pass citation text directly as a content block.
///
/// @example
/// ```typ
/// #references-page(entries: [
///   Author, A. (2020). Title of work. *Journal*, 12(3), 45--67.
///
///   Author, B. (2021). Another title. *Publisher*.
/// ])
/// ```
/// @endexample
///
/// -> none
#let references-page(
  /// Raw content block containing formatted reference text
  /// -> content
  entries: [],
) = {
```

- [ ] **Step 7: Commit all six files**

```bash
git add institutions/iu/template/sections/toc.typ \
        institutions/iu/template/sections/lot.typ \
        institutions/iu/template/sections/lof.typ \
        institutions/iu/template/sections/lop.typ \
        institutions/iu/template/sections/loa.typ \
        institutions/iu/template/sections/references.typ
git commit -m "docs: add tidy doc-comments to list/collection section functions"
```

---

### Task 3: Doc-comments on metadata section functions

**Files:**
- Modify: `scholarpress-catalog/institutions/iu/template/sections/title-page.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/acceptance.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/copyright.typ`

**Interfaces:**
- Produces: Each file has `///` doc-comments on its exported function

- [ ] **Step 1: Add doc-comment to title-page.typ**

Prepend before `#let title-page(` (line 3):

```typst
/// Renders the dissertation title page.
/// Reads `document.title` and `document.author.first()` from globals
/// when `title` or `author` are `none`. Set globals in entry.typ:
///   `#set document(title: "My Title", author: "Jane Doe")`
/// Then call `#title-page()` with zero arguments.
/// Custom metadata (school, degree, department, campus, month, year)
/// defaults to values from `styles.typ`.
///
/// @example
/// ```typ
/// #title-page(title: "A Study of X", author: "Jane Doe")
/// ```
/// @endexample
///
/// -> none
#let title-page(
  /// Dissertation title (falls back to document.title)
  /// -> str | none
  title: none,
  /// Author name (falls back to document.author.first())
  /// -> str | none
  author: none,
  /// School / college name (default: styles.typ school-name)
  /// -> str
  school: school-name,
  /// Degree name (default: styles.typ degree-name)
  /// -> str
  degree: degree-name,
  /// Department name (default: styles.typ department-name)
  /// -> str
  department: department-name,
  /// Campus name (default: styles.typ campus-name)
  /// -> str
  campus: campus-name,
  /// Graduation month (default: styles.typ grad-month)
  /// -> str
  month: grad-month,
  /// Graduation year (default: styles.typ grad-year)
  /// -> str
  year: grad-year,
) = {
```

(Keep the existing function body unchanged — replace only lines 3-12 with the above, preserving the rest of the file from `context {` onward.)

- [ ] **Step 2: Add doc-comment to acceptance.typ**

Prepend before `#let acceptance-page(` (line 3):

```typst
/// Renders the acceptance page with committee signatures.
/// Committee members and defense date default to values from `styles.typ`.
/// Override per-workspace via `data.json`.
///
/// @example
/// ```typ
/// #acceptance-page(
///   committee: ((name: "Dr. Smith", degree: "Ph.D.", role: "Chair"),),
///   defense_date: "May 2026",
/// )
/// ```
/// @endexample
///
/// -> none
#let acceptance-page(
  /// Committee members: list of dicts {name: str, degree: str, role: str}
  /// -> array
  committee: committee-members,
  /// Defense date string (e.g., "May 2026")
  /// -> str
  defense_date: defense-date,
) = {
```

- [ ] **Step 3: Add doc-comment to copyright.typ**

Prepend before `#let copyright-page(` (line 1):

```typst
/// Renders the copyright page with year and author name centered vertically.
///
/// @example
/// ```typ
/// #copyright-page(year: "2026", author: "Jane Doe")
/// ```
/// @endexample
///
/// -> none
#let copyright-page(
  /// Copyright year
  /// -> str
  year: "",
  /// Copyright holder name
  /// -> str
  author: "",
) = {
```

- [ ] **Step 4: Commit**

```bash
git add institutions/iu/template/sections/title-page.typ \
        institutions/iu/template/sections/acceptance.typ \
        institutions/iu/template/sections/copyright.typ
git commit -m "docs: add tidy doc-comments to metadata section functions"
```

---

### Task 4: Doc-comments on content section functions

**Files:**
- Modify: `scholarpress-catalog/institutions/iu/template/sections/dedication.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/acknowledgements.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/preface.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/abstract.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/chapters.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/appendices.typ`
- Modify: `scholarpress-catalog/institutions/iu/template/sections/cv.typ`

**Interfaces:**
- Produces: Each file has `///` doc-comments on its exported function(s)

- [ ] **Step 1: Add doc-comment to dedication.typ**

Prepend before `#let dedication-page(`:

```typst
/// Renders the dedication page. Takes a single content block.
///
/// @example
/// ```typ
/// #dedication-page(body: [To my family, for their unwavering support.])
/// ```
/// @endexample
///
/// -> none
#let dedication-page(
  /// Dedication text as content block
  /// -> content
  body: [],
) = {
```

- [ ] **Step 2: Add doc-comment to acknowledgements.typ**

Prepend before `#let acknowledgements-page(`:

```typst
/// Renders the acknowledgements page.
///
/// @example
/// ```typ
/// #acknowledgements-page(body: [I would like to thank...])
/// ```
/// @endexample
///
/// -> none
#let acknowledgements-page(
  /// Page heading (default: "Acknowledgements")
  /// -> str
  title: "Acknowledgements",
  /// Acknowledgements text as content block
  /// -> content
  body: [],
) = {
```

- [ ] **Step 3: Add doc-comment to preface.typ**

Prepend before `#let preface-page(`:

```typst
/// Renders the preface page.
///
/// @example
/// ```typ
/// #preface-page(title: "Author's Preface", body: [This work began...])
/// ```
/// @endexample
///
/// -> none
#let preface-page(
  /// Page heading (default: "Preface")
  /// -> str
  title: "Preface",
  /// Preface text as content block
  /// -> content
  body: [],
) = {
```

- [ ] **Step 4: Add doc-comment to abstract.typ**

Prepend before `#let abstract-page(` (line 3):

```typst
/// Renders the abstract page with title, author, body text, and committee lines.
/// Falls back to `document.title` and `document.author.first()` when title/author
/// are `none`. Set globals in entry.typ: `#set document(title: "...", author: "...")`
///
/// @example
/// ```typ
/// #abstract-page(
///   heading: "Abstract",
///   body: [This dissertation examines...],
/// )
/// ```
/// @endexample
///
/// -> none
#let abstract-page(
  /// Abstract heading text (default: "Abstract")
  /// -> str
  heading: "Abstract",
  /// Author name (falls back to document.author.first())
  /// -> str | none
  author: none,
  /// Dissertation title (falls back to document.title)
  /// -> str | none
  title: none,
  /// Abstract body text
  /// -> str
  body: "",
  /// Committee members: list of dicts {name: str, degree: str, role: str}
  /// -> array
  committee: committee-members,
) = {
```

- [ ] **Step 5: Add doc-comment to chapters.typ**

The `chapter` function already has doc-comments (lines 3-13). Enhance them for completeness:

Replace lines 3-13:

```typst
/// Renders a dissertation chapter with proper heading hierarchy.
///
/// Heading hierarchy (scoped to chapter body, front matter unaffected):
///   =  (H1) — chapter title, centered, underlined, uppercase, spelled-out number
///   == (H2) — centered, underlined, numbered "1.1", regular weight
///   === (H3) — left-aligned, underlined, numbered "1.1.1", regular weight
///
/// Use `first: true` on the first body chapter to:
///   - Reset page counter to 1 (`counter(page).update(1)`)
///   - Switch page numbering from Roman (front matter) to Arabic (body)
///
/// @example
/// ```typ
/// #chapter(number: "1", title: "Introduction", body: intro-body, first: true)
/// ```
/// @endexample
///
/// -> none
#let chapter(
  /// Chapter number as string (e.g., "1", "2"). Spelled out in heading.
  /// -> str
  number: "",
  /// Chapter title text
  /// -> str
  title: "",
  /// Chapter body as content block (write `==` and `===` headings inside)
  /// -> content
  body: [],
  /// Set true for first body chapter to reset page numbering to Arabic
  /// -> bool
  first: false,
) = {
```

- [ ] **Step 6: Add doc-comment to appendices.typ**

Replace the entire file with:

```typst
/// Renders the appendices divider page. Call once before individual appendix pages.
///
/// -> none
#let appendices-section() = {
  pagebreak()
  [
    #v(1fr)
    #align(center, text(12pt, upper("Appendices")))
    #v(2fr)
  ]
}

/// Renders an individual appendix with a letter label and title.
///
/// @example
/// ```typ
/// #appendix(label: "A", title: "Survey Instrument", body: [Survey details...])
/// ```
/// @endexample
///
/// -> none
#let appendix(
  /// Appendix letter label (e.g., "A", "B")
  /// -> str
  label: "A",
  /// Appendix title text
  /// -> str
  title: "",
  /// Appendix body as content block
  /// -> content
  body: [],
) = {
  pagebreak()
  [
    #v(1fr)
    #align(center, text(12pt, upper("APPENDIX " + label))) \
    #align(center, text(12pt, upper(title)))
    #v(2fr)
  ]
  pagebreak()
  body
}
```

- [ ] **Step 7: Add doc-comment to cv.typ**

Prepend before `#let curriculum-vitae(`:

```typst
/// Renders the Curriculum Vitae page. Sets `page(numbering: none)` to suppress
/// page numbers on CV pages.
///
///  @example
/// ```typ
/// #curriculum-vitae(body: [
///   == Education
///   Ph.D. Candidate, Indiana University, 2026
/// ])
/// ```
/// @endexample
///
/// -> none
#let curriculum-vitae(
  /// Name (falls back to document.author.first())
  /// -> str | none
  name: none,
  /// CV content as content block
  /// -> content
  body: [],
) = {
```

- [ ] **Step 8: Commit**

```bash
git add institutions/iu/template/sections/dedication.typ \
        institutions/iu/template/sections/acknowledgements.typ \
        institutions/iu/template/sections/preface.typ \
        institutions/iu/template/sections/abstract.typ \
        institutions/iu/template/sections/chapters.typ \
        institutions/iu/template/sections/appendices.typ \
        institutions/iu/template/sections/cv.typ
git commit -m "docs: add tidy doc-comments to content section functions"
```

---

### Task 5: Generate REFERENCE.json script + CI workflow

**Files:**
- Create: `scholarpress-catalog/institutions/iu/template/generate-json-ref.typ`
- Create: `scholarpress-catalog/.github/workflows/generate-reference.yml`

**Interfaces:**
- Produces: `REFERENCE.json` at `scholarpress-catalog/institutions/iu/template/REFERENCE.json`

- [ ] **Step 1: Create generate-json-ref.typ**

```typst
#import "@preview/tidy:0.4.3": parse-module

#let section-files = (
  ("title-page", "sections/title-page.typ"),
  ("acceptance-page", "sections/acceptance.typ"),
  ("copyright-page", "sections/copyright.typ"),
  ("dedication-page", "sections/dedication.typ"),
  ("acknowledgements-page", "sections/acknowledgements.typ"),
  ("preface-page", "sections/preface.typ"),
  ("abstract-page", "sections/abstract.typ"),
  ("toc-page", "sections/toc.typ"),
  ("list-of-tables", "sections/lot.typ"),
  ("list-of-figures", "sections/lof.typ"),
  ("list-of-pictures", "sections/lop.typ"),
  ("list-of-abbreviations", "sections/loa.typ"),
  ("chapter", "sections/chapters.typ"),
  ("references-page", "sections/references.typ"),
  ("appendices-section", "sections/appendices.typ"),
  ("appendix", "sections/appendices.typ"),
  ("curriculum-vitae", "sections/cv.typ"),
)

#let all-functions = ()

#for (_, path) in section-files {
  let file-content = read(path)
  let docs = parse-module(file-content)
  for func in docs.functions {
    func.file = path
    all-functions.push(func)
  }
}

#let functions = ()
#for func in all-functions {
  let params = ()
  for (name, info) in func.args.pairs() {
    let param-type = if info.types.len() > 0 {
      info.types.join(" | ")
    } else {
      "any"
    }
    let default = if "default" in info {
      info.default
    } else {
      none
    }
    params.push((
      name: name,
      type: param-type,
      default: default,
      description: info.description,
    ))
  }

  let sig = func.name + "("
  let first = true
  for (name, info) in func.args.pairs() {
    if not first { sig += ", " }
    first = false
    sig += name + ": "
    if "default" in info {
      sig += info.default
    } else {
      sig += info.types.first()
    }
  }
  sig += ")"

  functions.push((
    name: func.name,
    file: func.file,
    signature: sig,
    description: func.description,
    params: params,
  ))
}

#json.encode((
  profile: "institutions/iu",
  globals: (
    (
      key: "page_numbering",
      description: "Front matter uses Roman numerals (\"i\") set at template level. Chapter body switches to Arabic (\"1\") via `chapter(first: true)` which calls `counter(page).update(1)` and `set page(numbering: \"1\")`."
    ),
    (
      key: "data_json",
      description: "Write structured data to <workspace>/data.json before compile. Template reads via `json(\"data.json\")` or `read(\"data.json\")`."
    ),
    (
      key: "heading_hierarchy",
      description: "chapter() renders = (H1). Inside chapter body: == (H2: centered, underlined, numbered 1.1) and === (H3: left-aligned, underlined, numbered 1.1.1)."
    ),
    (
      key: "dollar_signs",
      description: "$ starts math mode in Typst prose. Use \\$ to escape (e.g., \\$17 million)."
    ),
    (
      key: "chapter_per_file",
      description: "Each chapter is one file in `chapters/`. ch01.typ exports `#let ch-name = [...]`. Entry file imports and calls `#chapter(number: \"1\", title: \"Title\", body: ch-name, first: true)`."
    ),
  ),
  functions: functions,
))
```

- [ ] **Step 2: Create CI workflow**

Create `.github/workflows/generate-reference.yml`:

```yaml
name: Generate Reference
on:
  push:
    paths:
      - 'institutions/*/template/sections/*.typ'
      - 'institutions/*/template/template.typ'
  workflow_dispatch:

jobs:
  generate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Typst
        run: |
          gh release download --repo typst/typst --pattern 'typst-x86_64-unknown-linux-musl.tar.xz'
          tar xJf typst-x86_64-unknown-linux-musl.tar.xz --strip-components=1
          sudo mv typst /usr/local/bin/
        env:
          GH_TOKEN: ${{ github.token }}
      - name: Generate REFERENCE.json
        run: |
          for profile_dir in institutions/*/template/; do
            if [ -f "$profile_dir/generate-json-ref.typ" ]; then
              echo "::group::Generating REFERENCE.json for ${profile_dir}"
              typst compile --root "$profile_dir" "$profile_dir/generate-json-ref.typ" "$profile_dir/REFERENCE.json"
              echo "::endgroup::"
            fi
          done
      - name: Verify REFERENCE.json exists and is valid JSON
        run: |
          for profile_dir in institutions/*/template/; do
            if [ -f "$profile_dir/generate-json-ref.typ" ]; then
              echo "Checking ${profile_dir}REFERENCE.json"
              python3 -m json.tool "${profile_dir}REFERENCE.json" > /dev/null
            fi
          done
      - name: Verify all functions have doc-comments
        run: |
          for profile_dir in institutions/*/template/; do
            if [ -f "$profile_dir/generate-json-ref.typ" ]; then
              echo "Checking doc-comment coverage for ${profile_dir}"
              python3 -c "
import json, sys
with open('${profile_dir}REFERENCE.json') as f:
    data = json.load(f)
missing = []
for func in data['functions']:
    desc = func.get('description', '').strip()
    if not desc:
        missing.append(func['name'])
if missing:
    print(f'ERROR: {len(missing)} functions missing doc-comments:')
    for name in missing:
        print(f'  - {name}')
    sys.exit(1)
print(f'All {len(data[\"functions\"])} functions have doc-comments.')
"
            fi
          done
      - name: Commit and push REFERENCE.json
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add institutions/*/template/REFERENCE.json
          if git diff --staged --quiet; then
            echo "No changes to REFERENCE.json"
          else
            git commit -m "chore: update REFERENCE.json [skip ci]"
            git push
          fi
```

- [ ] **Step 3: Run generation locally to produce initial REFERENCE.json**

```bash
cd institutions/iu/template
typst compile --root . generate-json-ref.typ REFERENCE.json
```

Verify the output is valid JSON:

```bash
python3 -m json.tool REFERENCE.json > /dev/null && echo "Valid JSON"
```

- [ ] **Step 4: Commit generate-json-ref.typ, REFERENCE.json, and CI workflow**

```bash
git add institutions/iu/template/generate-json-ref.typ \
        institutions/iu/template/REFERENCE.json \
        .github/workflows/generate-reference.yml
git commit -m "feat: add REFERENCE.json generation via tidy + CI workflow"
```

---

### Task 6: MCP tool + integration test

**Repo:** `scholarpress-backend`

**Files:**
- Modify: `crates/sp-mcp/src/tools.rs`
- Modify: `crates/sp-mcp/src/workspace.rs`

**Interfaces:**
- Produces: `interface_doc(workspace: PathBuf) -> String` in `workspace.rs`, wired as `scholarpress_interface_doc` MCP tool in `tools.rs`

- [ ] **Step 1: Add InterfaceDocParams struct to tools.rs**

After the existing `PandocConvertParams` struct (after line 56):

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct InterfaceDocParams {
    pub workspace: PathBuf,
}
```

- [ ] **Step 2: Add tool handler to tools.rs**

After the `format_typst` handler (after line 178), before the closing `}` of `impl ScholarPressService`:

```rust
    #[tool(
        description = "Returns a structured reference for every section function in the workspace template: function signatures, parameter types, default values, descriptions, and calling examples. Reads a pre-rendered template/REFERENCE.json — no compilation needed. Generated by CI from tidy doc-comments in the catalog."
    )]
    async fn interface_doc(
        &self,
        params: Parameters<InterfaceDocParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let doc = workspace::interface_doc(&p.workspace).map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(doc)]))
    }
```

- [ ] **Step 3: Add interface_doc function to workspace.rs**

After `pandoc_convert` (after line 449), before `#[cfg(test)] mod tests`:

```rust
pub fn interface_doc(workspace: &Path) -> Result<String, SpMcpError> {
    if !workspace.is_dir() {
        return Err(SpMcpError::WorkspaceNotFound(
            workspace.display().to_string(),
            workspace.to_path_buf(),
        ));
    }
    let ref_path = workspace.join("template").join("REFERENCE.json");
    if !ref_path.is_file() {
        return Err(SpMcpError::Compilation(format!(
            "REFERENCE.json not found at {}. Generate it by running CI in the catalog repo (generate-reference workflow), or run `typst compile generate-json-ref.typ REFERENCE.json` in the template directory.",
            ref_path.display()
        )));
    }
    let text = std::fs::read_to_string(&ref_path).map_err(|e| {
        SpMcpError::Compilation(format!(
            "failed to read REFERENCE.json at {}: {}",
            ref_path.display(), e
        ))
    })?;
    // Pretty-print the JSON so the agent can read it as formatted text
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        SpMcpError::Compilation(format!(
            "invalid JSON in REFERENCE.json at {}: {}",
            ref_path.display(), e
        ))
    })?;
    let pretty = serde_json::to_string_pretty(&value).map_err(|e| {
        SpMcpError::Compilation(format!(
            "failed to format REFERENCE.json: {}",
            e
        ))
    })?;
    Ok(pretty)
}
```

- [ ] **Step 4: Update server instructions in tools.rs**

The current `instructions` string in `get_info()` (line 189) is:

```rust
"ScholarPress: catalog + Typst template workspace tools. Use list_profiles to discover profiles, create_workspace to fork one into a scratch dir, then edit, compile_typst + check_pdf.\n\nWORKFLOW — Map–Scaffold–Migrate–Verify:\n\nMap — pandoc_convert(format: \"ast\") to survey structure, then scan pandoc_convert(format: \"typst\") output for Table of Contents. AST headings are unreliable (most DOCX uses direct formatting, not heading styles). The TOC is the source of truth for section count, order, and boundaries.\n\nScaffold — Create entry file with sections wired per template.typ comments (NAMED parameters, import pattern, chapter per-file convention).\n\nMigrate — One section at a time from the TOC: keyword-match the section title in pandoc typst output to find start boundary, keyword-match next section title for end boundary, slice the chunk, copy into the corresponding template section function. Run check_typst/format_typst on each section file, then compile_typst to catch errors early.\n\nVerify — compile_typst + check_pdf per milestone. Iterate incrementally.",
```

Replace with:

```rust
"ScholarPress: catalog + Typst template workspace tools. Use list_profiles to discover profiles, create_workspace to fork one into a scratch dir, then edit, compile_typst + check_pdf.\n\nWORKFLOW — Map–Scaffold–Migrate–Verify:\n\nInterface — After create_workspace, call interface_doc to see every section function's signature, parameter types, entry data shapes (dict vs tuple vs raw content), and calling examples. This eliminates guesswork about which functions expect dicts, tuples, or content.\n\nMap — pandoc_convert(format: \"ast\") to survey structure, then scan pandoc_convert(format: \"typst\") output for Table of Contents. AST headings are unreliable (most DOCX uses direct formatting, not heading styles). The TOC is the source of truth for section count, order, and boundaries.\n\nScaffold — Create entry file with sections wired per template.typ comments (NAMED parameters, import pattern, chapter per-file convention).\n\nMigrate — One section at a time from the TOC: keyword-match the section title in pandoc typst output to find start boundary, keyword-match next section title for end boundary, slice the chunk, copy into the corresponding template section function. Run check_typst/format_typst on each section file, then compile_typst to catch errors early.\n\nVerify — compile_typst + check_pdf per milestone. Iterate incrementally.",
```

- [ ] **Step 5: Write integration test in workspace.rs**

Add inside the `#[cfg(test)] mod tests` block, after the `pandoc_convert_missing_file_errors` test:

```rust
    #[test]
    fn interface_doc_reads_ref_json_or_errors_if_missing() {
        let ws = local_tempdir();
        // Case 1: no REFERENCE.json → error
        let result = interface_doc(&ws);
        match result {
            Err(SpMcpError::Compilation(msg)) => {
                assert!(msg.contains("REFERENCE.json not found"),
                    "expected message about missing file, got: {}", msg);
            }
            other => panic!("expected Compilation error for missing file, got {:?}", other),
        }

        // Case 2: REFERENCE.json exists → return pretty-printed content
        std::fs::create_dir_all(ws.join("template")).unwrap();
        fs::write(
            ws.join("template").join("REFERENCE.json"),
            r#"{"profile":"test","globals":[],"functions":[
                {"name":"foo","file":"sections/foo.typ","signature":"foo(x: 1)","description":"A test function.","params":[{"name":"x","type":"int","default":"1","description":"Test param"}]}
            ]}"#,
        ).unwrap();
        let result = interface_doc(&ws).unwrap();
        assert!(result.contains("\"foo\""), "output should contain function name");
        assert!(result.contains("\"signature\""), "output should contain signature field");
        assert!(result.contains("foo(x: 1)"), "output should contain the signature string");
    }

    #[test]
    fn interface_doc_workspace_not_found() {
        let result = interface_doc(Path::new("/nonexistent-ws-xyz"));
        assert!(matches!(result, Err(SpMcpError::WorkspaceNotFound(_, _))));
    }
```

- [ ] **Step 6: Build and run tests**

```bash
cargo build -p sp-mcp && cargo test -p sp-mcp
```

All existing tests must continue to pass. The two new interface_doc tests must pass.

- [ ] **Step 7: Commit**

```bash
git add crates/sp-mcp/src/tools.rs crates/sp-mcp/src/workspace.rs
git commit -m "feat: add scholarpress_interface_doc MCP tool"
```
