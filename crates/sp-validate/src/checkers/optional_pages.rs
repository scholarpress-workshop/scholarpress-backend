use crate::checkers::{CheckResult, Checker, EvidenceItem, Status};
use sp_extract::document::ParsedDocument as Document;
use serde_yaml::Value;

pub struct CopyrightPageFormatChecker;

impl Checker for CopyrightPageFormatChecker {
    fn category(&self) -> &'static str {
        "content"
    }
    fn name(&self) -> &'static str {
        "copyright_page_format"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let copyright_page = doc.pages.iter().find(|p| {
            p.spans
                .iter()
                .any(|s| s.text.contains('©') || s.text.to_lowercase().contains("copyright"))
        });

        let page = match copyright_page {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Pass,
                    evidence: vec![],
                    detail: "No copyright page detected (optional)".to_string(),
                }
            }
        };

        let page_center = page.width / 2.0;
        let tolerance = 36.0;

        let mut violations: Vec<EvidenceItem> = Vec::new();

        let has_copyright_symbol = page.spans.iter().any(|s| s.text.contains('©'));
        let full_text: String = page
            .spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let has_year = full_text
            .split_whitespace()
            .any(|w| w.len() == 4 && w.chars().all(|c| c.is_ascii_digit()));

        if !has_copyright_symbol || !has_year {
            violations.push(EvidenceItem {
                page: page.page_number,
                bbox: None,
                excerpt: Some(
                    "Copyright symbol (©) or year not found on copyright page".to_string(),
                ),
            });
        }

        let non_empty: Vec<&sp_extract::document::TextSpan> = page
            .spans
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .collect();

        let mut line_groups: std::collections::BTreeMap<i32, Vec<&&sp_extract::document::TextSpan>> =
            std::collections::BTreeMap::new();
        for s in &non_empty {
            let top_key = s.bbox.0.round() as i32;
            line_groups.entry(top_key).or_default().push(s);
        }
        let mut lines: Vec<(i32, Vec<&&sp_extract::document::TextSpan>)> =
            line_groups.into_iter().collect();
        lines.sort_by_key(|(top, _)| *top);

        let mut off_center_count = 0usize;
        for (top, spans) in &lines {
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

            if offset > tolerance {
                off_center_count += 1;
                let text: String = spans
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                violations.push(EvidenceItem {
                    page: page.page_number,
                    bbox: Some((*top as f32, *top as f32 + 12.0, line_left, line_right)),
                    excerpt: Some(format!(
                        "Off-center by {:.0}pt: \"{}\"",
                        offset,
                        if text.chars().count() > 50 {
                            text.chars().take(50).collect::<String>()
                        } else {
                            text
                        }
                    )),
                });
            }
        }

        if violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!(
                    "Copyright page format correct ({} lines centered)",
                    lines.len()
                ),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "{}/{} lines not centered on copyright page",
                    off_center_count,
                    lines.len()
                ),
                evidence: violations,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_extract::document::{ParsedDocument as Document, ParsedPage as Page};

    fn span_x(text: &str, top: f32, x0: f32, x1: f32) -> sp_extract::document::TextSpan {
        sp_extract::document::TextSpan {
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
    fn test_no_copyright_page_pass() {
        let doc = Document { raw_text: String::new(), paragraphs: vec![], headings: vec![], metadata: sp_extract::document::ParsedMetadata { title: None, author: None, page_count: 1, page_count_estimated: false, detected_fonts: vec![] },
            pages: vec![Page { text: String::new(),
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![span_x("Title", 200.0, 100.0, 200.0)],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = CopyrightPageFormatChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_copyright_page_centered_pass() {
        let center = 306.0;
        let doc = Document { raw_text: String::new(), paragraphs: vec![], headings: vec![], metadata: sp_extract::document::ParsedMetadata { title: None, author: None, page_count: 1, page_count_estimated: false, detected_fonts: vec![] },
            pages: vec![Page { text: String::new(),
                page_number: 3,
                width: 612.0,
                height: 792.0,
                spans: vec![
                    span_x("© 2025", 300.0, center - 40.0, center + 40.0),
                    span_x("Jane Smith", 380.0, center - 45.0, center + 45.0),
                ],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = CopyrightPageFormatChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_copyright_page_offcenter_fail() {
        let doc = Document { raw_text: String::new(), paragraphs: vec![], headings: vec![], metadata: sp_extract::document::ParsedMetadata { title: None, author: None, page_count: 1, page_count_estimated: false, detected_fonts: vec![] },
            pages: vec![Page { text: String::new(),
                page_number: 3,
                width: 612.0,
                height: 792.0,
                spans: vec![span_x("© 2025", 300.0, 80.0, 220.0)],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = CopyrightPageFormatChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail);
    }
}
