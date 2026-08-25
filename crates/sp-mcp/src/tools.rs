use crate::config::Config;
use crate::error::SpMcpError;
use crate::workspace;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ErrorData as McpError, *};
use rmcp::{tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

/// Full agent-facing instructions surfaced via `ServerHandler::get_info()`.
/// Edited in one place so unit tests can assert its content.
pub(crate) const SERVER_INSTRUCTIONS: &str = r#"When given a source document whose completeness is unclear, do NOT immediately call pandoc_convert, compile_typst, or check_pdf. First ask the user three things, in one `question` tool call: (1) a multi-select checklist of which sections are present in the draft (use the profile's section IDs, e.g. for IU: title_page, acceptance_page, copyright_page, dedication, acknowledgements, preface, abstract, toc, list_of_tables, list_of_figures, list_of_abbreviations, chapters, references, appendices, curriculum_vitae — derive the list from `spec.yaml` via interface_doc if the profile isn't IU); (2) how to handle missing required sections — three options: format only the present sections, help build the missing required sections interactively, or produce a structural skeleton the author fills in later; (3) a free-text prompt for any conventions the author used for the present sections (e.g. chapter break style, numbering, figure labeling). Wait for the user's response before proceeding.

When the skeleton branch is chosen, omit calls to each missing required section function entirely and leave a `// PLACEHOLDER(<section_id>): <one-line hint>` comment at the position where the call would go. Do NOT call missing section functions with placeholder parameter values — parameter types in the template may not accept plain strings, and the compile will fail. The skeleton produces a PDF of whatever sections are present; the structural checks (e.g. `front_matter_presence`) will FAIL with a clear missing-section list, which the agent should report back to the author alongside the `// PLACEHOLDER(` markers. Later runs can grep `// PLACEHOLDER(` to find what still needs content.

After the first compile_typst + check_pdf cycle completes, produce the annotated PDF (annotate_pdf is a visual artifact, safe to generate) and then pause and report findings to the user before iterating on fixes or starting to actively troubleshoot. Errors of assumption compound silently — the user may want to revise their initial intake answers (e.g. correct the section checklist, switch branches) rather than have the agent patch assumptions. Present the check_pdf outcome and wait for explicit confirmation before the next action.

After pandoc_convert(format: "ast") and pandoc_convert(format: "typst"), but before creating entry.typ or editing chapter Typst files, present a concise inferred map and wait for confirmation or correction. Keep the initial section checklist coarse; infer individual chapters from the converted source instead of asking the user to enumerate a large chapter list. The map must contain Detected chapter titles and numbers, Expected chapters that appear absent, Chapter boundaries, Candidate section and subsection boundaries, heading-level mappings based on source formatting and author conventions, Referenced media and required asset paths, and Chapter-file exports and corresponding imports. Example: "I infer: - Chapter 1: present - Results: H1, centered uppercase - Analysis: H1, centered uppercase - Chapter 2: present - Chapter 3: not detected - Images referenced: image1.png, image2.png. Please confirm or correct this map before I create entry.typ or edit chapter files."

For the IU template, make heading consequences explicit when confirming mappings: = / H1: centered uppercase, == / H2: centered and underlined, and === / H3: left-aligned and underlined. If a top-level subsection such as Results could be a chapter title, H1 section, or H2 subsection, the agent must not silently make that choice; ask a targeted clarification. A user's conventions answer is a hypothesis source, not a final mapping.

Before writing raw Typst, validate: Nested Typst brackets with a bracket-depth-aware parser, not naive regex; Every referenced image or media asset exists in the workspace; Every chapter import name matches the corresponding exported #let binding; and the confirmed chapter and heading map matches source content boundaries. Do not compile until these predictable delimiter, asset, import/export, and mapping failures are resolved or explicitly reported to the user.

ScholarPress workflow: call list_profiles, create_workspace, and interface_doc first. Convert the DOCX with pandoc_convert(format: "ast") and pandoc_convert(format: "typst") as needed. Write entry.typ and chapter files, then call compile_typst and check_pdf. Use check_ids to isolate PDF failures. After check_pdf reports failures, call annotate_pdf to produce a visually annotated copy for the author. The source DOCX is the source of truth for content; the workspace template is the source of truth for formatting — never edit the template to reproduce the source document's formatting. Pandoc output is best effort; use TOC text to map sections and clean Typst artifacts such as #underline[...] and #strong[...]."#;

#[derive(Clone)]
pub struct ScholarPressService {
    config: Arc<Config>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateWorkspaceParams {
    /// Workspace name to create under SCHOLARPRESS_WORKSPACE_ROOT.
    pub name: String,
    /// Catalog profile ID returned by list_profiles, such as institutions/iu-indianapolis.
    pub profile_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompileTypstParams {
    /// Absolute workspace path returned by create_workspace.
    pub workspace: PathBuf,
    /// Entry file path relative to the workspace, normally entry.typ.
    pub entry_path: PathBuf,
    /// Output filename only; written below the workspace's out/ directory.
    pub out_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckPdfParams {
    /// Absolute workspace path returned by create_workspace.
    pub workspace: PathBuf,
    /// PDF path relative to the workspace, normally out/entry.pdf.
    pub pdf_path: PathBuf,
    /// Optional check IDs to run; omit to run every check.
    pub check_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnnotatePdfParams {
    /// Absolute workspace path returned by create_workspace.
    pub workspace: PathBuf,
    /// PDF path relative to the workspace, normally out/entry.pdf.
    pub pdf_path: PathBuf,
    /// Optional check IDs to run; omit to run every check.
    pub check_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PandocConvertParams {
    /// DOCX input path relative to the workspace.
    pub file_path: PathBuf,
    /// Conversion format: typst or ast.
    pub format: String,
    /// Absolute workspace path returned by create_workspace.
    pub workspace: PathBuf,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InterfaceDocParams {
    /// Absolute workspace path returned by create_workspace.
    pub workspace: PathBuf,
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
        McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
    }

    #[tool(description = "List existing workspaces under SCHOLARPRESS_WORKSPACE_ROOT.")]
    async fn list_workspaces(&self) -> Result<CallToolResult, McpError> {
        let list = workspace::list_workspaces(&self.config).map_err(Self::err)?;
        let json = serde_json::to_string(&list)
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(description = "List catalog profiles available to create_workspace.")]
    async fn list_profiles(&self) -> Result<CallToolResult, McpError> {
        let list = workspace::list_profiles(&self.config).map_err(Self::err)?;
        let json = serde_json::to_string(&list)
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(
        description = "Create a workspace by copying a catalog profile into SCHOLARPRESS_WORKSPACE_ROOT. Returns the absolute workspace path."
    )]
    async fn create_workspace(
        &self,
        params: Parameters<CreateWorkspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let path =
            workspace::create_workspace(&self.config, &p.name, &p.profile_id).map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            path.display().to_string(),
        )]))
    }

    #[tool(
        description = "Compile a Typst entry file within a workspace. Writes the PDF below <workspace>/out/ and returns its absolute path. The entry path is relative to the workspace. Requires the typst executable."
    )]
    async fn compile_typst(
        &self,
        params: Parameters<CompileTypstParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let out = workspace::compile_typst(
            &self.config,
            &p.workspace,
            &p.entry_path,
            p.out_name.as_deref(),
        )
        .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            out.display().to_string(),
        )]))
    }

    #[tool(
        description = "Run the workspace spec's formatting checks against a PDF. Optionally limit execution with check_ids."
    )]
    async fn check_pdf(
        &self,
        params: Parameters<CheckPdfParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let outcomes = workspace::check_pdf(
            &self.config,
            &p.workspace,
            &p.pdf_path,
            p.check_ids.as_deref(),
        )
        .map_err(Self::err)?;
        let json = serde_json::to_string(&outcomes)
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(
        description = "Run the workspace spec's checks against a PDF, then write an annotated copy (prepended summary page + in-place highlights and sticky notes for findings) below <workspace>/out/. Returns the annotated PDF's absolute path."
    )]
    async fn annotate_pdf(
        &self,
        params: Parameters<AnnotatePdfParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let out = workspace::annotate_pdf(
            &self.config,
            &p.workspace,
            &p.pdf_path,
            p.check_ids.as_deref(),
        )
        .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            out.display().to_string(),
        )]))
    }

    #[tool(
        description = "Convert a workspace DOCX to Typst source or pandoc JSON AST. Use typst or ast as the format. TOC text is more reliable than AST headings for section mapping."
    )]
    async fn pandoc_convert(
        &self,
        params: Parameters<PandocConvertParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let out = workspace::pandoc_convert(&self.config, &p.file_path, &p.format, &p.workspace)
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            out.display().to_string(),
        )]))
    }

    #[tool(
        description = "Return the template's generated REFERENCE.json with section function signatures, parameter types, defaults, and examples."
    )]
    async fn interface_doc(
        &self,
        params: Parameters<InterfaceDocParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let doc = workspace::interface_doc(&self.config, &p.workspace).map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(doc)]))
    }
}

