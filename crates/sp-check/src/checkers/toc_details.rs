use crate::checkers::{CheckResult, Checker, EvidenceItem, Status};
use serde_yaml::Value;
use sp_extract::document::ParsedDocument as Document;
use std::collections::BTreeMap;

fn find_toc_page(doc: &Document) -> Option<&sp_extract::document::ParsedPage> {
    doc.pages.iter().find(|p| {
        let text: String = p
            .spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        text.to_lowercase().contains("table of contents")
    })
}

fn toc_lines(page: &sp_extract::document::ParsedPage) -> Vec<(f32, f32, f32, f32, String)> {
    let mut lines: BTreeMap<i32, Vec<&sp_extract::document::TextSpan>> = BTreeMap::new();
    for s in &page.spans {
        if !s.text.trim().is_empty() {
            let top_key = s.bbox.0.round() as i32;
            lines.entry(top_key).or_default().push(s);
        }
    }
    let mut result: Vec<(f32, f32, f32, f32, String)> = Vec::new();
    for spans in lines.values() {
        let mut sorted = spans.clone();
        sorted.sort_by(|a, b| {
            a.bbox
                .2
                .partial_cmp(&b.bbox.2)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let left = sorted
            .iter()
            .map(|s| s.bbox.2)
            .fold(f32::MAX, |a, b| a.min(b));
        let right = sorted
            .iter()
            .map(|s| s.bbox.3)
            .fold(f32::MIN, |a, b| a.max(b));
        let text: String = sorted
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let top = sorted[0].bbox.0;
        let bottom = sorted
            .iter()
            .map(|s| s.bbox.1)
            .fold(0.0f32, |a, b| a.max(b));
        result.push((top, bottom, left, right, text));
    }
    result
}

pub struct TocPageNumbersAlignedChecker;

impl Checker for TocPageNumbersAlignedChecker {
    fn category(&self) -> &'static str {
        "structure"
    }
    fn name(&self) -> &'static str {
        "toc_page_numbers_aligned"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let page = match find_toc_page(doc) {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "TOC page not found".to_string(),
                }
            }
        };

        let lines = toc_lines(page);
        let numbers: Vec<f32> = lines
            .iter()
            .filter_map(|(_top, _bottom, _left, right, text)| {
                let trimmed = text.trim();
                if trimmed
                    .chars()
                    .all(|c| c.is_ascii_digit() || c.is_whitespace())
                {
                    Some(*right)
                } else if let Some(last) = trimmed.split_whitespace().last() {
                    if last.chars().all(|c| c.is_ascii_digit()) && last.len() <= 4 {
                        Some(*right)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if numbers.len() < 3 {
            return CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "Not enough page numbers on TOC to check alignment".to_string(),
            };
        }

        let avg = numbers.iter().sum::<f32>() / numbers.len() as f32;
        let tolerance = 12.0;
        let mut violations: Vec<EvidenceItem> = Vec::new();

        for n in &numbers {
            if (*n - avg).abs() > tolerance {
                violations.push(EvidenceItem {
                    page: page.page_number,
                    bbox: None,
                    excerpt: Some(format!(
                        "Page number at x={:.0}pt deviates from average {:.0}pt",
                        n, avg
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
                    "TOC page numbers aligned (avg x={:.0}pt, {} entries)",
                    avg,
                    numbers.len()
                ),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!("{} TOC page number(s) misaligned", violations.len()),
                evidence: violations,
            }
        }
    }
}

pub struct TocNoOverhangChecker;

impl Checker for TocNoOverhangChecker {
    fn category(&self) -> &'static str {
        "structure"
    }
    fn name(&self) -> &'static str {
        "toc_no_overhang"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let page = match find_toc_page(doc) {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "TOC page not found".to_string(),
                }
            }
        };

        let lines = toc_lines(page);
        let _rightmost_x = lines
            .iter()
            .map(|(_, _, _, right, _)| *right)
            .fold(f32::MIN, |a, b| a.max(b));

        let number_x = lines
            .iter()
            .filter_map(|(_, _, _, right, text)| {
                let trimmed = text.trim();
                let has_num = trimmed
                    .split_whitespace()
                    .last()
                    .is_some_and(|w| w.chars().all(|c| c.is_ascii_digit()) && w.len() <= 4);
                if has_num {
                    Some(*right)
                } else {
                    None
                }
            })
            .fold(0.0f32, |a, b| a.max(b));

        if number_x < 100.0 {
            return CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "No page numbers found on TOC".to_string(),
            };
        }

        let tolerance = 12.0;
        let mut violations: Vec<EvidenceItem> = Vec::new();

        for (top, _bottom, _left, right, text) in &lines {
            if *right > number_x + tolerance {
                let trimmed = text.trim();
                let is_page_num = trimmed
                    .chars()
                    .all(|c| c.is_ascii_digit() || c.is_whitespace());
                if !is_page_num {
                    violations.push(EvidenceItem {
                        page: page.page_number,
                        bbox: Some((*top, *top + 12.0, *right - 50.0, *right)),
                        excerpt: Some(format!(
                            "Overhang: \"{}\" extends past page number column",
                            trimmed
                        )),
                    });
                }
            }
        }

        if violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "No TOC entries overhang the page number column".to_string(),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "{} TOC entry/entries overhang the page number column",
                    violations.len()
                ),
                evidence: violations,
            }
        }
    }
}

