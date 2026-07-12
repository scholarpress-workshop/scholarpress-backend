use crate::checkers::{CheckResult, Checker, EvidenceItem, Status};
use crate::document::Document;
use serde_yaml::Value;

pub struct TitlePageAllCapsChecker;

impl Checker for TitlePageAllCapsChecker {
    fn category(&self) -> &'static str {
        "typography"
    }

    fn name(&self) -> &'static str {
        "title_page_all_caps"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let page = match doc.pages.first() {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "Document has no pages".to_string(),
                }
            }
        };

        let non_empty_spans: Vec<&crate::document::TextSpan> = page
            .spans
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .collect();

        if non_empty_spans.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "No text found on title page".to_string(),
            };
        }

        let min_top = non_empty_spans
            .iter()
            .map(|s| s.bbox.0)
            .fold(f32::MAX, |a, b| a.min(b));

        let title_spans: Vec<&&crate::document::TextSpan> = non_empty_spans
            .iter()
            .filter(|s| (s.bbox.0 - min_top).abs() < 3.0)
            .collect();

        let title_text: String = title_spans
            .iter()
            .map(|s| s.text.trim())
            .collect::<Vec<_>>()
            .join(" ");

        if title_text.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "No title text found on title page".to_string(),
            };
        }

        let alpha_chars: Vec<char> = title_text.chars().filter(|c| c.is_alphabetic()).collect();

        if alpha_chars.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "No alphabetic characters in title".to_string(),
            };
        }

        let all_upper = alpha_chars.iter().all(|c| c.is_uppercase());
        let upper_ratio = alpha_chars.iter().filter(|c| c.is_uppercase()).count() as f32
            / alpha_chars.len() as f32;

        if all_upper {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!("Title is all uppercase: \"{}\"", title_text),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "Title must be all caps ({:.0}% uppercase): \"{}\"",
                    upper_ratio * 100.0,
                    title_text
                ),
                evidence: title_spans
                    .iter()
                    .map(|s| EvidenceItem {
                        page: 1,
                        bbox: Some(s.bbox),
                        excerpt: Some(s.text.clone()),
                    })
                    .collect(),
            }
        }
    }
}

const CLAUSE_KEYWORDS: &[&str] = &[
    "submitted",
    "faculty",
    "partial",
    "fulfillment",
    "degree",
    "department",
    "indiana university",
];

const COMMITTEE_KEYWORDS: &[&str] = &["committee", "chair", "doctoral"];

fn is_clause_text(text: &str) -> bool {
    let low = text.to_lowercase();
    CLAUSE_KEYWORDS.iter().any(|k| low.contains(k))
}

fn is_committee_text(text: &str) -> bool {
    let low = text.to_lowercase();
    COMMITTEE_KEYWORDS.iter().any(|k| low.contains(k))
}

pub struct TitlePageClauseCenteredChecker;

