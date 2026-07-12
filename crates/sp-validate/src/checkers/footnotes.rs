use crate::checkers::typography::normalize_family;
use crate::checkers::{CheckResult, Checker, EvidenceItem, Status};
use crate::document::Document;
use serde_yaml::Value;
use std::collections::HashMap;

fn compute_body_style(doc: &Document) -> (String, f32) {
    let start = 6usize.min(doc.pages.len().saturating_sub(1));
    let end = (start + 10).min(doc.pages.len());
    let mut families: HashMap<String, usize> = HashMap::new();
    let mut sizes: HashMap<i32, usize> = HashMap::new();
    for page in &doc.pages[start..end] {
        for s in &page.spans {
            let (top, bottom, _x0, _x1) = s.bbox;
            if bottom >= page.height - 53.0 || top < 72.0 {
                continue;
            }
            if s.text.trim().len() < 3 {
                continue;
            }
            *families.entry(normalize_family(&s.font_name)).or_insert(0) += 1;
            *sizes
                .entry((s.font_size * 10.0).round() as i32)
                .or_insert(0) += 1;
        }
    }
    let family = families
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(k, _)| k.clone())
        .unwrap_or_default();
    let size_key = sizes
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(k, _)| *k)
        .unwrap_or(0);
    (family, size_key as f32 / 10.0)
}

pub struct FootnotesFontChecker;

impl Checker for FootnotesFontChecker {
    fn category(&self) -> &'static str {
        "typography"
    }
    fn name(&self) -> &'static str {
        "footnotes_font_consistent"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let (body_family, body_size) = compute_body_style(doc);
        let mut violations: Vec<EvidenceItem> = Vec::new();

        for page in &doc.pages {
            if page.spans.is_empty() {
                continue;
            }
            let body_bottom = page
                .spans
                .iter()
                .filter(|s| {
                    let (top, bottom, _x0, _x1) = s.bbox;
                    top >= 72.0
                        && bottom <= page.height - 72.0
                        && (s.font_size - body_size).abs() <= 1.0
                        && s.text.trim().len() > 3
                })
                .map(|s| s.bbox.1)
                .fold(0.0f32, |a, b| a.max(b));

            if body_bottom < 100.0 || body_bottom > page.height - 50.0 {
                continue;
            }

            let has_separator_line = page.paths.iter().any(|(_top, _bottom, x0, x1)| {
                let width = x1 - x0;
                width > 18.0 && width < 180.0
            });

            let footnote_detected = has_separator_line
                || page.spans.iter().any(|s| {
                    let (top, _bottom, _x0, _x1) = s.bbox;
                    let t = s.text.trim();
                    top > body_bottom + 2.0
                        && s.font_size < body_size - 1.5
                        && s.font_size >= 8.0
                        && (t.starts_with(|c: char| c.is_ascii_digit())
                            || t.starts_with('*')
                            || t.contains("footnote")
                            || t.contains("note"))
                });

            if !footnote_detected {
                continue;
            }

            let footnote_spans: Vec<&crate::document::TextSpan> = page
                .spans
                .iter()
                .filter(|s| {
                    let (top, bottom, _x0, _x1) = s.bbox;
                    !s.text.trim().is_empty()
                        && bottom < page.height - 50.0
                        && top > body_bottom + 2.0
                        && s.font_size < body_size - 1.5
                        && s.font_size >= 8.0
                })
                .collect();

            for s in &footnote_spans {
                let fam_norm = normalize_family(&s.font_name);
                if fam_norm != body_family {
                    violations.push(EvidenceItem {
                        page: page.page_number,
                        bbox: Some(s.bbox),
                        excerpt: Some(format!(
                            "Footnote font '{}' ≠ body '{}' ({}, {:.0}pt, expected {} {:.0}pt)",
                            s.font_name, body_family, s.text, s.font_size, body_family, body_size
                        )),
                    });
                } else if s.font_size < 9.5 {
                    violations.push(EvidenceItem {
                        page: page.page_number,
                        bbox: Some(s.bbox),
                        excerpt: Some(format!(
                            "Footnote too small ({:.1}pt): \"{}\" (minimum 10pt)",
                            s.font_size, s.text
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
                detail: format!(
                    "Footnote text consistent with body font ({} {:.0}pt)",
                    body_family, body_size
                ),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!("{} footnote font issue(s)", violations.len()),
                evidence: violations,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, Page};

    fn span(text: &str, top: f32, font_size: f32, font_name: &str) -> crate::document::TextSpan {
        crate::document::TextSpan {
            text: text.to_string(),
            font_name: font_name.to_string(),
            font_size,
            bbox: (top, top + font_size, 100.0, 200.0),
            is_bold: false,
            is_italic: false,
            color: None,
        }
    }

    fn body_page(pn: usize) -> Page {
        let mut spans = Vec::new();
        for i in 0..25 {
            spans.push(span(
                "body text line",
                72.0 + i as f32 * 24.0,
                12.0,
                "TimesNewRoman",
            ));
        }
        Page {
            page_number: pn,
            width: 612.0,
            height: 792.0,
            spans,
            images: vec![],
            paths: vec![],
        }
    }

    #[test]
    fn test_footnotes_pass_no_footnotes() {
        let mut pages = Vec::new();
        for i in 1..12 {
            pages.push(body_page(i));
        }
        let doc = Document { pages };
        let r = FootnotesFontChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_footnotes_pass_good_footnote() {
        let mut pages: Vec<Page> = Vec::new();
        for i in 1..11 {
            pages.push(body_page(i));
        }
        let mut footnote_page = body_page(11);
        let body_bottom = 72.0 + 24.0 * 24.0 + 12.0; // last body line bottom
        footnote_page.spans.push(span(
            "1. See Smith (2020)",
            body_bottom + 8.0,
            10.0,
            "TimesNewRoman",
        ));
        pages.push(footnote_page);
        let doc = Document { pages };
        let r = FootnotesFontChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass, "{}", r.detail);
    }

    #[test]
    fn test_footnotes_fail_wrong_font() {
        let mut pages: Vec<Page> = Vec::new();
        for i in 1..11 {
            pages.push(body_page(i));
        }
        let mut footnote_page = body_page(11);
        let body_bottom = 72.0 + 24.0 * 24.0 + 12.0;
        footnote_page.spans.push(span(
            "1. See Smith (2020)",
            body_bottom + 8.0,
            10.0,
            "Arial",
        ));
        pages.push(footnote_page);
        let doc = Document { pages };
        let r = FootnotesFontChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail, "{}", r.detail);
    }

    #[test]
    fn test_footnotes_fail_too_small() {
        let mut pages: Vec<Page> = Vec::new();
        for i in 1..11 {
            pages.push(body_page(i));
        }
        let mut footnote_page = body_page(11);
        let body_bottom = 72.0 + 24.0 * 24.0 + 12.0;
        footnote_page.spans.push(span(
            "1. tiny text",
            body_bottom + 8.0,
            8.0,
            "TimesNewRoman",
        ));
        pages.push(footnote_page);
        let doc = Document { pages };
        let r = FootnotesFontChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail, "{}", r.detail);
    }
}
