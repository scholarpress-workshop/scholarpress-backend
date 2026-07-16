use crate::checkers::{CheckResult, Checker, EvidenceItem, Status};
use serde_yaml::Value;
use sp_extract::document::ParsedDocument as Document;
use std::collections::{HashMap, HashSet};

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

pub fn normalize_family(font_name: &str) -> String {
    let name = if let Some(idx) = font_name.find('+') {
        &font_name[idx + 1..]
    } else {
        font_name
    };

    let lower = name.to_lowercase();
    if lower.starts_with("newcm")
        || lower.starts_with("cmr")
        || lower.starts_with("cmbx")
        || lower.starts_with("cmmi")
        || lower.starts_with("cmsy")
        || lower.starts_with("cmex")
        || lower.starts_with("cmti")
        || lower.starts_with("cmsl")
        || lower.contains("computer modern")
        || lower.starts_with("tex-math")
    {
        return "ComputerModern".to_string();
    }

    let suffixes = [
        "PS",
        "MT",
        "-Regular",
        "-BoldItalic",
        "-Bold",
        "-Italic",
        "-Oblique",
        "-Identity-H",
        "-Book",
        "-BookItalic",
        "-RegularItalic",
    ];

    let mut result = name.to_string();
    for suffix in &suffixes {
        result = result.replace(suffix, "");
    }
    result.trim_matches('-').to_string()
}

fn is_internal_font_name(name: &str) -> bool {
    if name.len() < 4 {
        return true;
    }
    name.chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && name.len() <= 6
}

fn is_non_body_text(span: &sp_extract::document::TextSpan) -> bool {
    let text = span.text.trim();
    if text.is_empty() {
        return false;
    }

    let is_monospace = span.font_name.to_lowercase().contains("mono")
        || span.font_name.to_lowercase().contains("code");
    if is_monospace {
        return true;
    }

    let is_math = span.font_name.to_lowercase().contains("math");
    if is_math {
        return true;
    }

    let alpha_count = text.chars().filter(|c| c.is_alphabetic()).count();
    if alpha_count == 0 && text.len() <= 4 {
        return true;
    }

    false
}

fn is_near_image(
    page: &sp_extract::document::ParsedPage,
    span: &sp_extract::document::TextSpan,
) -> bool {
    let (st, sb, sx0, sx1) = span.bbox;
    for &(it, ib, ix0, ix1) in &page.images {
        let overlap = sx0 < ix1 && sx1 > ix0 && st < ib && sb > it;
        if overlap {
            return true;
        }
    }
    for &(pt, pb, px0, px1) in &page.paths {
        let overlap = sx0 < px1 && sx1 > px0 && st < pb && sb > pt;
        if overlap {
            return true;
        }
    }
    false
}

pub struct FontSizeChecker;

impl Checker for FontSizeChecker {
    fn category(&self) -> &'static str {
        "typography"
    }

    fn name(&self) -> &'static str {
        "font_size"
    }

    fn check(&self, doc: &Document, params: &Value) -> CheckResult {
        let allowed: Vec<f32> = params
            .get("allowed")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| parse_measurement(s).ok())
                    .collect()
            })
            .unwrap_or_default();

        let tolerance: f32 = 0.5;
        let consistent = params
            .get("consistent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut violations: Vec<EvidenceItem> = Vec::new();
        let mut body_sizes: HashMap<i32, usize> = HashMap::new();

        for page in &doc.pages {
            for span in &page.spans {
                let (top, bottom, _x0, _x1) = span.bbox;

                if bottom >= (page.height - 53.0) {
                    continue;
                }
                if top < 36.0 {
                    continue;
                }
                if span.text.trim().len() < 3 {
                    continue;
                }

                let size = span.font_size;

                if size < 8.5 {
                    continue;
                }

                if size < 10.0 && (is_near_image(page, span) || is_non_body_text(span)) {
                    continue;
                }

                if consistent {
                    let key = (size * 10.0).round() as i32;
                    *body_sizes.entry(key).or_insert(0) += 1;
                }

                if allowed.is_empty() {
                    continue;
                }

                let matched = allowed.iter().any(|a| (size - a).abs() <= tolerance);
                if !matched {
                    violations.push(EvidenceItem {
                        page: page.page_number,
                        bbox: Some(span.bbox),
                        excerpt: Some(format!("{} ({:.1}pt)", span.text, size,)),
                    });
                }
            }
        }

        if consistent && !body_sizes.is_empty() && body_sizes.len() > 1 {
            let modal_decipt = body_sizes
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(k, _)| *k)
                .unwrap_or(0);
            let modal_size = modal_decipt as f32 / 10.0;

            let mut body_violations: Vec<EvidenceItem> = Vec::new();
            for page in &doc.pages {
                for span in &page.spans {
                    let (top, bottom, _x0, _x1) = span.bbox;

                    if bottom >= (page.height - 53.0) || top < 36.0 {
                        continue;
                    }
                    if span.text.trim().len() < 3 {
                        continue;
                    }

                    let size = span.font_size;

                    if size < 8.5 {
                        continue;
                    }

                    if allowed.iter().any(|a| (size - a).abs() <= tolerance) {
                        continue;
                    }

                    if (size - modal_size).abs() > tolerance {
                        body_violations.push(EvidenceItem {
                            page: page.page_number,
                            bbox: Some(span.bbox),
                            excerpt: Some(format!(
                                "{} ({:.1}pt, expected {:.0}pt)",
                                span.text, size, modal_size,
                            )),
                        });
                    }
                }
            }

            if !body_violations.is_empty() {
                violations.extend(body_violations);
            }
        }

        if violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "All text conforms to font size requirements".to_string(),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "{} span(s) violate font size requirements",
                    violations.len(),
                ),
                evidence: violations,
            }
        }
    }
}