#[tool_handler]
impl rmcp::handler::server::ServerHandler for ScholarPressService {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder().enable_tools().build();
        let server_info = Implementation::new("scholarpress-mcp", env!("CARGO_PKG_VERSION"));
        ServerInfo::new(capabilities)
            .with_server_info(server_info)
            .with_instructions(SERVER_INSTRUCTIONS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_instructions_include_partial_draft_intake_prompt() {
        // The intake guidance must be at the START of the instructions,
        // before the existing "ScholarPress workflow:" sentence.
        let prefix_end = SERVER_INSTRUCTIONS
            .find("ScholarPress workflow:")
            .expect("legacy workflow text must still be present");
        let prefix = &SERVER_INSTRUCTIONS[..prefix_end];
        assert!(
            prefix.contains("do NOT immediately call"),
            "intake prompt must warn agent not to jump straight to pandoc_convert/compile_typst; prefix was: {prefix}"
        );
        assert!(
            prefix.contains("three things"),
            "intake prompt must enumerate three intake questions; prefix was: {prefix}"
        );
        assert!(
            prefix.contains("Wait for the user"),
            "intake prompt must instruct the agent to wait for the response; prefix was: {prefix}"
        );
    }

    #[test]
    fn server_instructions_include_placeholder_convention() {
        assert!(
            SERVER_INSTRUCTIONS.contains("PLACEHOLDER(<section_id>)"),
            "skeleton-branch guidance must document the PLACEHOLDER comment convention"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("Do NOT call missing section functions"),
            "skeleton-branch guidance must warn against calling section functions with placeholder values"
        );
    }

    #[test]
    fn server_instructions_include_hitl_pause() {
        assert!(
            SERVER_INSTRUCTIONS.contains("pause and report findings"),
            "HITL pause guidance must be present after the first compile/check cycle"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("explicit confirmation"),
            "HITL pause guidance must require explicit confirmation before the next action"
        );
    }

    #[test]
    fn server_instructions_require_confirmed_inference_before_typst_edits() {
        let inference = SERVER_INSTRUCTIONS
            .find("After pandoc_convert(format: \"ast\") and pandoc_convert(format: \"typst\")")
            .expect("inference checkpoint must follow both conversions");
        let edits = SERVER_INSTRUCTIONS
            .find("before creating entry.typ or editing chapter Typst files")
            .expect("inference checkpoint must precede raw Typst edits");
        assert!(
            inference < edits,
            "inference checkpoint must precede raw edits"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("Detected chapter titles and numbers"),
            "map must include detected chapter titles and numbers"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("Expected chapters that appear absent"),
            "map must include chapters that appear absent"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("Chapter boundaries"),
            "map must include chapter boundaries"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("Candidate section and subsection boundaries"),
            "map must include candidate section and subsection boundaries"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("Referenced media and required asset paths"),
            "map must include media and asset paths"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("Chapter-file exports and corresponding imports"),
            "map must include chapter exports and imports"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("Please confirm or correct this map"),
            "agent must wait for map confirmation"
        );
    }

    #[test]
    fn server_instructions_make_heading_level_consequences_explicit() {
        assert!(
            SERVER_INSTRUCTIONS.contains("= / H1: centered uppercase"),
            "instructions must document IU H1 rendering"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("== / H2: centered and underlined"),
            "instructions must document IU H2 rendering"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("=== / H3: left-aligned and underlined"),
            "instructions must document IU H3 rendering"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("must not silently make that choice"),
            "ambiguous heading mappings must require user confirmation"
        );
        assert!(
            SERVER_INSTRUCTIONS.contains("targeted clarification"),
            "ambiguous heading mappings must trigger targeted clarification"
        );
    }

    #[test]
    fn server_instructions_require_preflight_validation() {
        let preflight = SERVER_INSTRUCTIONS
            .find("Before writing raw Typst, validate:")
            .expect("preflight section must be present");
        let instructions = &SERVER_INSTRUCTIONS[preflight..];
        assert!(
            instructions.contains("Nested Typst brackets with a bracket-depth-aware parser"),
            "preflight must validate nested Typst brackets safely"
        );
        assert!(
            instructions.contains("Every referenced image or media asset exists in the workspace"),
            "preflight must validate referenced media"
        );
        assert!(
            instructions.contains(
                "Every chapter import name matches the corresponding exported #let binding"
            ),
            "preflight must validate chapter imports and exports"
        );
        assert!(
            instructions
                .contains("confirmed chapter and heading map matches source content boundaries"),
            "preflight must validate the confirmed map against source boundaries"
        );
    }
}
