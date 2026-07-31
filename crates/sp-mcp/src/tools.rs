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

#[derive(Clone)]
pub struct ScholarPressService {
    config: Arc<Config>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateWorkspaceParams {
    pub name: String,
    pub profile_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompileTypstParams {
    pub workspace: PathBuf,
    pub entry_path: PathBuf,
    pub out_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckPdfParams {
    pub workspace: PathBuf,
    pub pdf_path: PathBuf,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckTypstParams {
    pub workspace: PathBuf,
    pub file_path: PathBuf,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FormatTypstParams {
    pub workspace: PathBuf,
    pub file_path: PathBuf,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PandocConvertParams {
    pub file_path: PathBuf,
    pub format: String,
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

    #[tool(
        description = "List existing workspaces under SCHOLARPRESS_WORKSPACE_ROOT. Returns name, absolute path, profile_id (if spec.yaml identifies one), and mtime."
    )]
    async fn list_workspaces(&self) -> Result<CallToolResult, McpError> {
        let list = workspace::list_workspaces(&self.config).map_err(Self::err)?;
        let json = serde_json::to_string(&list)
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(
        description = "List available profiles in the catalog. Returns id (e.g. 'institutions/iu'), scope, and human-readable name."
    )]
    async fn list_profiles(&self) -> Result<CallToolResult, McpError> {
        let list = workspace::list_profiles(&self.config).map_err(Self::err)?;
        let json = serde_json::to_string(&list)
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(
        description = "Create a new workspace by copying a catalog profile (spec.yaml + template/) into a named dir under the workspace root. Returns the absolute path."
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
        description = "Compile a Typst entry file within a workspace. Writes the PDF to <workspace>/out/<out_name>.pdf (default = entry stem). Returns the absolute output path. Requires the `typst` binary on PATH. To pass structured data, write <workspace>/data.json with the agent's file tools before calling — the typst template can read it with `json(\"data.json\")` or `read(\"data.json\")`."
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
        description = "Run formatting checks against the workspace's spec.yaml. Always uses the workspace spec. Returns a list of check outcomes (id, status, message, page)."
    )]
    async fn check_pdf(
        &self,
        params: Parameters<CheckPdfParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let outcomes =
            workspace::check_pdf(&self.config, &p.workspace, &p.pdf_path).map_err(Self::err)?;
        let json = serde_json::to_string(&outcomes)
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(
        description = "Convert a DOCX file to Typst or pandoc JSON AST. Writes output to <workspace>/out/<stem>.typ (for format: \"typst\") or <workspace>/out/<stem>.json (for format: \"ast\"). Returns the absolute output path. Requires `pandoc` on PATH (already installed).\n\nAST headings are unreliable — many DOCX files use direct formatting instead of heading styles. Prefer TOC text over AST for section boundaries."
    )]
    async fn pandoc_convert(
        &self,
        params: Parameters<PandocConvertParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let out = workspace::pandoc_convert(&p.file_path, &p.format, &p.workspace)
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            out.display().to_string(),
        )]))
    }

    #[tool(
        description = "Validate Typst syntax without full compilation. Runs `typstyle --check`. Returns \"ok\" if the file is properly formatted, \"needs_format\" if issues found (e.g., $ in prose, unclosed delimiters). Requires `typstyle` on PATH (install with `cargo install typstyle --locked`)."
    )]
    async fn check_typst(
        &self,
        params: Parameters<CheckTypstParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let status = workspace::check_typst(&p.workspace, &p.file_path).map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(status)]))
    }

    #[tool(
        description = "Format a Typst file in-place. Runs `typstyle -i` to normalize indentation, whitespace, and line width. Returns the absolute path of the formatted file. Requires `typstyle` on PATH (install with `cargo install typstyle --locked`)."
    )]
    async fn format_typst(
        &self,
        params: Parameters<FormatTypstParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let path = workspace::format_typst(&p.workspace, &p.file_path).map_err(Self::err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(path)]))
    }
}

#[tool_handler]
impl rmcp::handler::server::ServerHandler for ScholarPressService {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder().enable_tools().build();
        let server_info = Implementation::new("scholarpress-mcp", env!("CARGO_PKG_VERSION"));
        ServerInfo::new(capabilities)
            .with_server_info(server_info)
            .with_instructions(
                "ScholarPress: catalog + Typst template workspace tools. Use list_profiles to discover profiles, create_workspace to fork one into a scratch dir, then edit, compile_typst + check_pdf.\n\nWORKFLOW — Map–Scaffold–Migrate–Verify:\n\nMap — pandoc_convert(format: \"ast\") to survey structure, then scan pandoc_convert(format: \"typst\") output for Table of Contents. AST headings are unreliable (most DOCX uses direct formatting, not heading styles). The TOC is the source of truth for section count, order, and boundaries.\n\nScaffold — Create entry file with sections wired per template.typ comments (NAMED parameters, import pattern, chapter per-file convention).\n\nMigrate — One section at a time from the TOC: keyword-match the section title in pandoc typst output to find start boundary, keyword-match next section title for end boundary, slice the chunk, copy into the corresponding template section function. Run check_typst/format_typst on each section file, then compile_typst to catch errors early.\n\nVerify — compile_typst + check_pdf per milestone. Iterate incrementally.",
            )
    }
}
