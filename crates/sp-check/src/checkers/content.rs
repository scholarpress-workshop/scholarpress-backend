use crate::checkers::{CheckResult, Checker, EvidenceItem, Status};
use regex::Regex;
use serde_yaml::Value;
use sp_extract::document::ParsedDocument as Document;
use std::collections::BTreeMap;
use std::sync::LazyLock;

static WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static VAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\\{(\w+)\\\}").unwrap());
static DEGREE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)ph\.?\s*d\.?|m\.\s*p\.\s*a\.?|m\.\s*a\.?|m\.\s*s\.?|j\.?\s*d\.?|ed\.?\s*d\.?")
        .unwrap()
});
static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(january|february|march|april|may|june|july|august|september|october|november|december)\s+\d{1,2},?\s+\d{4}$").unwrap()
});
static SKIP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[_\-\s]+$").unwrap());
static TOC_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.+?)\.{2,}\s*(\d+)\s*$").unwrap());
static CHAPTER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)chapter\s+\d+").unwrap());
static APPENDIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)appendix\s+[a-z]").unwrap());
static DASH_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[–—\-—]+").unwrap());
static NONALNUM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-z0-9\s\-:]").unwrap());

fn page_lines(page: &sp_extract::document::ParsedPage) -> Vec<String> {
    let mut lines: BTreeMap<i32, Vec<&sp_extract::document::TextSpan>> = BTreeMap::new();
    for s in &page.spans {
        if !s.text.trim().is_empty() {
            let top_key = s.bbox.0.round() as i32;
            lines.entry(top_key).or_default().push(s);
        }
    }
    let mut result: Vec<String> = Vec::new();
    for spans in lines.values() {
        let mut sorted = spans.clone();
        sorted.sort_by(|a, b| {
            a.bbox
                .2
                .partial_cmp(&b.bbox.2)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let line: String = sorted
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        result.push(line.trim().to_string());
    }
    result
}

fn normalize(s: &str) -> String {
    WS_RE.replace_all(s, " ").trim().to_lowercase()
}

fn line_matches(template_line: &str, page_line: &str) -> bool {
    let cleaned = template_line.trim_end_matches([',', '.', ';', ':']);
    let escaped = regex::escape(cleaned);
    let pattern_str = VAR_RE.replace_all(&escaped, "(.+)");
    let full_pattern = format!("^{}[,.;:]?$", pattern_str);
    if let Ok(re) = Regex::new(&full_pattern) {
        re.is_match(page_line)
    } else {
        false
    }
}

fn line_matches_multi(template_line: &str, page_lines: &[String], start: usize) -> (bool, usize) {
    for n in 1..4.min(page_lines.len() - start + 1) {
        let joined: String = page_lines[start..start + n].join(" ");
        if line_matches(template_line, &joined) {
            return (true, n);
        }
    }
    (false, 1)
}

fn match_template(template_lines: &[String], page_lines: &[String]) -> usize {
    let mut ti = 0usize;
    let mut pi = 0usize;
    let mut matched_count = 0usize;
    while ti < template_lines.len() && pi < page_lines.len() {
        let (matched, consumed) = line_matches_multi(&template_lines[ti], page_lines, pi);
        if matched {
            ti += 1;
            pi += consumed;
            matched_count += 1;
        } else {
            pi += 1;
        }
    }
    matched_count
}

pub struct BoilerplateMatchChecker;

impl Checker for BoilerplateMatchChecker {
    fn category(&self) -> &'static str {
        "content"
    }

    fn name(&self) -> &'static str {
        "boilerplate_match"
    }

    fn check(&self, doc: &Document, params: &Value) -> CheckResult {
        let template = params
            .get("template")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let page_filter = params
            .get("page")
            .and_then(|v| v.as_u64())
            .map(|p| p as usize);

        if template.trim().is_empty() {
            return CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: "No template provided".to_string(),
            };
        }

        let template_lines: Vec<String> = template
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(normalize)
            .collect();

        let mut best_match = 0usize;
        let mut violations: Vec<EvidenceItem> = Vec::new();

        for page in &doc.pages {
            if let Some(target) = page_filter {
                if page.page_number != target {
                    continue;
                }
            }

            let page_lines = page_lines(page);
            let normed: Vec<String> = page_lines.iter().map(|l| normalize(l)).collect();

            let matched = match_template(&template_lines, &normed);
            if matched > best_match {
                best_match = matched;
            }
            if matched == template_lines.len() {
                break;
            }
        }

        let ratio = best_match as f32 / template_lines.len() as f32;
        let threshold = 0.7;

        if ratio < threshold {
            violations.push(EvidenceItem {
                page: page_filter.unwrap_or(0),
                bbox: None,
                excerpt: Some(format!(
                    "Only {}/{} template lines matched ({:.0}%)",
                    best_match,
                    template_lines.len(),
                    ratio * 100.0
                )),
            });
        }

        if !violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "Template text not found on specified page ({}/{} lines, {:.0}%)",
                    best_match,
                    template_lines.len(),
                    ratio * 100.0
                ),
                evidence: violations,
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!(
                    "Template text matches ({}/{} lines)",
                    best_match,
                    template_lines.len()
                ),
            }
        }
    }
}

