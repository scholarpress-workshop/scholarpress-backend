use crate::checkers::{get_checker, CheckResult, Status};
use crate::spec::InstitutionSpec;
use sp_extract::document::ParsedDocument;
use std::path::Path;

#[derive(Default)]
pub struct CheckOptions {
    pub check_id: Option<String>,
    pub category: Option<String>,
}

pub fn run_checks(
    spec: &InstitutionSpec,
    pdf_path: &Path,
    options: &CheckOptions,
) -> Result<Vec<CheckResult>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(pdf_path)?;
    let doc: ParsedDocument = sp_extract::extract_pdf(&bytes)?;
    let mut results: Vec<CheckResult> = Vec::new();

    for check_def in &spec.checks {
        if let Some(ref filter_id) = options.check_id {
            if check_def.id != *filter_id {
                continue;
            }
        }
        if let Some(ref filter_cat) = options.category {
            if check_def.category != *filter_cat {
                continue;
            }
        }

        if !check_def.automatable {
            results.push(CheckResult {
                check_id: check_def.id.clone(),
                status: Status::Manual,
                evidence: vec![],
                detail: check_def
                    .review_hint
                    .clone()
                    .unwrap_or_else(|| "Manual review required".to_string()),
            });
            continue;
        }

        match get_checker(&check_def.category, &check_def.checker) {
            Some(checker) => {
                let params = serde_yaml::to_value(&check_def.params).unwrap_or_default();
                let mut result = checker.check(&doc, &params);
                result.check_id = check_def.id.clone();
                results.push(result);
            }
            None => {
                results.push(CheckResult {
                    check_id: check_def.id.clone(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: format!(
                        "No checker registered for {}/{}",
                        check_def.category, check_def.checker
                    ),
                });
            }
        }
    }

    Ok(results)
}
