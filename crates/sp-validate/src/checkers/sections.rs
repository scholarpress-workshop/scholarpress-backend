use crate::checkers::typography::normalize_family;
use crate::checkers::{CheckResult, Checker, EvidenceItem, Status};
use crate::document::Document;
use serde_yaml::Value;
use std::collections::HashMap;

pub(crate) fn find_section_pages(doc: &Document, keywords: &[&str]) -> Vec<usize> {
    doc.pages
        .iter()
        .filter(|p| {
            let text: String = p
                .spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let low = text.to_lowercase();
            keywords.iter().any(|k| low.contains(k))
        })
        .map(|p| p.page_number)
        .collect()
}

fn find_abstract_page(doc: &Document) -> Option<&crate::document::Page> {
    if let Some(p) = doc.pages.iter().find(|p| {
        let text: String = p
            .spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        text.to_lowercase().contains("abstract of")
    }) {
        return Some(p);
    }

    let acc_pg = doc
        .pages
        .iter()
        .find(|p| {
            let text: String = p
                .spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            text.to_lowercase().contains("accepted by")
        })
        .map(|p| p.page_number);

    let toc_pg = doc
        .pages
        .iter()
        .find(|p| {
            let text: String = p
                .spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            text.to_lowercase().contains("table of contents")
        })
        .map(|p| p.page_number);

    if let (Some(acc), Some(toc)) = (acc_pg, toc_pg) {
        for page in doc.pages.iter().rev() {
            if page.page_number > acc && page.page_number < toc {
                let n = page
                    .spans
                    .iter()
                    .filter(|s| !s.text.trim().is_empty())
                    .count();
                let text: String = page
                    .spans
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                let low = text.to_lowercase();
                let is_other = [
                    "dedication",
                    "acknowledgement",
                    "acknowledgments",
                    "preface",
                ]
                .iter()
                .any(|h| low[..200.min(low.len())].contains(h));
                if n > 100 && !is_other {
                    return Some(page);
                }
            }
        }
    }
    None
}

fn body_style(doc: &Document) -> (String, f32) {
    let start = 6usize.min(doc.pages.len().saturating_sub(1));
    let end = (start + 10).min(doc.pages.len());
    let mut fams: HashMap<String, usize> = HashMap::new();
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
            *fams.entry(normalize_family(&s.font_name)).or_insert(0) += 1;
            *sizes
                .entry((s.font_size * 10.0).round() as i32)
                .or_insert(0) += 1;
        }
    }
    let family = fams
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(k, _)| k.clone())
        .unwrap_or_default();
    let sz = sizes
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(k, _)| *k)
        .unwrap_or(0);
    (family, sz as f32 / 10.0)
}

fn page_center(page: &crate::document::Page) -> f32 {
    page.width / 2.0
}

fn line_center(spans: &[&&crate::document::TextSpan]) -> f32 {
    let left = spans
        .iter()
        .map(|s| s.bbox.2)
        .fold(f32::MAX, |a, b| a.min(b));
    let right = spans
        .iter()
        .map(|s| s.bbox.3)
        .fold(f32::MIN, |a, b| a.max(b));
    (left + right) / 2.0
}

// ── 7.7a: references_font_consistent ──────────────────────────────

pub struct ReferencesFontChecker;

