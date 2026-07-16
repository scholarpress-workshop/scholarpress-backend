use crate::checkers::{CheckResult, Status};
use crate::engine::{run_checks, CheckOptions};
use crate::report::{build_report, Summary};
use crate::spec::load_spec;
use serde::Serialize;
use std::path::Path;

const AUTO_CATS: &[&str] = &["layout", "typography", "structure", "content"];

#[derive(Debug, Serialize)]
pub struct DocumentResult {
    pub document: String,
    pub results: Vec<CheckResult>,
    pub summary: Summary,
}

#[derive(Debug, Serialize)]
pub struct CheckFrequency {
    pub check_id: String,
    pub category: String,
    pub pass_count: usize,
    pub fail_count: usize,
    pub manual_count: usize,
    pub error_count: usize,
    pub total_documents: usize,
    pub fail_documents: Vec<String>,
    pub fail_details: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CalibrationReport {
    pub spec_path: String,
    pub corpus_path: String,
    pub documents: Vec<String>,
    pub document_results: Vec<DocumentResult>,
    pub check_frequencies: Vec<CheckFrequency>,
}

impl CalibrationReport {
    const SYSTEMIC_THRESHOLD: f32 = 0.5;

    fn automated_checks(&self) -> Vec<&CheckFrequency> {
        self.check_frequencies
            .iter()
            .filter(|f| AUTO_CATS.contains(&f.category.as_str()))
            .collect()
    }

    pub fn systemic_fail_count(&self) -> usize {
        self.automated_checks()
            .iter()
            .filter(|f| {
                f.total_documents > 0
                    && f.fail_count as f32 / f.total_documents as f32 >= Self::SYSTEMIC_THRESHOLD
                    && f.fail_count >= 1
            })
            .count()
    }