pub struct FontWeightChecker;

impl Checker for FontWeightChecker {
    fn category(&self) -> &'static str {
        "typography"
    }

    fn name(&self) -> &'static str {
        "font_weight"
    }

    fn check(&self, doc: &Document, params: &Value) -> CheckResult {
        let expected = params
            .get("weight")
            .and_then(|v| v.as_str())
            .unwrap_or("normal");
        let page_filter = params
            .get("page")
            .and_then(|v| v.as_u64())
            .map(|p| p as usize);
        let invert = params
            .get("invert")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut violations: Vec<EvidenceItem> = Vec::new();

        for page in &doc.pages {
            if let Some(target) = page_filter {
                if page.page_number != target {
                    continue;
                }
            }

            for span in &page.spans {
                if span.text.trim().is_empty() {
                    continue;
                }
                let (top, bottom, _x0, _x1) = span.bbox;
                if bottom >= (page.height - 53.0) {
                    continue;
                }
                if top < 36.0 {
                    continue;
                }

                let detected = match (span.is_bold, span.is_italic) {
                    (true, true) => "bold-italic",
                    (true, false) => "bold",
                    (false, true) => "italic",
                    (false, false) => "normal",
                };

                let is_violation = if invert {
                    detected == expected
                } else {
                    detected != expected
                };

                if is_violation {
                    let detail = if invert {
                        format!("{} ({}, should not be {})", span.text, detected, expected,)
                    } else {
                        format!("{} ({}, expected {})", span.text, detected, expected,)
                    };
                    violations.push(EvidenceItem {
                        page: page.page_number,
                        bbox: Some(span.bbox),
                        excerpt: Some(detail),
                    });
                }
            }
        }

        if violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "All text conforms to font weight requirements".to_string(),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "{} span(s) violate font weight requirements",
                    violations.len(),
                ),
                evidence: violations,
            }
        }
    }
}

pub struct FontFamilyChecker;