impl Checker for ReferencesFontChecker {
    fn category(&self) -> &'static str {
        "typography"
    }
    fn name(&self) -> &'static str {
        "references_font_consistent"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let (body_fam, body_sz) = body_style(doc);
        let ref_pages = find_section_pages(doc, &["references", "bibliography", "works cited"]);
        if ref_pages.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "References section not found".to_string(),
            };
        }

        let mut violations: Vec<EvidenceItem> = Vec::new();
        for pg in &ref_pages {
            let page = &doc.pages[pg - 1];
            for s in &page.spans {
                let (top, bottom, _x0, _x1) = s.bbox;
                if bottom >= page.height - 53.0 || top < 72.0 {
                    continue;
                }
                if s.text.trim().len() < 3 {
                    continue;
                }
                if s.font_size < 8.0 {
                    continue;
                }
                let fam = normalize_family(&s.font_name);
                let special: &[&str] = &[
                    "Symbol",
                    "Wingdings",
                    "CambriaMath",
                    "ZapfDingbats",
                    "Aptos",
                ];
                if special.contains(&fam.as_str()) {
                    continue;
                }
                if fam != body_fam || (s.font_size - body_sz).abs() > 2.0 {
                    violations.push(EvidenceItem {
                        page: *pg,
                        bbox: Some(s.bbox),
                        excerpt: Some(format!(
                            "{} ({}, {:.0}pt, expected {} {:.0}pt)",
                            s.text, s.font_name, s.font_size, body_fam, body_sz
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
                    "References font matches body ({} {:.0}pt)",
                    body_fam, body_sz
                ),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!("{} reference(s) with mismatched font", violations.len()),
                evidence: violations,
            }
        }
    }
}

// ── 7.7b: references_heading_format ──────────────────────────────

fn get_chapter_heading_style(doc: &Document) -> Option<(String, f32, bool)> {
    let mut fams: HashMap<String, usize> = HashMap::new();
    let mut sizes: HashMap<i32, usize> = HashMap::new();
    let mut bold_count = 0usize;

    for page in &doc.pages {
        let page_text: String = page
            .spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let low = page_text.to_lowercase();
        if !(low.contains("chapter ") || low.contains("appendix ")) {
            continue;
        }

        for s in &page.spans {
            let (top, bottom, _x0, _x1) = s.bbox;
            let low_s = s.text.to_lowercase();
            if !low_s.contains("chapter") && !low_s.contains("appendix") {
                continue;
            }
            if bottom >= page.height - 53.0 || top < 36.0 {
                continue;
            }
            *fams.entry(normalize_family(&s.font_name)).or_insert(0) += 1;
            *sizes
                .entry((s.font_size * 10.0).round() as i32)
                .or_insert(0) += 1;
            if s.is_bold {
                bold_count += 1;
            }
        }
    }

    if fams.is_empty() {
        return None;
    }
    let fam = fams
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(k, _)| k.clone())?;
    let sz = sizes.iter().max_by_key(|(_, c)| *c).map(|(k, _)| *k)?;
    Some((fam, sz as f32 / 10.0, bold_count > 0))
}

pub struct ReferencesHeadingChecker;

impl Checker for ReferencesHeadingChecker {
    fn category(&self) -> &'static str {
        "structure"
    }
    fn name(&self) -> &'static str {
        "references_heading_format"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let ch_style = match get_chapter_heading_style(doc) {
            Some(s) => s,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "No chapter headings found for comparison".to_string(),
                }
            }
        };
        let (ch_fam, ch_sz, ch_bold) = ch_style;
        let ref_pages = find_section_pages(doc, &["references", "bibliography", "works cited"]);

        if ref_pages.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "References section not found".to_string(),
            };
        }

        let page = &doc.pages[ref_pages[0] - 1];
        let ref_heading = page.spans.iter().find(|s| {
            let (top, bottom, _x0, _x1) = s.bbox;
            let low = s.text.to_lowercase();
            bottom <= page.height - 53.0
                && top >= 36.0
                && (low.contains("references")
                    || low.contains("bibliography")
                    || low.contains("works cited"))
        });

        match ref_heading {
            Some(s) => {
                let fam = normalize_family(&s.font_name);
                if fam == ch_fam && (s.font_size - ch_sz).abs() <= 1.5 && s.is_bold == ch_bold {
                    CheckResult {
                        check_id: String::new(),
                        status: Status::Pass,
                        evidence: vec![],
                        detail: format!(
                            "References heading matches chapter style ({}, {:.0}pt)",
                            ch_fam, ch_sz
                        ),
                    }
                } else {
                    CheckResult {
                        check_id: String::new(),
                        status: Status::Fail,
                        detail: format!(
                            "References heading font differs: {} {:.0}pt (chapters: {} {:.0}pt)",
                            s.font_name, s.font_size, ch_fam, ch_sz
                        ),
                        evidence: vec![EvidenceItem {
                            page: ref_pages[0],
                            bbox: Some(s.bbox),
                            excerpt: Some(s.text.clone()),
                        }],
                    }
                }
            }
            None => CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "References heading not found".to_string(),
            },
        }
    }
}

