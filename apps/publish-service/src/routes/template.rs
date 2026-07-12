use crate::error::AppError;
use crate::institutions::Registry;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct TemplateResponse {
    pub id: String,
    pub entry: String,
    pub files: Vec<TemplateFileRef>,
}

#[derive(Serialize)]
pub struct TemplateFileRef {
    pub path: String,
    pub content: String,
}

pub async fn handler(
    State(registry): State<Registry>,
    Path(id): Path<String>,
) -> Result<Json<TemplateResponse>, AppError> {
    let inst = registry
        .get(&id)
        .ok_or_else(|| AppError::InstitutionNotFound(id.clone()))?;

    let template_set = sp_typst::template::load_template(&inst.template_dir)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(TemplateResponse {
        id: inst.id.clone(),
        entry: template_set.entry,
        files: template_set
            .files
            .into_iter()
            .map(|f| TemplateFileRef {
                path: f.path,
                content: f.content,
            })
            .collect(),
    }))
}