pub struct TocCvNoDotsChecker;

impl Checker for TocCvNoDotsChecker {
    fn category(&self) -> &'static str {
        "structure"
    }
    fn name(&self) -> &'static str {
        "toc_cv_no_dots"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let page = match find_toc_page(doc) {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "TOC page not found".to_string(),
                }
            }
        };

        let lines = toc_lines(page);
        let cv_line = lines.iter().find(|(_, _, _, _, text)| {
            let low = text.to_lowercase();
            low.contains("curriculum vitae")
                || (low.contains("curriculum") && low.contains("vitae"))
        });

        match cv_line {
            Some((top, bottom, left, right, text)) => {
                let has_dots = text.contains('.');
                let has_number = text
                    .split_whitespace()
                    .last()
                    .is_some_and(|w| w.chars().all(|c| c.is_ascii_digit()));
                let mut vigns: Vec<EvidenceItem> = Vec::new();
                if has_dots {
                    vigns.push(EvidenceItem {
                        page: page.page_number,
                        bbox: Some((*top, *bottom, *left, *right)),
                        excerpt: Some("CV entry has leader dots".to_string()),
                    });
                }
                if has_number {
                    vigns.push(EvidenceItem {
                        page: page.page_number,
                        bbox: Some((*top, *bottom, *left, *right)),
                        excerpt: Some("CV entry has page number".to_string()),
                    });
                }
                if vigns.is_empty() {
                    CheckResult {
                        check_id: String::new(),
                        status: Status::Pass,
                        evidence: vec![],
                        detail: "CV entry in TOC has no dots or page number".to_string(),
                    }
                } else {
                    CheckResult {
                        check_id: String::new(),
                        status: Status::Fail,
                        detail: "CV entry in TOC has leader dots or page number".to_string(),
                        evidence: vigns,
                    }
                }
            }
            None => CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "No CV entry found in TOC".to_string(),
            },
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

    fn make_toc_page(spans: Vec<sp_extract::document::TextSpan>) -> Document {
        Document {
            raw_text: String::new(),
            paragraphs: vec![],
            headings: vec![],
            metadata: sp_extract::document::ParsedMetadata {
                title: None,
                author: None,
                page_count: 1,
                page_count_estimated: false,
                detected_fonts: vec![],
            },
            pages: vec![Page {
                text: String::new(),
                page_number: 5,
                width: 612.0,
                height: 792.0,
                spans,
                images: vec![],
                paths: vec![],
            }],
        }
    }

    #[test]
    fn test_toc_page_numbers_aligned_pass() {
        let doc = make_toc_page(vec![
            span_x("TABLE OF CONTENTS", 72.0, 100.0, 250.0),
            span_x("Chapter 1: Introduction", 150.0, 100.0, 300.0),
            span_x("1", 150.0, 530.0, 540.0),
            span_x("Chapter 2: Methods", 174.0, 100.0, 280.0),
            span_x("15", 174.0, 520.0, 540.0),
            span_x("Chapter 3: Results", 198.0, 100.0, 280.0),
            span_x("42", 198.0, 530.0, 545.0),
        ]);
        let r = TocPageNumbersAlignedChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_toc_no_overhang_pass() {
        let doc = make_toc_page(vec![
            span_x("TABLE OF CONTENTS", 72.0, 100.0, 250.0),
            span_x("Chapter 1: Introduction", 150.0, 100.0, 300.0),
            span_x("1", 150.0, 530.0, 540.0),
        ]);
        let r = TocNoOverhangChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_toc_cv_no_dots_pass() {
        let doc = make_toc_page(vec![
            span_x("table of contents entry", 100.0, 150.0, 300.0),
            span_x("Curriculum Vitae", 500.0, 150.0, 300.0),
        ]);
        let r = TocCvNoDotsChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_toc_cv_no_dots_fail() {
        let doc = make_toc_page(vec![
            span_x("table of contents entry", 100.0, 150.0, 300.0),
            span_x("Curriculum Vitae", 500.0, 150.0, 300.0),
            span_x(".....", 500.0, 310.0, 520.0),
            span_x("250", 500.0, 530.0, 550.0),
        ]);
        let r = TocCvNoDotsChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail);
    }
}
