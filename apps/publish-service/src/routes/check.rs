use crate::error::AppError;
use crate::institutions::Registry;
use axum::extract::{Json, State};
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CheckRequest {
    pub pdf_base64: String,
    pub institution: String,
}

#[derive(Serialize)]
pub struct CheckResponse {
    pub violations: Vec<CheckViolation>,
    pub pass_count: usize,
    pub fail_count: usize,
    pub error_count: usize,
}

#[derive(Serialize)]
pub struct CheckViolation {
    pub check_id: String,
    pub status: String,
    pub detail: String,
    pub page: Option<i32>,
}

pub async fn handler(
    State(registry): State<Registry>,
    Json(body): Json<CheckRequest>,
) -> Result<Json<CheckResponse>, AppError> {
    let institution = registry
        .get(&body.institution)
        .ok_or_else(|| AppError::InstitutionNotFound(body.institution.clone()))?;

    let pdf_bytes = base64::engine::general_purpose::STANDARD
        .decode(&body.pdf_base64)
        .map_err(|e| AppError::Check(format!("Invalid base64: {}", e)))?;

    let tmp_dir =
        std::env::temp_dir().join(format!("scholarpress-check-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_dir)?;
    let pdf_path = tmp_dir.join("input.pdf");
    std::fs::write(&pdf_path, &pdf_bytes)?;

    let spec_path = tmp_dir.join("spec.yaml");
    let spec_yaml = serde_yaml::to_string(&institution.spec)
        .map_err(|e| AppError::Check(e.to_string()))?;
    std::fs::write(&spec_path, &spec_yaml)?;

    let spec = sp_check::spec::load_spec(&spec_path)
        .map_err(|e| AppError::Check(e.to_string()))?;

    let options = sp_check::engine::CheckOptions::default();
    let results = sp_check::engine::run_checks(&spec, &pdf_path, &options)
        .map_err(|e| AppError::Check(e.to_string()))?;

    let report = sp_check::report::build_report(results);

    let violations: Vec<CheckViolation> = report
        .results
        .iter()
        .map(|r| {
            let page = r.evidence.first().map(|e| e.page as i32);
            CheckViolation {
                check_id: r.check_id.clone(),
                status: format!("{:?}", r.status),
                detail: r.detail.clone(),
                page,
            }
        })
        .collect();

    std::fs::remove_dir_all(&tmp_dir).ok();

    Ok(Json(CheckResponse {
        violations,
        pass_count: report.summary.pass,
        fail_count: report.summary.fail,
        error_count: report.summary.error,
    }))
}