// ── 7.8a: cv_heading_format ──────────────────────────────────────

pub struct CvHeadingChecker;

impl Checker for CvHeadingChecker {
    fn category(&self) -> &'static str {
        "structure"
    }
    fn name(&self) -> &'static str {
        "cv_heading_format"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let ch_style = match get_chapter_heading_style(doc) {
            Some(s) => s,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "No chapter headings found for comparison".to_string(),
                }
            }
        };
        let (ch_fam, ch_sz, _ch_bold) = ch_style;
        let cv_page = doc.pages.iter().rev().find(|p| {
            let text: String = p
                .spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let low = text.to_lowercase();
            low.contains("curriculum vitae")
                || (low.contains("curriculum") && low.contains("vitae"))
        });

        match cv_page {
            Some(p) => {
                let cv_heading = p
                    .spans
                    .iter()
                    .filter(|s| {
                        let (top, bottom, _x0, _x1) = s.bbox;
                        let low = s.text.to_lowercase();
                        bottom <= p.height - 53.0
                            && top >= 36.0
                            && (low.contains("curriculum") || low.contains("vitae"))
                    })
                    .min_by(|a, b| {
                        a.bbox
                            .0
                            .partial_cmp(&b.bbox.0)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                match cv_heading {
                    Some(s) => {
                        let fam = normalize_family(&s.font_name);
                        if fam == ch_fam && (s.font_size - ch_sz).abs() <= 1.5 {
                            CheckResult {
                                check_id: String::new(),
                                status: Status::Pass,
                                evidence: vec![],
                                detail: format!(
                                    "CV heading matches chapter style ({}, {:.0}pt)",
                                    ch_fam, ch_sz
                                ),
                            }
                        } else {
                            CheckResult {
                                check_id: String::new(),
                                status: Status::Fail,
                                detail: format!(
                                    "CV heading font differs: {} {:.0}pt (chapters: {} {:.0}pt)",
                                    s.font_name, s.font_size, ch_fam, ch_sz
                                ),
                                evidence: vec![EvidenceItem {
                                    page: p.page_number,
                                    bbox: Some(s.bbox),
                                    excerpt: Some(s.text.clone()),
                                }],
                            }
                        }
                    }
                    None => CheckResult {
                        check_id: String::new(),
                        status: Status::Error,
                        evidence: vec![],
                        detail: "CV heading not found".to_string(),
                    },
                }
            }
            None => CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "No CV page detected (optional)".to_string(),
            },
        }
    }
}

// ── 7.8b: cv_name_position ───────────────────────────────────────

pub struct CvNamePositionChecker;

impl Checker for CvNamePositionChecker {
    fn category(&self) -> &'static str {
        "structure"
    }
    fn name(&self) -> &'static str {
        "cv_name_position"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let cv_page = doc.pages.iter().rev().find(|p| {
            let text: String = p
                .spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let low = text.to_lowercase();
            low.contains("curriculum vitae")
                || (low.contains("curriculum") && low.contains("vitae"))
        });

        let page = match cv_page {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Pass,
                    evidence: vec![],
                    detail: "No CV page detected (optional)".to_string(),
                }
            }
        };

        if page.spans.len() < 2 {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "Not enough text on CV page".to_string(),
            };
        }

        let mut non_empty: Vec<&crate::document::TextSpan> = page
            .spans
            .iter()
            .filter(|s| {
                let (top, _bottom, _x0, _x1) = s.bbox;
                !s.text.trim().is_empty() && top >= 72.0
            })
            .collect();
        non_empty.sort_by(|a, b| {
            a.bbox
                .0
                .partial_cmp(&b.bbox.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let first_top = non_empty[0].bbox.0;
        let name_spans: Vec<&&crate::document::TextSpan> = non_empty
            .iter()
            .filter(|s| (s.bbox.0 - first_top).abs() < 6.0)
            .collect();

        let center = line_center(&name_spans);
        let page_cx = page_center(page);
        let left_margin = 90.0;
        let tolerance = 36.0;

        let is_centered = (center - page_cx).abs() <= tolerance;
        let is_left = (center - left_margin).abs() <= tolerance || center < page_cx - tolerance;

        if is_centered || is_left {
            let pos = if is_centered {
                "centered"
            } else {
                "left-aligned"
            };
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!("CV name is {}", pos),
            }
        } else {
            let name_text: String = name_spans
                .iter()
                .map(|s| s.text.trim())
                .collect::<Vec<_>>()
                .join(" ");
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!("CV name not centered or left-aligned: \"{}\"", name_text),
                evidence: name_spans
                    .iter()
                    .map(|s| EvidenceItem {
                        page: page.page_number,
                        bbox: Some(s.bbox),
                        excerpt: Some(s.text.clone()),
                    })
                    .collect(),
            }
        }
    }
}

