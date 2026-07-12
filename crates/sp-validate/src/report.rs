use crate::checkers::{CheckResult, Status};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Summary {
    pub pass: usize,
    pub fail: usize,
    pub manual: usize,
    pub error: usize,
}

#[derive(Debug)]
pub struct Report {
    pub results: Vec<CheckResult>,
    pub summary: Summary,
}

pub fn build_report(results: Vec<CheckResult>) -> Report {
    let mut summary = Summary {
        pass: 0,
        fail: 0,
        manual: 0,
        error: 0,
    };

    for r in &results {
        match r.status {
            Status::Pass => summary.pass += 1,
            Status::Fail => summary.fail += 1,
            Status::Manual => summary.manual += 1,
            Status::Error => summary.error += 1,
        }
    }

    Report { results, summary }
}

pub fn format_text(report: &Report) -> String {
    format_text_with_options(report, false)
}

pub fn format_text_quiet(report: &Report) -> String {
    format_text_with_options(report, true)
}

fn format_text_with_options(report: &Report, quiet: bool) -> String {
    let mut lines: Vec<String> = vec![];

    if !quiet {
        lines.push("=".repeat(60).to_string());
        lines.push("DISSERTATION FORMAT CHECK REPORT".to_string());
        lines.push("=".repeat(60).to_string());
    }

    for result in &report.results {
        let marker = match result.status {
            Status::Pass => {
                if quiet {
                    continue;
                } else {
                    "[PASS]"
                }
            }
            Status::Fail => "[FAIL]",
            Status::Manual => {
                if quiet {
                    continue;
                } else {
                    "[MANUAL]"
                }
            }
            Status::Error => "[ERROR]",
        };

        lines.push(format!("\n{} {}", marker, result.check_id));

        if !result.detail.is_empty() {
            lines.push(format!("  {}", result.detail));
        }

        for ev in &result.evidence {
            let mut page_info = format!("page {}", ev.page);
            if let Some(bbox) = ev.bbox {
                page_info.push_str(&format!(
                    " @ ({:.0},{:.0},{:.0},{:.0})",
                    bbox.0, bbox.1, bbox.2, bbox.3
                ));
            }
            let excerpt = ev.excerpt.as_deref().unwrap_or("");
            lines.push(format!("    [{}] {}", page_info, excerpt));
        }
    }

    let s = &report.summary;
    if !quiet {
        lines.push(format!("\n{}", "\u{2500}".repeat(60)));
    }
    lines.push(format!(
        "Summary: {} PASS, {} FAIL, {} MANUAL, {} ERROR",
        s.pass, s.fail, s.manual, s.error,
    ));
    if !quiet {
        lines.push("=".repeat(60).to_string());
    }

    lines.join("\n")
}

pub fn format_json(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&serde_json::json!({
        "results": report.results.iter().map(|r| {
            serde_json::json!({
                "check_id": r.check_id,
                "status": r.status.as_str(),
                "detail": r.detail,
                "evidence": r.evidence.iter().map(|e| {
                    serde_json::json!({
                        "page": e.page,
                        "bbox": e.bbox,
                        "excerpt": e.excerpt,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "pass": report.summary.pass,
            "fail": report.summary.fail,
            "manual": report.summary.manual,
            "error": report.summary.error,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkers::{CheckResult, EvidenceItem};

    fn make_result(status: Status) -> CheckResult {
        CheckResult {
            check_id: "test_check".to_string(),
            status,
            evidence: vec![],
            detail: "test detail".to_string(),
        }
    }

    #[test]
    fn test_summary_counts() {
        let results = vec![
            make_result(Status::Pass),
            make_result(Status::Pass),
            make_result(Status::Fail),
            make_result(Status::Manual),
            make_result(Status::Error),
        ];
        let report = build_report(results);
        assert_eq!(report.summary.pass, 2);
        assert_eq!(report.summary.fail, 1);
        assert_eq!(report.summary.manual, 1);
        assert_eq!(report.summary.error, 1);
    }

    #[test]
    fn test_format_text_includes_statuses() {
        let results = vec![make_result(Status::Pass), make_result(Status::Fail)];
        let report = build_report(results);
        let output = format_text(&report);
        assert!(output.contains("[PASS]"));
        assert!(output.contains("[FAIL]"));
        assert!(output.contains("PASS, 1 FAIL"));
    }

    #[test]
    fn test_format_text_includes_evidence() {
        let results = vec![CheckResult {
            check_id: "test".to_string(),
            status: Status::Fail,
            evidence: vec![EvidenceItem {
                page: 5,
                bbox: Some((10.0, 22.0, 30.0, 44.0)),
                excerpt: Some("bad text".to_string()),
            }],
            detail: "found issue".to_string(),
        }];
        let report = build_report(results);
        let output = format_text(&report);
        assert!(output.contains("page 5"));
        assert!(output.contains("bad text"));
        assert!(output.contains("found issue"));
    }

    #[test]
    fn test_format_json_valid() {
        let results = vec![make_result(Status::Pass), make_result(Status::Fail)];
        let report = build_report(results);
        let json = format_json(&report).expect("JSON should be valid");
        assert!(json.contains("\"pass\": 1"));
        assert!(json.contains("\"fail\": 1"));
        assert!(json.contains("\"PASS\""));
        assert!(json.contains("\"FAIL\""));
    }

    #[test]
    fn test_empty_report() {
        let report = build_report(vec![]);
        assert_eq!(report.summary.pass, 0);
        assert_eq!(report.summary.fail, 0);
        assert_eq!(report.results.len(), 0);
        let output = format_text(&report);
        assert!(output.contains("0 PASS"));
    }
}
