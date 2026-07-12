use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    Extraction(String),
    Compilation(String),
    Validation(String),
    InstitutionNotFound(String),
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Extraction(m) => write!(f, "Extraction failed: {}", m),
            AppError::Compilation(m) => write!(f, "Compilation failed: {}", m),
            AppError::Validation(m) => write!(f, "Validation failed: {}", m),
            AppError::InstitutionNotFound(m) => write!(f, "Institution not found: {}", m),
            AppError::Internal(m) => write!(f, "Internal error: {}", m),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::InstitutionNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