impl Checker for TitlePageClauseCenteredChecker {
    fn category(&self) -> &'static str {
        "typography"
    }

    fn name(&self) -> &'static str {
        "title_page_clause_centered"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let page = match doc.pages.first() {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "Document has no pages".to_string(),
                }
            }
        };

        let non_empty: Vec<&crate::document::TextSpan> = page
            .spans
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .collect();

        if non_empty.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "No text found on title page".to_string(),
            };
        }

        let mut line_groups: std::collections::BTreeMap<i32, Vec<&&crate::document::TextSpan>> =
            std::collections::BTreeMap::new();
        for s in &non_empty {
            let top_key = s.bbox.0.round() as i32;
            line_groups.entry(top_key).or_default().push(s);
        }

        let mut lines: Vec<(i32, Vec<&&crate::document::TextSpan>)> =
            line_groups.into_iter().collect();
        lines.sort_by_key(|(top, _)| *top);

        if lines.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "No text lines on title page".to_string(),
            };
        }

        let title_top = lines[0].0;

        let page_center = page.width / 2.0;
        let tolerance = 36.0; // 0.5in tolerance for centering

        let mut clause_found = false;
        let mut violations: Vec<EvidenceItem> = Vec::new();
        let mut centered_count = 0usize;
        let mut checked_count = 0usize;

        for (top, spans) in &lines {
            if *top == title_top {
                continue;
            }

            let line_text: String = spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let low = line_text.to_lowercase();

            if is_committee_text(&low) {
                break;
            }

            if !clause_found && !is_clause_text(&low) {
                continue;
            }

            clause_found = true;
            checked_count += 1;

            let line_left = spans
                .iter()
                .map(|s| s.bbox.2)
                .fold(f32::MAX, |a, b| a.min(b));
            let line_right = spans
                .iter()
                .map(|s| s.bbox.3)
                .fold(f32::MIN, |a, b| a.max(b));
            let line_center = (line_left + line_right) / 2.0;
            let offset = (line_center - page_center).abs();

            if offset <= tolerance {
                centered_count += 1;
            } else {
                violations.push(EvidenceItem {
                    page: 1,
                    bbox: Some((*top as f32, *top as f32 + 12.0, line_left, line_right)),
                    excerpt: Some(format!(
                        "Off-center by {:.0}pt: \"{}\"",
                        offset,
                        if line_text.len() > 60 {
                            &line_text[..60]
                        } else {
                            &line_text
                        },
                    )),
                });
            }
        }

        if !clause_found {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "Clause text not found on title page".to_string(),
            };
        }

        if violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!(
                    "Clause is centered ({}/{} lines within {:.0}pt tolerance)",
                    centered_count, checked_count, tolerance
                ),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "{}/{} clause lines not centered",
                    violations.len(),
                    checked_count
                ),
                evidence: violations,
            }
        }
    }
}

fn compute_body_line_spacing(doc: &Document) -> Option<f32> {
    let mut gaps: Vec<f32> = Vec::new();
    let start_page = 6usize.min(doc.pages.len().saturating_sub(1));
    let end_page = (start_page + 10).min(doc.pages.len());
    for page in &doc.pages[start_page..end_page] {
        let mut tops: Vec<i32> = page
            .spans
            .iter()
            .filter(|s| {
                let (top, bottom, _x0, _x1) = s.bbox;
                !s.text.trim().is_empty() && top >= 72.0 && bottom <= page.height - 72.0
            })
            .map(|s| s.bbox.0.round() as i32)
            .collect();
        tops.sort();
        tops.dedup();
        for w in tops.windows(2) {
            let gap = (w[1] - w[0]) as f32;
            if gap > 2.0 && gap < 40.0 {
                gaps.push(gap);
            }
        }
    }
    if gaps.is_empty() {
        return None;
    }
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(gaps[gaps.len() / 2])
}

pub struct TitlePageClauseSpacingChecker;

