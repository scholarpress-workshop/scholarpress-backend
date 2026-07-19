use crate::checkers::{CheckResult, Checker, EvidenceItem, Status};
use serde_yaml::Value;
use sp_extract::document::ParsedDocument as Document;

fn parse_measurement(value: &str) -> Result<f32, String> {
    let value = value.trim();
    if let Some(inches) = value.strip_suffix("in") {
        inches
            .trim()
            .parse::<f32>()
            .map(|v| v * 72.0)
            .map_err(|e| format!("Invalid inches: {}", e))
    } else if let Some(pts) = value.strip_suffix("pt") {
        pts.trim()
            .parse::<f32>()
            .map_err(|e| format!("Invalid points: {}", e))
    } else {
        Err(format!("Unsupported measurement: {}", value))
    }
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

fn left_edge_ptile_from_values(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<i32> = values.iter().map(|v| v.round() as i32).collect();
    sorted.sort();
    let idx = (sorted.len() as f32 * 0.05) as usize;
    sorted[idx.min(sorted.len() - 1)] as f32
}

#[derive(Debug, Clone)]
struct Line {
    x0: f32,
    x1: f32,
    top: f32,
    bottom: f32,
}

fn group_spans_into_lines(spans: &[&sp_extract::document::TextSpan]) -> Vec<Line> {
    if spans.is_empty() {
        return vec![];
    }
    let mut sorted: Vec<&&sp_extract::document::TextSpan> = spans.iter().collect();
    sorted.sort_by(|a, b| {
        a.bbox
            .0
            .partial_cmp(&b.bbox.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut lines: Vec<Line> = Vec::new();
    let first = sorted[0];
    let mut current = Line {
        x0: first.bbox.2,
        x1: first.bbox.3,
        top: first.bbox.0,
        bottom: first.bbox.1,
    };
    for span in sorted.iter().skip(1) {
        let (top, bottom, x0, x1) = span.bbox;
        let overlaps = top <= current.bottom + 3.0 && bottom >= current.top - 3.0;
        if overlaps {
            current.x0 = current.x0.min(x0);
            current.x1 = current.x1.max(x1);
            current.top = current.top.min(top);
            current.bottom = current.bottom.max(bottom);
        } else {
            lines.push(current);
            current = Line {
                x0,
                x1,
                top,
                bottom,
            };
        }
    }
    lines.push(current);
    lines
}

pub struct MarginsChecker;

impl Checker for MarginsChecker {
    fn category(&self) -> &'static str {
        "layout"
    }
    fn name(&self) -> &'static str {
        "margins"
    }

    fn check(&self, doc: &Document, params: &Value) -> CheckResult {
        let top_req = parse_measurement(params["top"].as_str().unwrap_or("1in")).unwrap_or(72.0);
        let bottom_req =
            parse_measurement(params["bottom"].as_str().unwrap_or("1in")).unwrap_or(72.0);
        let left_req =
            parse_measurement(params["left"].as_str().unwrap_or("1.25in")).unwrap_or(90.0);
        let right_req =
            parse_measurement(params["right"].as_str().unwrap_or("1.25in")).unwrap_or(90.0);
        let tolerance =
            parse_measurement(params["tolerance"].as_str().unwrap_or("0.125in")).unwrap_or(9.0);

        let mut excluded_pages: std::collections::HashSet<usize> = std::collections::HashSet::new();
        excluded_pages.insert(1);
        for pg in super::sections::find_section_pages(doc, &["accepted by"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["©", "copyright"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["dedication"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["abstract"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["table of contents"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["curriculum vitae"]) {
            excluded_pages.insert(pg);
        }

        let mut all_x0s: Vec<f32> = Vec::new();
        let mut all_right_gaps: Vec<f32> = Vec::new();
        let mut page_first_tops: Vec<f32> = Vec::new();
        let mut page_last_bottoms: Vec<f32> = Vec::new();

        for page in &doc.pages {
            if excluded_pages.contains(&page.page_number) {
                continue;
            }

            let raw_body: Vec<&sp_extract::document::TextSpan> = page
                .spans
                .iter()
                .filter(|s| {
                    let (top, bottom, _x0, _x1) = s.bbox;
                    top >= 36.0 && bottom <= page.height - 53.0 && s.text.trim().len() >= 3
                })
                .collect();
            if raw_body.is_empty() {
                continue;
            }

            let lines = group_spans_into_lines(&raw_body);
            let full_width: Vec<&Line> =
                lines.iter().filter(|l| l.x1 >= page.width * 0.7).collect();

            if full_width.len() < 3 {
                continue;
            }

            for line in &full_width {
                all_x0s.push(line.x0);
                all_right_gaps.push((page.width - line.x1).max(0.0));
            }

            if let Some(s) = raw_body.iter().min_by(|a, b| {
                a.bbox
                    .0
                    .partial_cmp(&b.bbox.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
                page_first_tops.push(s.bbox.0);
            }
            if let Some(s) = raw_body.iter().max_by(|a, b| {
                a.bbox
                    .1
                    .partial_cmp(&b.bbox.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
                page_last_bottoms.push(page.height - s.bbox.1);
            }
        }

        if all_x0s.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "Insufficient body text to measure margins".to_string(),
            };
        }

        let left_edge = left_edge_ptile_from_values(&all_x0s);
        let right_margin = left_edge_ptile_from_values(&all_right_gaps);

        let mut lines: Vec<String> = Vec::new();
        let mut violations: Vec<EvidenceItem> = Vec::new();

        for (label, value, req) in [
            ("left edge", left_edge, left_req),
            ("right margin", right_margin, right_req),
            ("top edge", mean(&page_first_tops), top_req),
            ("bottom margin", mean(&page_last_bottoms), bottom_req),
        ] {
            let lower = req - tolerance;
            let upper = req + tolerance;
            let pass = value >= lower && value <= upper;
            let status = if pass { "PASS" } else { "FAIL" };
            let direction = if value < lower {
                " too narrow"
            } else if value > upper {
                " too wide"
            } else {
                ""
            };
            lines.push(format!(
                "{}: {:.0}pt ({:.2}in) — range [{:.2}in–{:.2}in]. {}{}",
                label,
                value,
                value / 72.0,
                lower / 72.0,
                upper / 72.0,
                status,
                direction
            ));
            if !pass {
                violations.push(EvidenceItem {
                    page: 0,
                    bbox: None,
                    excerpt: Some(format!(
                        "{} {}pt outside [{}-{}pt]",
                        label,
                        value as i32,
                        (req - tolerance) as i32,
                        (req + tolerance) as i32
                    )),
                });
            }
        }

        if violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: lines.join("; "),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: lines.join("; "),
                evidence: violations,
            }
        }
    }
}

pub struct MarginSymmetryChecker;

impl Checker for MarginSymmetryChecker {
    fn category(&self) -> &'static str {
        "layout"
    }
    fn name(&self) -> &'static str {
        "margin_symmetry"
    }

    fn check(&self, doc: &Document, params: &Value) -> CheckResult {
        let threshold =
            parse_measurement(params["threshold"].as_str().unwrap_or("0.25in")).unwrap_or(18.0);

        let mut excluded_pages: std::collections::HashSet<usize> = std::collections::HashSet::new();
        excluded_pages.insert(1);
        for pg in super::sections::find_section_pages(doc, &["accepted by"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["©", "copyright"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["dedication"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["abstract"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["table of contents"]) {
            excluded_pages.insert(pg);
        }
        for pg in super::sections::find_section_pages(doc, &["curriculum vitae"]) {
            excluded_pages.insert(pg);
        }

        let mut evidence: Vec<EvidenceItem> = Vec::new();
        let mut asymmetrical_pages = 0usize;

        for page in &doc.pages {
            if excluded_pages.contains(&page.page_number) {
                continue;
            }

            let raw_body: Vec<&sp_extract::document::TextSpan> = page
                .spans
                .iter()
                .filter(|s| {
                    let (top, bottom, _x0, _x1) = s.bbox;
                    bottom < page.height - 53.0 && top >= 36.0 && s.text.trim().len() >= 3
                })
                .collect();
            if raw_body.is_empty() {
                continue;
            }

            let lines = group_spans_into_lines(&raw_body);
            let mut lefts: Vec<f32> = Vec::new();
            let mut rights: Vec<f32> = Vec::new();
            for line in &lines {
                if line.x1 >= page.width * 0.8 {
                    lefts.push(line.x0);
                    rights.push((page.width - line.x1).max(0.0));
                }
            }

            if lefts.len() < 3 {
                continue;
            }

            let left_mean = lefts.iter().sum::<f32>() / lefts.len() as f32;
            let right_mean = rights.iter().sum::<f32>() / rights.len() as f32;
            let diff = left_mean - right_mean;
            if diff.abs() > threshold {
                asymmetrical_pages += 1;
                let direction = if diff > 0.0 {
                    "left wider"
                } else {
                    "right wider"
                };
                evidence.push(EvidenceItem {
                    page: page.page_number,
                    bbox: None,
                    excerpt: Some(format!(
                        "asymmetry {:.0}pt ({:.2}in): L={:.0}pt R={:.0}pt ({})",
                        diff.abs(),
                        diff.abs() / 72.0,
                        left_mean,
                        right_mean,
                        direction
                    )),
                });
            }
        }

        if asymmetrical_pages == 0 {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "Left and right margins are symmetric".to_string(),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "{} page(s) have asymmetric margins (threshold: {:.0}pt / {:.2}in)",
                    asymmetrical_pages,
                    threshold,
                    threshold / 72.0
                ),
                evidence,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_extract::document::{ParsedDocument as Document, ParsedPage as Page, TextSpan};

    fn make_span(text: &str, bbox: (f32, f32, f32, f32)) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            font_name: "TimesNewRoman".to_string(),
            font_size: 12.0,
            bbox,
            is_bold: false,
            is_italic: false,
            color: None,
        }
    }

    fn default_params() -> Value {
        serde_yaml::from_str("top: 1in\nbottom: 1in\nleft: 1.25in\nright: 1.25in\n").unwrap()
    }

    fn symmetry_params() -> Value {
        serde_yaml::from_str("threshold: 0.25in\nleft: 1.25in\nright: 1.25in\ntolerance: 0.125in\n")
            .unwrap()
    }

    fn body_span_bboxes(
        count: usize,
        left_x: f32,
        right_x: f32,
        top_start: f32,
        gap: f32,
    ) -> Vec<(f32, f32, f32, f32)> {
        (0..count)
            .map(|i| {
                let top = top_start + i as f32 * gap;
                (top, top + 12.0, left_x, right_x)
            })
            .collect()
    }

    fn build_doc(
        body_bboxes: Vec<(f32, f32, f32, f32)>,
        extra: Vec<(f32, f32, f32, f32, &str)>,
    ) -> Document {
        let mut all: Vec<(f32, f32, f32, f32)> = body_bboxes.clone();
        for &(top, bottom, x0, x1, _text) in &extra {
            all.push((top, bottom, x0, x1));
        }
        let mut spans: Vec<TextSpan> = all.iter().map(|&b| make_span("text here", b)).collect();
        for (i, &(_, _, _, _, text)) in extra.iter().enumerate() {
            spans[body_bboxes.len() + i].text = text.to_string();
        }
        Document {
            markdown_text: None,
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
                page_number: 2,
                width: 612.0,
                height: 792.0,
                spans,
                images: vec![],
                paths: vec![],
            }],
        }
    }

    // --- Line grouping tests ---

    #[test]
    fn test_line_grouping_basic() {
        let spans: Vec<TextSpan> = vec![
            make_span("hello", (80.0, 92.0, 90.0, 130.0)),
            make_span("world", (80.5, 92.5, 140.0, 190.0)),
            make_span("next", (110.0, 122.0, 90.0, 130.0)),
            make_span("line", (112.0, 124.0, 140.0, 180.0)),
        ];
        let refs: Vec<&TextSpan> = spans.iter().collect();
        let lines = group_spans_into_lines(&refs);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].x0 <= lines[0].x1);
        assert!(lines[1].x0 <= lines[1].x1);
    }

    #[test]
    fn test_line_grouping_single_span() {
        let spans = [make_span("solo", (100.0, 112.0, 90.0, 130.0))];
        let refs: Vec<&TextSpan> = spans.iter().collect();
        let lines = group_spans_into_lines(&refs);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].x0, 90.0);
    }

    #[test]
    fn test_line_grouping_empty() {
        let spans: Vec<TextSpan> = vec![];
        let refs: Vec<&TextSpan> = spans.iter().collect();
        let lines = group_spans_into_lines(&refs);
        assert!(lines.is_empty());
    }

    // --- MarginsChecker tests ---

    #[test]
    fn test_margins_pass() {
        let body = body_span_bboxes(30, 94.0, 518.0, 80.0, 24.0);
        let extra = vec![
            (80.0, 92.0, 200.0, 350.0, "Centered Heading"),
            (300.0, 312.0, 100.0, 400.0, "Short caption"),
        ];
        let doc = build_doc(body, extra);
        let r = MarginsChecker.check(&doc, &default_params());
        assert_eq!(r.status, Status::Pass, "{}", r.detail);
    }

    #[test]
    fn test_margins_fail_left() {
        let body = body_span_bboxes(30, 60.0, 518.0, 80.0, 24.0);
        let doc = build_doc(body, vec![]);
        let r = MarginsChecker.check(&doc, &default_params());
        assert_eq!(r.status, Status::Fail, "{}", r.detail);
    }

    #[test]
    fn test_margins_fail_right() {
        let body = body_span_bboxes(30, 94.0, 550.0, 80.0, 24.0);
        let doc = build_doc(body, vec![]);
        let r = MarginsChecker.check(&doc, &default_params());
        assert_eq!(r.status, Status::Fail, "{}", r.detail);
    }

    #[test]
    fn test_chapter_heading_top_margin() {
        let spans: Vec<(f32, f32, f32, f32)> = vec![
            (144.0, 156.0, 200.0, 412.0),
            (180.0, 192.0, 90.0, 522.0),
            (204.0, 216.0, 90.0, 522.0),
            (228.0, 240.0, 90.0, 522.0),
            (252.0, 264.0, 90.0, 522.0),
        ];
        let doc = Document {
            markdown_text: None,
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
                page_number: 2,
                width: 612.0,
                height: 792.0,
                spans: spans.iter().map(|&b| make_span("text", b)).collect(),
                images: vec![],
                paths: vec![],
            }],
        };
        let r = MarginsChecker.check(&doc, &default_params());
        assert!(
            r.detail.contains("top edge"),
            "should measure top margin from heading: {}",
            r.detail
        );
    }

    #[test]
    fn test_sparse_page_skip() {
        let body = body_span_bboxes(1, 94.0, 518.0, 80.0, 24.0);
        let extra = vec![
            (80.0, 92.0, 200.0, 350.0, "Centered"),
            (104.0, 116.0, 200.0, 350.0, "Centered2"),
        ];
        let doc = build_doc(body, extra);
        let r = MarginsChecker.check(&doc, &default_params());
        assert_eq!(
            r.status,
            Status::Error,
            "page with <3 full-width lines should be skipped: {}",
            r.detail
        );
    }

    #[test]
    fn test_margins_error_empty() {
        let doc = Document {
            markdown_text: None,
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
            pages: vec![],
        };
        let r = MarginsChecker.check(&doc, &default_params());
        assert_eq!(r.status, Status::Error);
    }

    // --- MarginSymmetryChecker tests ---

    #[test]
    fn test_symmetry_pass() {
        let body = body_span_bboxes(30, 90.0, 522.0, 80.0, 24.0);
        let extra = vec![(80.0, 92.0, 200.0, 350.0, "Centered")];
        let doc = build_doc(body, extra);
        let r = MarginSymmetryChecker.check(&doc, &symmetry_params());
        assert_eq!(r.status, Status::Pass, "{}", r.detail);
    }

    #[test]
    fn test_symmetry_fail() {
        let body = body_span_bboxes(30, 90.0, 502.0, 80.0, 24.0);
        let doc = build_doc(body, vec![]);
        let r = MarginSymmetryChecker.check(&doc, &symmetry_params());
        assert_eq!(r.status, Status::Fail, "{}", r.detail);
    }
}