// ── 7.9a: abstract_text_centered ─────────────────────────────────

pub struct AbstractTextCenteredChecker;

impl Checker for AbstractTextCenteredChecker {
    fn category(&self) -> &'static str {
        "typography"
    }
    fn name(&self) -> &'static str {
        "abstract_text_centered"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let page = match find_abstract_page(doc) {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "Abstract page not found".to_string(),
                }
            }
        };

        let pc = page_center(page);
        let tolerance = 36.0;

        let non_empty: Vec<&crate::document::TextSpan> = page
            .spans
            .iter()
            .filter(|s| {
                !s.text.trim().is_empty() && s.bbox.0 >= 72.0 && s.bbox.1 <= page.height - 72.0
            })
            .collect();

        if non_empty.len() < 2 {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "Not enough text on abstract page".to_string(),
            };
        }

        let body_sort: Vec<(i32, f32)> = {
            let mut tops: Vec<(i32, f32)> = non_empty
                .iter()
                .map(|s| (s.bbox.0.round() as i32, s.font_size))
                .collect();
            tops.sort_by_key(|(t, _)| *t);
            tops.dedup_by_key(|(t, _)| *t);
            tops
        };

        if body_sort.len() < 2 {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "Not enough lines on abstract page".to_string(),
            };
        }

        let first_top = body_sort[0].0 as f32;
        let second_top = body_sort[1].0 as f32;
        let name_lines: Vec<i32> = {
            let mut group: std::collections::BTreeMap<i32, Vec<&&crate::document::TextSpan>> =
                std::collections::BTreeMap::new();
            for s in &non_empty {
                let tk = s.bbox.0.round() as i32;
                if (tk as f32 - first_top).abs() < 6.0 {
                    group.entry(tk).or_default().push(s);
                }
            }
            let mut ks: Vec<i32> = group.keys().copied().collect();
            ks.sort();
            ks
        };

        let title_lines: Vec<i32> = {
            let mut group: std::collections::BTreeMap<i32, Vec<&&crate::document::TextSpan>> =
                std::collections::BTreeMap::new();
            for s in &non_empty {
                let tk = s.bbox.0.round() as i32;
                if (tk as f32 - second_top).abs() < 6.0 {
                    group.entry(tk).or_default().push(s);
                }
            }
            let mut ks: Vec<i32> = group.keys().copied().collect();
            ks.sort();
            ks
        };

        let mut violations: Vec<EvidenceItem> = Vec::new();

        for top in &name_lines {
            let spans: Vec<&&crate::document::TextSpan> = non_empty
                .iter()
                .filter(|s| (s.bbox.0.round() as i32 - top).abs() < 3)
                .collect();
            if spans.is_empty() {
                continue;
            }
            let cx = line_center(&spans);
            if (cx - pc).abs() > tolerance {
                violations.push(EvidenceItem {
                    page: page.page_number,
                    bbox: Some((*top as f32, *top as f32 + 12.0, 0.0, 0.0)),
                    excerpt: Some(format!("Name line off-center by {:.0}pt", (cx - pc).abs())),
                });
            }
        }

        for top in &title_lines {
            let spans: Vec<&&crate::document::TextSpan> = non_empty
                .iter()
                .filter(|s| (s.bbox.0.round() as i32 - top).abs() < 3)
                .collect();
            if spans.is_empty() {
                continue;
            }
            let cx = line_center(&spans);
            if (cx - pc).abs() > tolerance {
                violations.push(EvidenceItem {
                    page: page.page_number,
                    bbox: Some((*top as f32, *top as f32 + 12.0, 0.0, 0.0)),
                    excerpt: Some(format!("Title line off-center by {:.0}pt", (cx - pc).abs())),
                });
            }
        }

        if violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "Abstract name and title centered".to_string(),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!("{}/2 header lines not centered", violations.len()),
                evidence: violations,
            }
        }
    }
}

