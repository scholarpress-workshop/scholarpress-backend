use crate::checkers::{get_checker, CheckResult, Status};
use crate::spec::InstitutionSpec;
use sp_extract::document::ParsedDocument;
use std::path::Path;

#[derive(Default)]
pub struct CheckOptions {
    pub check_ids: Option<Vec<String>>,
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
        if let Some(ref ids) = options.check_ids {
            if !ids.is_empty() && !ids.contains(&check_def.id) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::InstitutionSpec;

    fn minimal_spec() -> InstitutionSpec {
        let yaml = r#"
institution: Test
source_revision: "1"
document_structure:
  front_matter: []
  body:
    - id: body
      required: true
  end_matter: []
checks:
  - id: test_margins
    category: layout
    checker: margins
    target:
      scope: all_pages
    automatable: false
    review_hint: "check margins"
  - id: test_font_size
    category: typography
    checker: font_size
    target:
      scope: all_pages
    automatable: false
    review_hint: "check font size"
  - id: test_justification
    category: typography
    checker: justification
    target:
      scope: all_pages
    automatable: false
    review_hint: "check justification"
"#;
        serde_yaml::from_str(yaml).unwrap()
    }

    fn minimal_pdf(dir: &std::path::Path) -> std::path::PathBuf {
        let typ = dir.join("minimal.typ");
        std::fs::write(&typ, "= Hello\n").unwrap();
        let out = dir.join("minimal.pdf");
        let status = std::process::Command::new("typst")
            .arg("compile")
            .arg(&typ)
            .arg(&out)
            .output();
        match status {
            Ok(o) if o.status.success() => out,
            _ => {
                eprintln!("SKIP: typst not on PATH");
                dir.join("missing.pdf")
            }
        }
    }

    #[test]
    fn run_checks_filters_by_check_ids() {
        let dir = std::env::temp_dir().join(format!("sp-check-eng-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pdf = minimal_pdf(&dir);
        if !pdf.is_file() {
            return;
        }
        let spec = minimal_spec();
        let options = CheckOptions {
            check_ids: Some(vec!["test_margins".into(), "test_justification".into()]),
            ..Default::default()
        };
        let results = run_checks(&spec, &pdf, &options).unwrap();
        let ids: Vec<&str> = results.iter().map(|r| r.check_id.as_str()).collect();
        assert_eq!(ids, vec!["test_margins", "test_justification"]);
    }

    #[test]
    fn run_checks_unknown_check_id_returns_empty() {
        let dir = std::env::temp_dir().join(format!("sp-check-eng2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pdf = minimal_pdf(&dir);
        if !pdf.is_file() {
            return;
        }
        let spec = minimal_spec();
        let options = CheckOptions {
            check_ids: Some(vec!["nonexistent".into()]),
            ..Default::default()
        };
        let results = run_checks(&spec, &pdf, &options).unwrap();
        assert!(results.is_empty());
    }
}