impl Checker for FontFamilyChecker {
    fn category(&self) -> &'static str {
        "typography"
    }

    fn name(&self) -> &'static str {
        "font_family"
    }

    fn check(&self, doc: &Document, params: &Value) -> CheckResult {
        let allowed: HashSet<String> = params
            .get("allowed")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let consistent = params
            .get("consistent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let special_fonts: HashSet<&str> = [
            "Symbol",
            "Wingdings",
            "CambriaMath",
            "ZapfDingbats",
            "Aptos",
        ]
        .into_iter()
        .collect();

        let mut violations: Vec<EvidenceItem> = Vec::new();
        let mut family_counts: HashMap<String, usize> = HashMap::new();

        for page in &doc.pages {
            for span in &page.spans {
                if span.text.trim().is_empty() {
                    continue;
                }
                let (top, bottom, _x0, _x1) = span.bbox;
                if bottom >= (page.height - 53.0) {
                    continue;
                }
                if top < 36.0 {
                    continue;
                }

                let family = normalize_family(&span.font_name);

                if is_internal_font_name(&family) || special_fonts.contains(family.as_str()) {
                    continue;
                }

                let is_chart_text = span.font_size < 10.0 && is_near_image(page, span);

                if consistent && !is_chart_text {
                    *family_counts.entry(family.clone()).or_insert(0) += 1;
                }

                if allowed.is_empty() {
                    continue;
                }

                if !is_chart_text && !is_non_body_text(span) && !allowed.contains(&family) {
                    violations.push(EvidenceItem {
                        page: page.page_number,
                        bbox: Some(span.bbox),
                        excerpt: Some(format!("{} ({})", span.text, family)),
                    });
                }
            }
        }

        if consistent && !family_counts.is_empty() && family_counts.len() > 1 {
            let modal_family = family_counts
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(k, _)| k.clone())
                .unwrap_or_default();

            let total_spans: usize = family_counts.values().sum();
            let threshold = (total_spans as f32 * 0.01) as usize;

            for page in &doc.pages {
                for span in &page.spans {
                    if span.text.trim().is_empty() {
                        continue;
                    }
                    let (top, bottom, _x0, _x1) = span.bbox;
                    if bottom >= (page.height - 53.0) || top < 36.0 {
                        continue;
                    }
                    let family = normalize_family(&span.font_name);
                    if is_internal_font_name(&family) || special_fonts.contains(family.as_str()) {
                        continue;
                    }
                    if *family_counts.get(&family).unwrap_or(&0) <= threshold {
                        continue;
                    }
                    if family != modal_family
                        && (allowed.is_empty() || !allowed.contains(&family))
                        && !(span.font_size < 10.0 && is_near_image(page, span))
                        && !is_non_body_text(span)
                    {
                        violations.push(EvidenceItem {
                            page: page.page_number,
                            bbox: Some(span.bbox),
                            excerpt: Some(format!(
                                "{} ({}, expected {})",
                                span.text, family, modal_family,
                            )),
                        });
                    }
                }
            }
        }

        if violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "All text conforms to font family requirements".to_string(),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "{} span(s) violate font family requirements",
                    violations.len(),
                ),
                evidence: violations,
            }
        }
    }
}

fn stdev(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;
    variance.sqrt()
}