// ── 7.9b: abstract_word_count ────────────────────────────────────

pub struct AbstractWordCountChecker;

impl Checker for AbstractWordCountChecker {
    fn category(&self) -> &'static str {
        "content"
    }
    fn name(&self) -> &'static str {
        "abstract_word_count"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let page = match find_abstract_page(doc) {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "Abstract page not found".to_string(),
                }
            }
        };

        let words: Vec<&str> = page
            .spans
            .iter()
            .filter(|s| {
                let (top, bottom, _x0, _x1) = s.bbox;
                !s.text.trim().is_empty() && top >= 72.0 && bottom <= page.height - 72.0
            })
            .flat_map(|s| s.text.split_whitespace())
            .collect();

        let limit = 350;
        if words.len() <= limit {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!("Abstract: {} words (limit {})", words.len(), limit),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "Abstract: {} words exceeds {} word limit",
                    words.len(),
                    limit
                ),
                evidence: vec![EvidenceItem {
                    page: page.page_number,
                    bbox: None,
                    excerpt: Some(format!("{} words", words.len())),
                }],
            }
        }
    }
}

// ── 7.9c: abstract_title_format ──────────────────────────────────

pub struct AbstractTitleFormatChecker;

impl Checker for AbstractTitleFormatChecker {
    fn category(&self) -> &'static str {
        "typography"
    }
    fn name(&self) -> &'static str {
        "abstract_title_format"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let page = match find_abstract_page(doc) {
            Some(p) => p,
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    evidence: vec![],
                    detail: "Abstract page not found".to_string(),
                }
            }
        };

        let non_empty: Vec<&crate::document::TextSpan> = page
            .spans
            .iter()
            .filter(|s| {
                !s.text.trim().is_empty() && s.bbox.0 >= 72.0 && s.bbox.1 <= page.height - 72.0
            })
            .collect();

        if non_empty.len() < 2 {
            return CheckResult {
                check_id: String::new(),
                status: Status::Error,
                evidence: vec![],
                detail: "Not enough text on abstract page".to_string(),
            };
        }

        let mut tops: Vec<(i32, f32)> = non_empty
            .iter()
            .map(|s| (s.bbox.0.round() as i32, s.font_size))
            .collect();
        tops.sort_by_key(|(t, _)| *t);
        tops.dedup_by_key(|(t, _)| *t);

        let second_top = if tops.len() > 1 {
            tops[1].0 as f32
        } else {
            tops[0].0 as f32
        };
        let title_spans: Vec<&&crate::document::TextSpan> = non_empty
            .iter()
            .filter(|s| (s.bbox.0 - second_top).abs() < 6.0)
            .collect();

        let title_text: String = title_spans
            .iter()
            .map(|s| s.text.trim())
            .collect::<Vec<_>>()
            .join(" ");
        let alpha: Vec<char> = title_text.chars().filter(|c| c.is_alphabetic()).collect();

        if alpha.is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "No alphabetic characters in abstract title".to_string(),
            };
        }

        let all_upper = alpha.iter().all(|c| c.is_uppercase());
        let first_letter_upper = alpha.first().is_none_or(|c| c.is_uppercase());
        let has_lower = alpha.iter().any(|c| c.is_lowercase());
        let title_case = first_letter_upper && has_lower;

        if all_upper || title_case {
            let fmt = if all_upper { "all-caps" } else { "title case" };
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!("Abstract title is {}: \"{}\"", fmt, title_text),
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "Abstract title not all-caps or title case: \"{}\"",
                    title_text
                ),
                evidence: vec![EvidenceItem {
                    page: page.page_number,
                    bbox: None,
                    excerpt: Some(title_text),
                }],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, Page};

    fn span(text: &str, top: f32, fs: f32, fam: &str) -> crate::document::TextSpan {
        crate::document::TextSpan {
            text: text.to_string(),
            font_name: fam.to_string(),
            font_size: fs,
            bbox: (top, top + fs, 100.0, 200.0),
            is_bold: false,
            is_italic: false,
            color: None,
        }
    }

    fn body_pages(count: usize) -> Vec<Page> {
        let mut pages = Vec::new();
        for i in 1..=count {
            let mut spans = Vec::new();
            for j in 0..25 {
                spans.push(span(
                    "body text line",
                    72.0 + j as f32 * 24.0,
                    12.0,
                    "TimesNewRoman",
                ));
            }
            pages.push(Page {
                page_number: i,
                width: 612.0,
                height: 792.0,
                spans,
                images: vec![],
                paths: vec![],
            });
        }
        pages
    }

    fn page_with_heading(pn: usize, spans: Vec<crate::document::TextSpan>) -> Page {
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
    fn test_references_font_pass() {
        let mut pages = body_pages(8);
        pages.push(page_with_heading(
            9,
            vec![
                span("References", 100.0, 12.0, "TimesNewRoman"),
                span("Smith, J. (2020)", 130.0, 12.0, "TimesNewRoman"),
            ],
        ));
        let doc = Document { pages };
        let r = ReferencesFontChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_references_font_fail() {
        let mut pages = body_pages(8);
        pages.push(page_with_heading(
            9,
            vec![
                span("References", 100.0, 12.0, "TimesNewRoman"),
                span("Smith, J. (2020)", 130.0, 10.0, "Arial"),
            ],
        ));
        let doc = Document { pages };
        let r = ReferencesFontChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail);
    }

    fn abstract_pages() -> Vec<Page> {
        let mut pages = body_pages(4);
        pages.push(page_with_heading(
            5,
            vec![span("Accepted by", 80.0, 12.0, "TimesNewRoman")],
        ));
        pages
    }

    #[test]
    fn test_abstract_word_count_pass() {
        let mut pages = abstract_pages();
        let mut spans = vec![span("Jane Smith", 200.0, 12.0, "TimesNewRoman")];
        for i in 0..110 {
            spans.push(span(
                "body text",
                230.0 + i as f32 * 5.0,
                12.0,
                "TimesNewRoman",
            ));
        }
        pages.push(page_with_heading(7, spans));
        pages.push(page_with_heading(
            9,
            vec![span("TABLE OF CONTENTS", 100.0, 12.0, "TimesNewRoman")],
        ));
        let doc = Document { pages };
        let r = AbstractWordCountChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass, "{}", r.detail);
    }

    #[test]
    fn test_abstract_title_format_all_caps_pass() {
        let mut pages = abstract_pages();
        let mut spans = vec![
            span("Jane Smith", 200.0, 12.0, "TimesNewRoman"),
            span("POWER AND FREEDOM", 230.0, 12.0, "TimesNewRoman"),
        ];
        for i in 0..110 {
            spans.push(span(
                "abstract body text here word",
                260.0 + i as f32 * 5.0,
                12.0,
                "TimesNewRoman",
            ));
        }
        pages.push(page_with_heading(7, spans));
        pages.push(page_with_heading(
            9,
            vec![span("TABLE OF CONTENTS", 100.0, 12.0, "TimesNewRoman")],
        ));
        let doc = Document { pages };
        let r = AbstractTitleFormatChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass, "{}", r.detail);
    }

    #[test]
    fn test_abstract_title_format_title_case_pass() {
        let mut pages = abstract_pages();
        let mut spans = vec![
            span("Jane Smith", 200.0, 12.0, "TimesNewRoman"),
            span(
                "Power and Freedom in Urban Spaces",
                230.0,
                12.0,
                "TimesNewRoman",
            ),
        ];
        for i in 0..110 {
            spans.push(span(
                "abstract body text here word",
                260.0 + i as f32 * 5.0,
                12.0,
                "TimesNewRoman",
            ));
        }
        pages.push(page_with_heading(7, spans));
        pages.push(page_with_heading(
            9,
            vec![span("TABLE OF CONTENTS", 100.0, 12.0, "TimesNewRoman")],
        ));
        let doc = Document { pages };
        let r = AbstractTitleFormatChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass, "{}", r.detail);
    }
}
