use crate::error::AppError;
use axum::{extract::Multipart, Json};

pub async fn handler(mut multipart: Multipart) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Extraction(e.to_string()))?
    {
        let content_type = field.content_type().map(|c| c.to_string());
        let name = field.file_name().unwrap_or("unknown").to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::Extraction(e.to_string()))?;

        tracing::info!(filename = %name, mime = ?content_type, size = data.len(), "extracting document");

        let mime = content_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        let parsed =
            match mime {
                "application/pdf" => sp_extract::extract_pdf(&data)
                    .map_err(|e| {
                        tracing::error!(filename = %name, error = %e, "PDF extraction failed");
                        AppError::Extraction(e.to_string())
                    })?,
                mt if mt.contains("wordprocessingml") => sp_extract::extract_docx(&data)
                    .map_err(|e| {
                        tracing::error!(filename = %name, error = %e, "DOCX extraction failed");
                        AppError::Extraction(e.to_string())
                    })?,
                _ => {
                    return Err(AppError::Extraction(format!(
                        "Unsupported format: {}",
                        mime
                    )))
                }
            };

        return Ok(Json(serde_json::to_value(parsed)?));
    }
    Err(AppError::Extraction("No file uploaded".into()))
}