fn find_committee(page: &sp_extract::document::ParsedPage) -> Vec<(String, bool)> {
    let lines = page_lines(page);
    let mut committee: Vec<(String, bool)> = Vec::new();
    let mut in_committee = false;

    for line in &lines {
        let low = line.to_lowercase();
        if low.contains("doctoral committee") || low.contains("committee") {
            in_committee = true;
            continue;
        }

        if !in_committee {
            continue;
        }

        if low.contains("date of defense") || low.contains("defense date") {
            break;
        }

        if SKIP_RE.is_match(line) {
            continue;
        }

        if DATE_RE.is_match(&low) {
            continue;
        }

        let is_name = DEGREE_RE.is_match(&low);
        if !is_name {
            if !committee.is_empty() && !SKIP_RE.is_match(line) {
                break;
            }
            continue;
        }

        let is_chair = low.contains("chair");
        committee.push((line.clone(), is_chair));
    }

    committee
}

pub struct CommitteeOrderChecker;

impl Checker for CommitteeOrderChecker {
    fn category(&self) -> &'static str {
        "content"
    }

    fn name(&self) -> &'static str {
        "committee_order"
    }

    fn check(&self, doc: &Document, params: &Value) -> CheckResult {
        let chair_first = params
            .get("chair_first")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let page_filter = params
            .get("page")
            .and_then(|v| v.as_u64())
            .map(|p| p as usize);

        for page in &doc.pages {
            if let Some(target) = page_filter {
                if page.page_number != target {
                    continue;
                }
            }

            let committee = find_committee(page);
            if committee.is_empty() {
                continue;
            }

            if chair_first {
                let mut violations: Vec<EvidenceItem> = Vec::new();
                for (i, (name, is_chair)) in committee.iter().enumerate() {
                    if *is_chair && i > 0 {
                        violations.push(EvidenceItem {
                            page: page.page_number,
                            bbox: None,
                            excerpt: Some(format!(
                                "Chair '{}' listed at position {}, should be first",
                                name,
                                i + 1,
                            )),
                        });
                    }
                }

                if !violations.is_empty() {
                    return CheckResult {
                        check_id: String::new(),
                        status: Status::Fail,
                        detail: "Committee chair not listed first".to_string(),
                        evidence: violations,
                    };
                }
            }

            if !committee.iter().any(|(_, is_chair)| *is_chair) {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Fail,
                    detail: "Chair not explicitly labeled".to_string(),
                    evidence: vec![EvidenceItem {
                        page: page.page_number,
                        bbox: None,
                        excerpt: Some(
                            "Chair label missing — IU requires 'Chair' after chair's degrees"
                                .to_string(),
                        ),
                    }],
                };
            }

            return CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!("Committee chair listed first ({} members)", committee.len(),),
            };
        }

        CheckResult {
            check_id: String::new(),
            status: Status::Error,
            detail: "Committee not found on specified page".to_string(),
            evidence: vec![],
        }
    }
}