    pub fn automated_fail_count(&self) -> usize {
        self.automated_checks()
            .iter()
            .filter(|f| f.fail_count > 0)
            .count()
    }
}

pub fn run_calibration(
    spec_path: &Path,
    corpus_path: &Path,
) -> Result<CalibrationReport, Box<dyn std::error::Error>> {
    let spec = load_spec(spec_path)?;
    let mut pdf_files: Vec<_> = std::fs::read_dir(corpus_path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    pdf_files.sort_by_key(|e| e.file_name());

    if pdf_files.is_empty() {
        return Err(format!("No PDF files found in {}", corpus_path.display()).into());
    }

    let mut check_freqs: std::collections::HashMap<String, CheckFrequency> =
        std::collections::HashMap::new();
    for check_def in &spec.checks {
        check_freqs.insert(
            check_def.id.clone(),
            CheckFrequency {
                check_id: check_def.id.clone(),
                category: check_def.category.clone(),
                pass_count: 0,
                fail_count: 0,
                manual_count: 0,
                error_count: 0,
                total_documents: 0,
                fail_documents: vec![],
                fail_details: vec![],
            },
        );
    }

    let mut document_results: Vec<DocumentResult> = Vec::new();

    for entry in &pdf_files {
        let path = entry.path();
        let filename = entry.file_name().to_string_lossy().to_string();

        let results = run_checks(&spec, &path, &CheckOptions::default())?;
        let report = build_report(results.clone());

        for result in &results {
            if let Some(freq) = check_freqs.get_mut(&result.check_id) {
                freq.total_documents += 1;
                match result.status {
                    Status::Pass => freq.pass_count += 1,
                    Status::Fail => {
                        freq.fail_count += 1;
                        freq.fail_documents.push(filename.clone());
                        freq.fail_details
                            .push(format!("[{}] {}", filename, result.detail));
                    }
                    Status::Manual => freq.manual_count += 1,
                    Status::Error => freq.error_count += 1,
                }
            }
        }

        document_results.push(DocumentResult {
            document: filename,
            results,
            summary: report.summary,
        });
    }

    Ok(CalibrationReport {
        spec_path: spec_path.display().to_string(),
        corpus_path: corpus_path.display().to_string(),
        documents: pdf_files
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect(),
        document_results,
        check_frequencies: check_freqs.into_values().collect(),
    })
}

pub fn format_text(report: &CalibrationReport) -> String {
    let mut lines: Vec<String> = vec![
        "=".repeat(70),
        "CALIBRATION REPORT".to_string(),
        "=".repeat(70),
        format!("Spec:     {}", report.spec_path),
        format!(
            "Corpus:   {} ({} documents)",
            report.corpus_path,
            report.documents.len()
        ),
        String::new(),
        "Documents:".to_string(),
    ];
    for doc in &report.documents {
        lines.push(format!("  - {}", doc));
    }

    lines.push(String::new());
    lines.push("-".repeat(70));

    for doc_result in &report.document_results {
        let s = &doc_result.summary;
        let mut parts = vec![];
        if s.pass > 0 {
            parts.push(format!("{} PASS", s.pass));
        }
        if s.fail > 0 {
            parts.push(format!("{} FAIL", s.fail));
        }
        if s.manual > 0 {
            parts.push(format!("{} MANUAL", s.manual));
        }
        if s.error > 0 {
            parts.push(format!("{} ERROR", s.error));
        }
        lines.push(format!("  {}: {}", doc_result.document, parts.join(", ")));
    }

    lines.push(String::new());
    lines.push("=".repeat(70));
    lines.push(String::new());
    lines.push("AUTOMATED CHECKS".to_string());
    lines.push(String::new());

    let auto_checks: Vec<_> = report
        .check_frequencies
        .iter()
        .filter(|f| AUTO_CATS.contains(&f.category.as_str()))
        .collect();

    for freq in &auto_checks {
        lines.push(format!("{} [{}]", freq.check_id, freq.category));
        lines.push(format!(
            "  PASS={} FAIL={} ERROR={}",
            freq.pass_count, freq.fail_count, freq.error_count
        ));

        if freq.fail_count > 0 {
            let fail_ratio = freq.fail_count as f32 / freq.total_documents as f32;
            if fail_ratio >= CalibrationReport::SYSTEMIC_THRESHOLD {
                lines.push(format!(
                    "  STATUS: SYSTEMIC ({}/{})",
                    freq.fail_count, freq.total_documents
                ));
            } else {
                lines.push(format!(
                    "  STATUS: isolated ({}/{})",
                    freq.fail_count, freq.total_documents
                ));
            }
            for detail in &freq.fail_details {
                lines.push(format!("    {}", detail));
            }
        } else {
            lines.push(format!(
                "  STATUS: clean ({}/{})",
                freq.pass_count, freq.total_documents
            ));
        }
        lines.push(String::new());
    }

    let manual_checks: Vec<_> = report
        .check_frequencies
        .iter()
        .filter(|f| !AUTO_CATS.contains(&f.category.as_str()))
        .collect();

    if !manual_checks.is_empty() {
        lines.push(String::new());
        lines.push("MANUAL CHECKS".to_string());
        lines.push(String::new());
        for freq in &manual_checks {
            lines.push(format!(
                "{} [{}] — MANUAL review ({}/{})",
                freq.check_id, freq.category, freq.manual_count, freq.total_documents
            ));
        }
        lines.push(String::new());
    }

    lines.push("=".repeat(70));
    lines.push(format!(
        "Automated checks with >=1 FAIL: {}",
        report.automated_fail_count()
    ));
    lines.push(format!(
        "Systemic FAILs (>={}% of corpus): {}",
        (CalibrationReport::SYSTEMIC_THRESHOLD * 100.0) as usize,
        report.systemic_fail_count()
    ));
    lines.push("=".repeat(70));

    lines.join("\n")
}

pub fn format_json(report: &CalibrationReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_freq(
        check_id: &str,
        category: &str,
        pass: usize,
        fail: usize,
        total: usize,
    ) -> CheckFrequency {
        CheckFrequency {
            check_id: check_id.to_string(),
            category: category.to_string(),
            pass_count: pass,
            fail_count: fail,
            manual_count: 0,
            error_count: 0,
            total_documents: total,
            fail_documents: vec![],
            fail_details: vec![],
        }
    }

    fn make_report(freqs: Vec<CheckFrequency>) -> CalibrationReport {
        CalibrationReport {
            spec_path: "spec.yaml".to_string(),
            corpus_path: "corpus/".to_string(),
            documents: vec!["doc1.pdf".to_string(), "doc2.pdf".to_string()],
            document_results: vec![],
            check_frequencies: freqs,
        }
    }

    #[test]
    fn test_systemic_fail_count() {
        let report = make_report(vec![
            make_freq("margins", "layout", 0, 2, 2),
            make_freq("font_size", "typography", 1, 1, 2),
            make_freq("justification", "typography", 2, 0, 2),
        ]);
        assert_eq!(report.systemic_fail_count(), 2);
        assert_eq!(report.automated_fail_count(), 2);
    }

    #[test]
    fn test_systemic_threshold() {
        let report = make_report(vec![make_freq("margins", "layout", 1, 1, 2)]);
        assert_eq!(report.systemic_fail_count(), 1);
    }

    #[test]
    fn test_isolated_not_systemic() {
        let report = make_report(vec![make_freq("margins", "layout", 2, 1, 3)]);
        assert_eq!(report.systemic_fail_count(), 0);
        assert_eq!(report.automated_fail_count(), 1);
    }

    #[test]
    fn test_format_text_structure() {
        let report = make_report(vec![
            make_freq("margins", "layout", 0, 2, 2),
            make_freq("font_size", "typography", 2, 0, 2),
            make_freq("human_check", "human", 0, 0, 2),
        ]);
        let output = format_text(&report);
        assert!(output.contains("CALIBRATION REPORT"));
        assert!(output.contains("AUTOMATED CHECKS"));
        assert!(output.contains("MANUAL CHECKS"));
        assert!(output.contains("SYSTEMIC"));
        assert!(output.contains("clean"));
    }

    #[test]
    fn test_format_json() {
        let report = make_report(vec![make_freq("margins", "layout", 1, 1, 2)]);
        let json = format_json(&report).unwrap();
        assert!(json.contains("margins"));
        assert!(json.contains("layout"));
    }

    #[test]
    fn test_empty_corpus_error() {
        let result = run_calibration(
            std::path::Path::new("../scholarpress-catalog/institutions/iu/spec.yaml"),
            std::path::Path::new("/nonexistent/path"),
        );
        assert!(result.is_err());
    }
}
