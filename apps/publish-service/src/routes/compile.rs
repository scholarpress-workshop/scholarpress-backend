use crate::error::AppError;
use crate::institutions::Registry;
use axum::{
    extract::{Json, Query, State},
    http::header,
    response::IntoResponse,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CompileRequest {
    pub typst_code: String,
    #[serde(default)]
    pub variables: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize)]
pub struct CompileParams {
    pub institution: String,
}

pub async fn handler(
    State(registry): State<Registry>,
    Query(params): Query<CompileParams>,
    Json(body): Json<CompileRequest>,
) -> Result<axum::response::Response, AppError> {
    let institution = registry
        .get(&params.institution)
        .ok_or_else(|| AppError::InstitutionNotFound(params.institution.clone()))?;

    let code = if let Some(ref vars) = body.variables {
        sp_typst::template::render_template(&body.typst_code, vars)
    } else {
        body.typst_code
    };

    let pdf = sp_typst::compile(&code, Some(&institution.template_dir))
        .map_err(|e| AppError::Compilation(e.to_string()))?;

    Ok(([(header::CONTENT_TYPE, "application/pdf")], pdf).into_response())
}
