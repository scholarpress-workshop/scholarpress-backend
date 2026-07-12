use crate::error::AppError;
use crate::institutions::Registry;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct SpecResponse {
    pub id: String,
    pub yaml: String,
    pub summary: SpecSummary,
}

#[derive(Serialize)]
pub struct SpecSummary {
    pub document_structure: serde_yaml::Value,
    pub constants: serde_yaml::Value,
    pub automated_checks: usize,
    pub human_checks: usize,
}

pub async fn handler(
    State(registry): State<Registry>,
    Path(id): Path<String>,
) -> Result<Json<SpecResponse>, AppError> {
    let inst = registry
        .get(&id)
        .ok_or_else(|| AppError::InstitutionNotFound(id.clone()))?;

    let yaml = serde_yaml::to_string(&inst.spec).map_err(|e| AppError::Internal(e.to_string()))?;

    let checks = inst.spec.get("checks").and_then(|c| c.as_sequence());

    let automated = checks
        .map(|c| {
            c.iter()
                .filter(|ch| {
                    ch.get("automatable")
                        .and_then(|a| a.as_bool())
                        .unwrap_or(true)
                })
                .count()
        })
        .unwrap_or(0);

    let human = checks
        .map(|c| {
            c.iter()
                .filter(|ch| {
                    !ch.get("automatable")
                        .and_then(|a| a.as_bool())
                        .unwrap_or(true)
                })
                .count()
        })
        .unwrap_or(0);

    Ok(Json(SpecResponse {
        id: inst.id.clone(),
        yaml,
        summary: SpecSummary {
            document_structure: inst
                .spec
                .get("document_structure")
                .cloned()
                .unwrap_or_default(),
            constants: inst.spec.get("constants").cloned().unwrap_or_default(),
            automated_checks: automated,
            human_checks: human,
        },
    }))
}