fn extract_toc_entries(page: &sp_extract::document::ParsedPage) -> Vec<(String, usize)> {
    let mut lines: BTreeMap<i32, Vec<&sp_extract::document::TextSpan>> = BTreeMap::new();
    for s in &page.spans {
        if !s.text.trim().is_empty() {
            let top_key = s.bbox.0.round() as i32;
            lines.entry(top_key).or_default().push(s);
        }
    }

    let mut entries: Vec<(String, usize)> = Vec::new();

    for spans in lines.values() {
        let mut sorted = spans.clone();
        sorted.sort_by(|a, b| {
            a.bbox
                .2
                .partial_cmp(&b.bbox.2)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let text: String = sorted
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let text = text.trim();
        if let Some(caps) = TOC_RE.captures(text) {
            let title = caps.get(1).unwrap().as_str().trim().to_string();
            if let Ok(pg) = caps.get(2).unwrap().as_str().parse::<usize>() {
                entries.push((title, pg));
            }
        }
    }
    entries
}

fn extract_page_heading(page: &sp_extract::document::ParsedPage) -> String {
    let mut body: Vec<&sp_extract::document::TextSpan> = page
        .spans
        .iter()
        .filter(|s| {
            let (top, bottom, _x0, _x1) = s.bbox;
            !s.text.trim().is_empty() && top >= 36.0 && bottom <= page.height - 36.0
        })
        .collect();

    body.sort_by(|a, b| {
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

    let full_text: String = body
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let low = full_text.to_lowercase();

    let extract = |keyword: &str| -> Option<String> {
        let pos = low.find(keyword)?;
        let heading: String = full_text[pos..].chars().take(80).collect();
        Some(heading.trim().to_string())
    };

    if let Some(h) = extract("chapter ") {
        return h;
    }
    if let Some(h) = extract("appendix ") {
        return h;
    }
    String::new()
}

fn contains_chapter_keyword(title: &str) -> bool {
    let low = title.to_lowercase();
    CHAPTER_RE.is_match(&low) || APPENDIX_RE.is_match(&low)
}

fn normalize_title(title: &str) -> String {
    let t = WS_RE.replace_all(title, " ").trim().to_lowercase();
    let t = DASH_RE.replace_all(&t, "-").to_string();
    NONALNUM_RE.replace_all(&t, "").to_string()
}

fn titles_match_norm(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a.len() > b.len() && a.contains(b) {
        return true;
    }
    if b.len() >= a.len() && b.contains(a) {
        return true;
    }

    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if a_words.is_empty() || b_words.is_empty() {
        return false;
    }
    let common = a_words.intersection(&b_words).count();
    let min_len = a_words.len().min(b_words.len());
    common as f32 >= min_len as f32 * 0.7
}

pub struct TocTitleParityChecker;

impl Checker for TocTitleParityChecker {
    fn category(&self) -> &'static str {
        "content"
    }

    fn name(&self) -> &'static str {
        "toc_title_parity"
    }

    fn check(&self, doc: &Document, _params: &Value) -> CheckResult {
        let mut toc_page_index: Option<usize> = None;
        for (i, page) in doc.pages.iter().enumerate() {
            let text: String = page
                .spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            if text.to_lowercase().contains("table of contents") {
                toc_page_index = Some(i);
                break;
            }
        }

        let toc_page = match toc_page_index {
            Some(i) => &doc.pages[i],
            None => {
                return CheckResult {
                    check_id: String::new(),
                    status: Status::Error,
                    detail: "TOC page not found".to_string(),
                    evidence: vec![],
                };
            }
        };

        let entries = extract_toc_entries(toc_page);
        let toc_page_num = toc_page.page_number;

        let mut violations: Vec<EvidenceItem> = Vec::new();
        let mut checked = 0usize;

        for (entry_title, _entry_pg) in &entries {
            if !contains_chapter_keyword(entry_title) {
                continue;
            }

            let entry_norm = normalize_title(entry_title);
            let mut found = false;

            for page in &doc.pages {
                if page.page_number == toc_page_num {
                    continue;
                }
                let body_heading = extract_page_heading(page);
                let body_norm = normalize_title(&body_heading);
                if titles_match_norm(&entry_norm, &body_norm) {
                    found = true;
                    break;
                }
            }

            checked += 1;
            if !found {
                let excerpt = format!(
                    "TOC: \"{}\" — no matching heading found in body",
                    if entry_title.len() > 60 {
                        &entry_title[..60]
                    } else {
                        entry_title
                    },
                );
                violations.push(EvidenceItem {
                    page: 0,
                    bbox: None,
                    excerpt: Some(excerpt),
                });
            }
        }

        if !violations.is_empty() {
            CheckResult {
                check_id: String::new(),
                status: Status::Fail,
                detail: format!(
                    "{}/{} chapter title(s) mismatch between TOC and body",
                    violations.len(),
                    checked,
                ),
                evidence: violations,
            }
        } else {
            CheckResult {
                check_id: String::new(),
                status: Status::Pass,
                evidence: vec![],
                detail: format!("All {} chapter titles match between TOC and body", checked,),
            }
        }
    }
}

pub struct HumanReviewChecker;

impl Checker for HumanReviewChecker {
    fn category(&self) -> &'static str {
        "human"
    }

    fn name(&self) -> &'static str {
        "review"
    }

    fn check(&self, _doc: &Document, params: &Value) -> CheckResult {
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("Manual review required");
        CheckResult {
            check_id: String::new(),
            status: Status::Manual,
            evidence: vec![],
            detail: prompt.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_extract::document::{ParsedDocument as Document, ParsedPage as Page, TextSpan};

    fn span(text: &str, top: f32, x0: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            font_name: "Times".to_string(),
            font_size: 12.0,
            bbox: (top, top + 12.0, x0, x0 + text.len() as f32 * 5.0),
            is_bold: false,
            is_italic: false,
            color: None,
        }
    }

    #[test]
    fn test_boilerplate_match_pass() {
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
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![
                    span("Submitted to the faculty", 100.0, 100.0),
                    span("in partial fulfillment", 114.0, 100.0),
                    span("for the degree", 128.0, 100.0),
                    span("Doctor of Philosophy", 142.0, 100.0),
                    span("in the department,", 156.0, 100.0),
                    span("Indiana University", 170.0, 100.0),
                    span("May 2025", 184.0, 100.0),
                ],
                images: vec![],
                paths: vec![],
            }],
        };
        let params: Value = serde_yaml::from_str("template: |\n  Submitted to the faculty\n  in partial fulfillment\n  for the degree\n  {degree}\n  in the {department},\n  Indiana University\n  {month} {year}\npage: 1\n").unwrap();
        let r = BoilerplateMatchChecker.check(&doc, &params);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_boilerplate_match_fail() {
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
                page_number: 1,
                width: 612.0,
                height: 792.0,
                spans: vec![span("Something else", 100.0, 100.0)],
                images: vec![],
                paths: vec![],
            }],
        };
        let params: Value =
            serde_yaml::from_str("template: |\n  Submitted to the faculty\npage: 1\n").unwrap();
        let r = BoilerplateMatchChecker.check(&doc, &params);
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn test_boilerplate_empty_template_pass() {
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
        let r = BoilerplateMatchChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Pass);
    }

    #[test]
    fn test_human_review_manual() {
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
        let params: Value = serde_yaml::from_str("prompt: Check this\n").unwrap();
        let r = HumanReviewChecker.check(&doc, &params);
        assert_eq!(r.status, Status::Manual);
        assert_eq!(r.detail, "Check this");
    }

    #[test]
    fn test_normalize_title() {
        let t = normalize_title("Chapter 1: Power and Freedom");
        assert_eq!(t, "chapter 1: power and freedom");
    }

    #[test]
    fn test_titles_match_exact() {
        assert!(titles_match_norm("chapter 1 intro", "chapter 1 intro"));
    }

    #[test]
    fn test_titles_match_contains() {
        assert!(titles_match_norm(
            "chapter 1 power",
            "chapter 1 power and freedom in urban spaces"
        ));
    }

    #[test]
    fn test_titles_match_overlap() {
        assert!(titles_match_norm(
            "chapter 1 power freedom urban",
            "chapter 1 power and freedom"
        ));
    }

    #[test]
    fn test_titles_no_match() {
        assert!(!titles_match_norm("chapter 1 power", "chapter 2 methods"));
    }

    #[test]
    fn test_committee_chair_not_first() {
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
                spans: vec![
                    span("Doctoral Committee", 100.0, 100.0),
                    span("Jane Smith, Ph.D.", 140.0, 100.0),
                    span("John Doe, Ph.D., Chair", 180.0, 100.0),
                ],
                images: vec![],
                paths: vec![],
            }],
        };
        let r = CommitteeOrderChecker.check(&doc, &Value::Null);
        assert_eq!(r.status, Status::Fail);
    }
}
