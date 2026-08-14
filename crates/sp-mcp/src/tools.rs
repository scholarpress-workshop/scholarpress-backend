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
            .with_instructions(
                "ScholarPress workflow: call list_profiles, create_workspace, and interface_doc first. Convert the DOCX with pandoc_convert(format: \"ast\") and pandoc_convert(format: \"typst\") as needed. Write entry.typ and chapter files, then call compile_typst and check_pdf. Use check_ids to isolate PDF failures. The source DOCX is the source of truth for content; the workspace template is the source of truth for formatting — never edit the template to reproduce the source document's formatting. Pandoc output is best effort; use TOC text to map sections and clean Typst artifacts such as #underline[...] and #strong[...].",
            )
    }
}
