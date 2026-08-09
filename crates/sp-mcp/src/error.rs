use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SpMcpError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("profile {0:?} not found in catalog; available: {1:?}")]
    ProfileNotFound(String, Vec<String>),

    #[error("workspace {0:?} not found under {1}")]
    WorkspaceNotFound(String, PathBuf),

    #[error("workspace {0} already exists")]
    WorkspaceExists(PathBuf),

    #[error("path boundary violation: {0}")]
    PathViolation(String),

    #[error("invalid workspace name {0:?} (must be non-empty, no '/' or '..')")]
    BadWorkspaceName(String),

    #[error("spec.yaml not found at {0}")]
    SpecMissing(PathBuf),

    #[error("extraction failed: {0}")]
    Extraction(String),

    #[error("conversion failed: {0}")]
    Conversion(String),

    #[error("compilation failed: {0}")]
    Compilation(String),

    #[error("check failed: {0}")]
    Check(String),

    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),
}
