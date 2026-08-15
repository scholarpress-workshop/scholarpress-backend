use pdf_oxide::geometry::Rect;
use sp_check::checkers::Status;
use sp_check::report::Report;

/// Convert a top-left-origin bbox `(top, bottom, x0, x1)` to a PDF-space
/// `Rect` (bottom-left origin) given the page height in points.
pub(crate) fn bbox_to_rect(bbox: (f32, f32, f32, f32), page_height: f32) -> Rect {
    let (top, bottom, x0, x1) = bbox;
    Rect::new(x0, page_height - bottom, x1 - x0, bottom - top)
}

/// Build the sticky-note body for an in-place finding.
pub(crate) fn note_contents(
    check_id: &str,
    status: &str,
    detail: &str,
    excerpt: Option<&str>,
) -> String {
    let mut s = format!("[{check_id}] {status}\n{detail}");
    if let Some(ex) = excerpt {
        s.push_str(&format!("\n\u{201c}{ex}\u{201d}"));
    }
    s
}

/// Build the ASCII text rendered on the prepended summary page:
/// counts plus document-level (non-bbox) findings only.
pub(crate) fn summary_text(report: &Report) -> String {
    let mut lines = vec![
        "ScholarPress Format Check - Annotated Summary".to_string(),
        "=".repeat(60),
        format!(
            "Pass: {}  Fail: {}  Manual: {}  Error: {}",
            report.summary.pass, report.summary.fail, report.summary.manual, report.summary.error
        ),
        String::new(),
        "Document-level findings".to_string(),
        "-".repeat(60),
    ];

    let mut count = 0;
    for r in &report.results {
        if r.status == Status::Pass {
            continue;
        }
        let doc_evidence: Vec<_> = r.evidence.iter().filter(|e| e.bbox.is_none()).collect();
        if doc_evidence.is_empty() {
            continue;
        }
        count += 1;
        lines.push(format!("[{}] {}", r.status.as_str(), r.check_id));
        if !r.detail.is_empty() {
            lines.push(format!("    {}", r.detail));
        }
        for e in &doc_evidence {
            let ex = e.excerpt.as_deref().unwrap_or("");
            lines.push(format!("    page {}  {}", e.page, ex));
        }
    }
    if count == 0 {
        lines.push("(none - all findings are highlighted in place)".to_string());
    }
    lines.push(String::new());
    lines.push("In-place findings are highlighted and annotated on their pages.".to_string());
    lines.join("\n")
}

// Task 3 fills this in; stub so the crate compiles now.
pub(crate) fn annotate_bytes(_input: &[u8], _report: &Report) -> Result<Vec<u8>, crate::AnnotateError> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_check::checkers::{CheckResult, EvidenceItem, Status};
    use sp_check::report::build_report;

    fn sample_report() -> Report {
        build_report(vec![
            CheckResult {
                check_id: "test_inplace".to_string(),
                status: Status::Fail,
                detail: "margins too small".to_string(),
                evidence: vec![EvidenceItem {
                    page: 1,
                    bbox: Some((100.0, 120.0, 200.0, 300.0)),
                    excerpt: Some("1 inch".to_string()),
                }],
            },
            CheckResult {
                check_id: "test_global".to_string(),
                status: Status::Fail,
                detail: "missing section".to_string(),
                evidence: vec![EvidenceItem {
                    page: 3,
                    bbox: None,
                    excerpt: Some("Abstract".to_string()),
                }],
            },
            CheckResult {
                check_id: "test_pass".to_string(),
                status: Status::Pass,
                detail: "ok".to_string(),
                evidence: vec![],
            },
        ])
    }

    #[test]
    fn bbox_to_rect_flips_y() {
        let r = bbox_to_rect((100.0, 120.0, 200.0, 300.0), 792.0);
        assert_eq!((r.x, r.y, r.width, r.height), (200.0, 672.0, 100.0, 20.0));
    }

    #[test]
    fn note_contents_includes_fields() {
        let s = note_contents("global_margins", "FAIL", "margins too small", Some("1 inch"));
        assert!(s.contains("[global_margins] FAIL"));
        assert!(s.contains("margins too small"));
        assert!(s.contains("1 inch"));
    }

    #[test]
    fn summary_text_lists_document_level_only() {
        let text = summary_text(&sample_report());
        assert!(text.contains("test_global"));
        assert!(text.contains("missing section"));
        assert!(text.contains("Fail: 2"));
        assert!(text.contains("Pass: 1"));
        assert!(!text.contains("test_inplace"), "in-place finding must not be duplicated");
    }
}