impl Checker for TitlePageClauseSpacingChecker {
    fn category(&self) -> &'static str {
        "typography"
    }

    fn name(&self) -> &'static str {
        "title_page_clause_spacing"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let page = match doc.pages.first() {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "Document has no pages".to_string(),
                }
            }
        };

        let body_spacing = compute_body_line_spacing(doc).unwrap_or(24.0);

        let non_empty: Vec<&crate::document::TextSpan> = page
            .spans
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .collect();

        let mut line_groups: std::collections::BTreeMap<i32, Vec<&&crate::document::TextSpan>> =
            std::collections::BTreeMap::new();
        for s in &non_empty {
            let top_key = s.bbox.0.round() as i32;
            line_groups.entry(top_key).or_default().push(s);
        }
        let mut lines: Vec<(i32, Vec<&&crate::document::TextSpan>)> =
            line_groups.into_iter().collect();
        lines.sort_by_key(|(top, _)| *top);

        if lines.len() < 2 {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "Not enough lines on title page".to_string(),
            };
        }

        let title_top = lines[0].0;
        let mut clause_tops: Vec<i32> = Vec::new();
        let mut clause_found = false;

        for (top, spans) in &lines {
            if *top == title_top {
                continue;
            }
            let line_text: String = spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let low = line_text.to_lowercase();
            if is_committee_text(&low) {
                break;
            }
            if !clause_found && !is_clause_text(&low) {
                continue;
            }
            clause_found = true;
            clause_tops.push(*top);
        }

        if !clause_found || clause_tops.len() < 2 {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "Not enough clause lines on title page".to_string(),
            };
        }

        let gaps: Vec<f32> = clause_tops
            .windows(2)
            .map(|w| (w[1] - w[0]) as f32)
            .collect();
        if gaps.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "Single clause line — spacing check not applicable".to_string(),
            };
        }

        let avg_gap = gaps.iter().sum::<f32>() / gaps.len() as f32;

        let is_single = (12.0..=18.0).contains(&avg_gap);
        let matches_body = (avg_gap - body_spacing).abs() <= 4.0;

        if is_single || matches_body {
            let label = if is_single {
                "single-spaced"
            } else {
                "matches body"
            };
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!(
                    "Clause line spacing consistent ({:.0}pt average, {})",
                    avg_gap, label
                ),
            }
        } else {
            let mut violations: Vec<EvidenceItem> = Vec::new();
            for (i, gap) in gaps.iter().enumerate() {
                let single_ok = *gap >= 12.0 && *gap <= 18.0;
                let body_ok = (*gap - body_spacing).abs() <= 4.0;
                if !single_ok && !body_ok {
                    violations.push(EvidenceItem {
                        page: 1,
                        bbox: Some((clause_tops[i] as f32, clause_tops[i+1] as f32, 0.0, 0.0)),
                        excerpt: Some(format!("Line gap {:.0}pt — not single-spaced (12-18pt) nor body spacing ({:.0}pt)", gap, body_spacing)),
                    });
                }
            }
            if violations.is_empty() || violations.len() == 1 && gaps.len() > 4 {
                CheckResult {
                    check_id: String::new(),
                    status: Status::Pass,
                    evidence: vec![],
                    detail: format!("Clause line spacing acceptable ({:.0}pt average)", avg_gap),
                }
            } else {
                CheckResult { check_id: String::new(), status: Status::Fail,
                    detail: format!("{}/{} clause line gaps not single-spaced (12-18pt) nor matching body ({:.0}pt)", violations.len(), gaps.len(), body_spacing),
                    evidence: violations }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, Page};

    fn span(text: &str, top: f32) -> crate::document::TextSpan {
        span_x(text, top, 100.0, 200.0)
    }

    fn span_x(text: &str, top: f32, x0: f32, x1: f32) -> crate::document::TextSpan {
        crate::document::TextSpan {
            text: text.to_string(),
            font_name: "Times".to_string(),
            font_size: 12.0,
            bbox: (top, top + 12.0, x0, x1),
            is_bold: false,
            is_italic: false,
            color: None,
        }
    }

    #[test]
    fn test_title_all_caps_pass() {
        let doc = Document {
            pages: vec![Page {
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![span("POWER AND FREEDOM", 200.0), span("Jane Smith", 250.0)],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = TitlePageAllCapsChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_title_all_caps_fail() {
        let doc = Document {
            pages: vec![Page {
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![span("Power and Freedom", 200.0), span("Jane Smith", 250.0)],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = TitlePageAllCapsChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn test_title_all_caps_fail_mixed() {
        let doc = Document {
            pages: vec![Page {
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![span("POWER AND freedom", 200.0)],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = TitlePageAllCapsChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn test_title_all_caps_no_alpha() {
        let doc = Document {
            pages: vec![Page {
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![span("2025", 200.0)],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = TitlePageAllCapsChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_title_empty_doc() {
        let doc = Document { pages: vec![] };
        let r = TitlePageAllCapsChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Error);
    }

    #[test]
    fn test_clause_centered_pass() {
        let center = 306.0; // 612/2 = page center
        let doc = Document {
            pages: vec![Page {
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![
                    span_x("TITLE", 200.0, 250.0, 362.0),
                    span_x("Jane Smith", 250.0, 256.0, 356.0),
                    span_x(
                        "Submitted to the faculty",
                        320.0,
                        center - 100.0,
                        center + 100.0,
                    ),
                    span_x(
                        "in partial fulfillment",
                        334.0,
                        center - 90.0,
                        center + 90.0,
                    ),
                    span_x("for the degree", 348.0, center - 70.0, center + 70.0),
                    span_x("Indiana University", 376.0, center - 80.0, center + 80.0),
                    span_x("Dr. Chair, Committee", 450.0, 100.0, 400.0),
                ],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = TitlePageClauseCenteredChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_clause_centered_fail_offcenter() {
        let doc = Document {
            pages: vec![Page {
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![
                    span_x("TITLE", 200.0, 100.0, 300.0),
                    span_x("Submitted to the faculty", 320.0, 100.0, 300.0),
                    span_x("in partial fulfillment", 334.0, 100.0, 300.0),
                    span_x("Indiana University", 376.0, 100.0, 300.0),
                ],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = TitlePageClauseCenteredChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn test_clause_centered_no_clause_found() {
        let doc = Document {
            pages: vec![Page {
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![
                    span_x("TITLE", 200.0, 100.0, 300.0),
                    span_x("Jane Smith", 250.0, 100.0, 300.0),
                ],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = TitlePageClauseCenteredChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Error);
    }

    #[test]
    fn test_clause_centered_stops_at_committee() {
        let center = 306.0;
        let doc = Document {
            pages: vec![Page {
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![
                    span_x("TITLE", 200.0, 250.0, 362.0),
                    span_x(
                        "Submitted to the faculty",
                        320.0,
                        center - 100.0,
                        center + 100.0,
                    ),
                    span_x("Indiana University", 376.0, center - 80.0, center + 80.0),
                    span_x("Doctoral Committee:", 450.0, 100.0, 400.0),
                    span_x("Not part of clause", 464.0, 100.0, 200.0),
                ],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = TitlePageClauseCenteredChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    fn body_page(page_num: usize) -> Page {
        let mut spans = Vec::new();
        for i in 0..25 {
            let top = 72.0 + i as f32 * 24.0;
            spans.push(span_x("body text line here", top, 90.0, 522.0));
        }
        Page {
            page_number: page_num,
            width: 612.0,
            height: 792.0,
            spans,
            images: vec![],
            paths: vec![],
        }
    }

    #[test]
    fn test_clause_spacing_pass() {
        let title_page = Page {
            page_number: 1,
            width: 612.0,
            height: 792.0,
            spans: vec![
                span_x("TITLE", 200.0, 100.0, 300.0),
                span_x("Submitted to the faculty", 320.0, 156.0, 456.0),
                span_x("in partial fulfillment", 334.0, 156.0, 456.0),
                span_x("for the degree", 348.0, 156.0, 456.0),
                span_x("Indiana University", 362.0, 156.0, 456.0),
            ],
            images: vec![],
            paths: vec![],
        };
        let mut pages = vec![title_page];
        for i in 2..12 {
            pages.push(body_page(i));
        }
        let doc = Document { pages };
        let r = TitlePageClauseSpacingChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass, "{}", r.detail);
    }

    #[test]
    fn test_clause_spacing_fail() {
        let title_page = Page {
            page_number: 1,
            width: 612.0,
            height: 792.0,
            spans: vec![
                span_x("TITLE", 200.0, 100.0, 300.0),
                span_x("Submitted to the faculty", 320.0, 156.0, 456.0),
                span_x("in partial fulfillment", 350.0, 156.0, 456.0), // 30pt gap — neither single nor 24pt body
                span_x("for the degree", 380.0, 156.0, 456.0),         // 30pt gap
                span_x("Indiana University", 428.0, 156.0, 456.0),     // 48pt gap
            ],
            images: vec![],
            paths: vec![],
        };
        let mut pages = vec![title_page];
        for i in 2..12 {
            pages.push(body_page(i));
        }
        let doc = Document { pages };
        let r = TitlePageClauseSpacingChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail);
    }
}