fn classify_page_justification(page: &sp_extract::document::ParsedPage) -> Option<String> {
    let body_spans: Vec<_> = page
        .spans
        .iter()
        .filter(|s| {
            let (top, bottom, _x0, _x1) = s.bbox;
            !s.text.trim().is_empty() && top >= 72.0 && bottom <= (page.height - 72.0)
        })
        .collect();

    if body_spans.len() < 50 {
        return None;
    }

    let mut sorted = body_spans;
    sorted.sort_by(|a, b| {
        a.bbox
            .0
            .partial_cmp(&b.bbox.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.bbox
                    .2
                    .partial_cmp(&b.bbox.2)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    let mut rights: Vec<f32> = Vec::new();
    let mut line_start: usize = 0;

    for i in 1..sorted.len() {
        let prev = sorted[i - 1];
        let curr = sorted[i];
        if curr.bbox.2 < prev.bbox.2 - 10.0 || (curr.bbox.0 - prev.bbox.0).abs() > 3.0 {
            let line_max = sorted[line_start..i]
                .iter()
                .map(|s| s.bbox.3)
                .fold(f32::MIN, f32::max);
            rights.push(line_max);
            line_start = i;
        }
    }
    if line_start < sorted.len() {
        let line_max = sorted[line_start..]
            .iter()
            .map(|s| s.bbox.3)
            .fold(f32::MIN, f32::max);
        rights.push(line_max);
    }

    if rights.len() < 5 {
        return None;
    }

    let sd = stdev(&rights);

    if sd < 8.0 {
        Some("justified".to_string())
    } else {
        Some("left".to_string())
    }
}

pub struct JustificationChecker;

impl Checker for JustificationChecker {
    fn category(&self) -> &'static str {
        "typography"
    }

    fn name(&self) -> &'static str {
        "justification"
    }

    fn check(&self, doc: &Document, params: &Value) -> CheckResult {
        let consistent = params
            .get("consistent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut page_styles: Vec<(usize, String)> = Vec::new();

        for page in &doc.pages {
            if page.page_number <= 5 {
                continue;
            }
            let has_roman_pn = page
                .spans
                .iter()
                .filter(|s| s.bbox.1 >= (page.height - 53.0) && !s.text.trim().is_empty())
                .any(|s| {
                    let t = s.text.trim();
                    !t.is_empty() && t.chars().all(|c| "ivxlcdmIVXLCDM".contains(c))
                });
            if has_roman_pn {
                continue;
            }

            if let Some(style) = classify_page_justification(page) {
                page_styles.push((page.page_number, style));
            }
        }

        if page_styles.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "No body pages to analyze".to_string(),
            };
        }

        if consistent {
            let mut styles = std::collections::HashSet::new();
            for (_, style) in &page_styles {
                styles.insert(style.clone());
            }

            if styles.len() > 1 {
                let mut style_counts: HashMap<String, usize> = HashMap::new();
                for (_, style) in &page_styles {
                    *style_counts.entry(style.clone()).or_insert(0) += 1;
                }

                let dominant = style_counts
                    .iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_default();

                let mut violations: Vec<EvidenceItem> = Vec::new();
                for (pn, st) in &page_styles {
                    if *st != dominant {
                        violations.push(EvidenceItem {
                            page: *pn,
                            bbox: None,
                            excerpt: Some(format!("Page {}: {} (expected {})", pn, st, dominant,)),
                        });
                    }
                }

                return CheckResult {
                    check_id: String::new(),
                    status: Status::Fail,
                    detail: format!(
                        "{} page(s) have inconsistent justification",
                        violations.len(),
                    ),
                    evidence: violations,
                };
            }
        }

        let style = &page_styles[0].1;
        CheckResult {
            check_id: String::new(),
            status: Status::Pass,
            evidence: vec![],
            detail: format!(
                "All {} body pages consistently {}-aligned",
                page_styles.len(),
                style,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_extract::document::{ParsedDocument as Document, ParsedPage as Page, TextSpan};

    fn make_span(
        text: &str,
        font_size: f32,
        font_name: &str,
        bbox: (f32, f32, f32, f32),
        is_bold: bool,
        is_italic: bool,
    ) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            font_name: font_name.to_string(),
            font_size,
            bbox,
            is_bold,
            is_italic,
            color: None,
        }
    }

    fn make_page(spans: Vec<TextSpan>) -> Page {
        Page {
            text: String::new(),
            page_number: 1,
            width: 612.0,
            height: 792.0,
            spans,
            images: vec![],
            paths: vec![],
        }
    }

    fn make_doc(spans: Vec<TextSpan>) -> Document {
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
            pages: vec![make_page(spans)],
        }
    }

    #[test]
    fn test_font_size_pass() {
        let doc = make_doc(vec![make_span(
            "Hello",
            12.0,
            "Times",
            (100.0, 112.0, 92.0, 200.0),
            false,
            false,
        )]);
        let r = FontSizeChecker.check(
            &doc,
            &serde_yaml::from_str("allowed: [\"10pt\",\"11pt\",\"12pt\"]\n").unwrap(),
        );
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_font_size_fail() {
        let doc = make_doc(vec![make_span(
            "Big",
            14.0,
            "Times",
            (100.0, 112.0, 92.0, 200.0),
            false,
            false,
        )]);
        let r = FontSizeChecker.check(
            &doc,
            &serde_yaml::from_str("allowed: [\"10pt\",\"11pt\",\"12pt\"]\n").unwrap(),
        );
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn test_font_size_skips_tiny() {
        let doc = make_doc(vec![make_span(
            "tiny",
            6.0,
            "Times",
            (100.0, 112.0, 92.0, 200.0),
            false,
            false,
        )]);
        let r = FontSizeChecker.check(
            &doc,
            &serde_yaml::from_str("allowed: [\"10pt\"]\n").unwrap(),
        );
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_font_size_skips_page_number_zone() {
        let doc = make_doc(vec![make_span(
            "10",
            12.0,
            "Times",
            (740.0, 752.0, 300.0, 310.0),
            false,
            false,
        )]);
        let r = FontSizeChecker.check(
            &doc,
            &serde_yaml::from_str("allowed: [\"10pt\"]\n").unwrap(),
        );
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_font_weight_normal_pass() {
        let doc = make_doc(vec![make_span(
            "Hello",
            12.0,
            "Times",
            (100.0, 112.0, 92.0, 200.0),
            false,
            false,
        )]);
        let r = FontWeightChecker.check(&doc, &serde_yaml::from_str("weight: normal\n").unwrap());
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_font_weight_bold_fail() {
        let doc = make_doc(vec![make_span(
            "Bold",
            12.0,
            "Times-Bold",
            (100.0, 112.0, 92.0, 200.0),
            true,
            false,
        )]);
        let r = FontWeightChecker.check(&doc, &serde_yaml::from_str("weight: normal\n").unwrap());
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn test_font_weight_bold_expected_pass() {
        let doc = make_doc(vec![make_span(
            "Bold",
            12.0,
            "Times-Bold",
            (100.0, 112.0, 92.0, 200.0),
            true,
            false,
        )]);
        let r = FontWeightChecker.check(&doc, &serde_yaml::from_str("weight: bold\n").unwrap());
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_font_weight_italic_fail() {
        let doc = make_doc(vec![make_span(
            "Italic",
            12.0,
            "Times-Italic",
            (100.0, 112.0, 92.0, 200.0),
            false,
            true,
        )]);
        let r = FontWeightChecker.check(&doc, &serde_yaml::from_str("weight: normal\n").unwrap());
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn test_font_weight_invert() {
        let doc = make_doc(vec![make_span(
            "Normal",
            12.0,
            "Times",
            (100.0, 112.0, 92.0, 200.0),
            false,
            false,
        )]);
        let r = FontWeightChecker.check(
            &doc,
            &serde_yaml::from_str("weight: normal\ninvert: true\n").unwrap(),
        );
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn test_font_weight_page_filter() {
        let mut page = make_page(vec![make_span(
            "Hello",
            12.0,
            "Times",
            (100.0, 112.0, 92.0, 200.0),
            false,
            false,
        )]);
        page.page_number = 5;
        let doc = Document {
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
            pages: vec![page],
        };
        let r = FontWeightChecker.check(
            &doc,
            &serde_yaml::from_str("weight: normal\npage: 1\n").unwrap(),
        );
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_font_family_consistent_pass() {
        let doc = make_doc(vec![
            make_span(
                "A",
                12.0,
                "TimesNewRoman",
                (100.0, 112.0, 92.0, 110.0),
                false,
                false,
            ),
            make_span(
                "B",
                12.0,
                "TimesNewRoman",
                (114.0, 126.0, 92.0, 110.0),
                false,
                false,
            ),
        ]);
        let r = FontFamilyChecker.check(&doc, &serde_yaml::from_str("consistent: true\n").unwrap());
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_font_family_mixed_fail() {
        let doc = make_doc(vec![
            make_span(
                "A",
                12.0,
                "TimesNewRoman",
                (100.0, 112.0, 92.0, 110.0),
                false,
                false,
            ),
            make_span(
                "B",
                12.0,
                "Arial",
                (114.0, 126.0, 92.0, 110.0),
                false,
                false,
            ),
        ]);
        let r = FontFamilyChecker.check(&doc, &serde_yaml::from_str("consistent: true\n").unwrap());
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn test_font_family_skips_symbol() {
        let doc = make_doc(vec![make_span(
            "X",
            12.0,
            "Symbol",
            (100.0, 112.0, 92.0, 200.0),
            false,
            false,
        )]);
        let r = FontFamilyChecker.check(&doc, &serde_yaml::from_str("consistent: true\n").unwrap());
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_justification_skips_sparse_pages() {
        let page = Page {
            text: String::new(),
            page_number: 7,
            width: 612.0,
            height: 792.0,
            spans: vec![make_span(
                "x",
                12.0,
                "Times",
                (100.0, 112.0, 92.0, 200.0),
                false,
                false,
            )],
            images: vec![],
            paths: vec![],
        };
        let doc = Document {
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
            pages: vec![page],
        };
        let r =
            JustificationChecker.check(&doc, &serde_yaml::from_str("consistent: true\n").unwrap());
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_justification_skips_early_pages() {
        let mut page = Page {
            text: String::new(),
            page_number: 3,
            width: 612.0,
            height: 792.0,
            spans: vec![],
            images: vec![],
            paths: vec![],
        };
        for i in 0..60 {
            page.spans.push(make_span(
                "x",
                12.0,
                "Times",
                (
                    100.0,
                    112.0,
                    92.0 + (i as f32 * 5.0),
                    200.0 + (i as f32 * 5.0),
                ),
                false,
                false,
            ));
        }
        let doc = Document {
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
            pages: vec![page],
        };
        let r =
            JustificationChecker.check(&doc, &serde_yaml::from_str("consistent: true\n").unwrap());
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_normalize_family_strips_prefix() {
        assert_eq!(
            normalize_family("SYTYAE+TimesNewRomanPSMT"),
            "TimesNewRoman"
        );
    }

    #[test]
    fn test_is_internal_font_name() {
        assert!(is_internal_font_name("TT0"));
        assert!(!is_internal_font_name("TimesNewRoman"));
    }
}
