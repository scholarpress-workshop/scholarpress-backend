use pdf_oxide::annotation_types::TextMarkupType;
use pdf_oxide::editor::DocumentEditor;
use pdf_oxide::geometry::Rect;
use pdf_oxide::writer::{DocumentBuilder, PageSize, TextAnnotation, TextMarkupAnnotation};
use sp_check::checkers::Status;
use sp_check::report::Report;
use std::collections::HashSet;

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

/// Map any non-ASCII character to '?' so the summary page stays ASCII-only.
fn ascii_only(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect()
}

/// Build the ASCII text rendered on the prepended summary page:
/// counts plus every non-PASS finding that is not exclusively in-place
/// (i.e. document-level non-bbox evidence, and results with no evidence at all).
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
        let any_bbox = r.evidence.iter().any(|e| e.bbox.is_some());
        if any_bbox && doc_evidence.is_empty() {
            // All findings are in-place highlights; nothing to list here.
            continue;
        }
        count += 1;
        lines.push(format!("[{}] {}", r.status.as_str(), r.check_id));
        if !r.detail.is_empty() {
            lines.push(format!("    {}", ascii_only(&r.detail)));
        }
        for e in &doc_evidence {
            let ex = ascii_only(e.excerpt.as_deref().unwrap_or(""));
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

/// Maximum characters per wrapped summary line (9pt Helvetica on a Letter
/// page with 54pt side margins; conservative so lines stay within the page).
const SUMMARY_LINE_CHARS: usize = 90;

/// Greedily wrap an ASCII line to at most `max_chars` characters per output
/// line. Blank lines are preserved as a single empty line.
fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.trim().is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in line.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.len() + 1 + word.len() <= max_chars {
            cur.push(' ');
            cur.push_str(word);
        } else {
            out.push(std::mem::take(&mut cur));
            cur.push_str(word);
        }
        while cur.len() > max_chars {
            let tail = cur.split_off(max_chars);
            out.push(std::mem::take(&mut cur));
            cur = tail;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub(crate) fn annotate_bytes(
    input: &[u8],
    report: &Report,
) -> Result<Vec<u8>, crate::AnnotateError> {
    // 1. Annotate the source PDF in place. `edit_page`/`get_page_media_box` on a
    // `DocumentEditor` only address its own source pages, not pages appended by
    // `merge_from_bytes`, so the source must be annotated before it is merged.
    let mut editor = DocumentEditor::from_bytes(input.to_vec())?;
    let page_count = editor.current_page_count();
    let mut media_boxes = vec![[0.0f32; 4]; page_count];
    for (i, mb) in media_boxes.iter_mut().enumerate() {
        *mb = editor.get_page_media_box(i)?;
    }

    let mut seen: HashSet<(String, usize, i32, i32, i32, i32)> = HashSet::new();
    for result in &report.results {
        if result.status == Status::Pass {
            continue;
        }
        for ev in &result.evidence {
            let Some(bbox) = ev.bbox else { continue };
            let page_idx = ev.page; // 1-based
            if page_idx == 0 || page_idx > page_count {
                continue;
            }
            // Deduplicate identical (check_id, page, bbox) findings so a
            // checker that reports the same region twice gets one highlight.
            let key = (
                result.check_id.clone(),
                page_idx,
                bbox.0.round() as i32,
                bbox.1.round() as i32,
                bbox.2.round() as i32,
                bbox.3.round() as i32,
            );
            if !seen.insert(key) {
                continue;
            }
            let mb = media_boxes[page_idx - 1];
            let h = mb[3] - mb[1];
            let rect = bbox_to_rect(bbox, h);
            let rect = if rect.width < 1.0 {
                Rect::new(mb[0], rect.y, mb[2] - mb[0], rect.height)
            } else {
                rect
            };
            let note_rect = Rect::new(rect.x, (rect.y + rect.height - 14.0).max(0.0), 14.0, 14.0);
            let contents = note_contents(
                &result.check_id,
                result.status.as_str(),
                &result.detail,
                ev.excerpt.as_deref(),
            );
            editor.edit_page(page_idx - 1, |page| {
                page.add_annotation(
                    TextMarkupAnnotation::from_rect(TextMarkupType::Highlight, rect)
                        .with_color(1.0, 1.0, 0.0)
                        .with_opacity(0.4)
                        .with_author("scholarpress"),
                );
                page.add_annotation(
                    TextAnnotation::new(note_rect, contents).with_author("scholarpress"),
                );
                Ok(())
            })?;
        }
    }
    let annotated = editor.save_to_bytes()?;

    // 2. Build a one-page summary PDF.
    let summary = summary_text(report);
    let mut builder = DocumentBuilder::new();
    {
        let mut page = builder.page(PageSize::Letter);
        page = page.font("Helvetica", 9.0).at(54.0, 720.0);
        for line in summary.lines() {
            for wrapped in wrap_line(line, SUMMARY_LINE_CHARS) {
                if page.remaining_space() < 14.0 {
                    page = page.new_page_same_size().at(54.0, 720.0);
                }
                page = page.text(&wrapped).newline();
            }
        }
        page.done();
    }
    let summary_bytes = builder.build()?;

    // 3. Prepend the summary page, then append the annotated source pages.
    let mut out = DocumentEditor::from_bytes(summary_bytes)?;
    out.merge_from_bytes(&annotated)?;
    out.save_to_bytes().map_err(Into::into)
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
                evidence: vec![
                    EvidenceItem {
                        page: 1,
                        bbox: Some((100.0, 120.0, 200.0, 300.0)),
                        excerpt: Some("1 inch".to_string()),
                    },
                    EvidenceItem {
                        page: 1,
                        bbox: Some((100.0, 120.0, 200.0, 300.0)),
                        excerpt: Some("1 inch (duplicate)".to_string()),
                    },
                ],
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
        let s = note_contents(
            "global_margins",
            "FAIL",
            "margins too small",
            Some("1 inch"),
        );
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
        assert!(
            !text.contains("test_inplace"),
            "in-place finding must not be duplicated"
        );
    }

    #[test]
    fn summary_text_lists_no_evidence_results() {
        let report = build_report(vec![CheckResult {
            check_id: "no_evidence".to_string(),
            status: Status::Error,
            detail: "checker found nothing to measure".to_string(),
            evidence: vec![],
        }]);
        let text = summary_text(&report);
        assert!(
            text.contains("[ERROR] no_evidence"),
            "no-evidence results must appear in the summary, not be silently dropped"
        );
        assert!(text.contains("checker found nothing to measure"));
    }

    #[test]
    fn wrap_line_wraps_and_preserves_blanks() {
        assert_eq!(wrap_line("", 90), vec![""]);
        assert_eq!(wrap_line("   ", 90), vec![""]);
        assert_eq!(wrap_line("short", 90), vec!["short"]);
        assert_eq!(
            wrap_line("the quick brown fox jumps", 12),
            vec!["the quick", "brown fox", "jumps"]
        );
        // single overlong word hard-breaks
        assert_eq!(wrap_line("abcdefghijklm", 5), vec!["abcde", "fghij", "klm"]);
    }

    #[test]
    fn annotate_bytes_prepends_summary_and_adds_annotations() {
        // Locate the IU baseline fixture, skipping if absent (matches sp-mcp pattern).
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let catalog = (0..6)
            .map(|i| {
                let mut p = manifest.clone();
                for _ in 0..=i {
                    p.pop();
                }
                p.join("scholarpress-catalog")
            })
            .find(|p| p.is_dir());
        let baseline = match catalog {
            Some(c) => c.join("institutions/iu-indianapolis/tests/fixtures/baseline.pdf"),
            None => {
                eprintln!("SKIP: scholarpress-catalog not found");
                return;
            }
        };
        let input = match std::fs::read(&baseline) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("SKIP: baseline.pdf fixture not present (run bash compile.sh)");
                return;
            }
        };

        let original_pages = pdf_oxide::PdfDocument::from_bytes(input.clone())
            .expect("open input")
            .page_count()
            .expect("page count");

        let out = annotate_bytes(&input, &sample_report()).expect("annotate should succeed");
        assert!(out.starts_with(b"%PDF"));

        let doc = pdf_oxide::PdfDocument::from_bytes(out).expect("reopen output");
        assert_eq!(doc.page_count().expect("pages"), original_pages + 1);

        let first = doc.extract_text(0).expect("summary text");
        assert!(
            first.contains("Annotated Summary"),
            "page 0 should be the summary page"
        );

        // In-place finding on page 1 (1-based) lives at output index 1.
        let annots = doc.get_annotations(1).expect("annotations on page 1");
        assert!(!annots.is_empty(), "expected annotations on page 1");
        let highlight_count = annots
            .iter()
            .filter(|a| {
                matches!(
                    a.subtype_enum,
                    pdf_oxide::annotation_types::AnnotationSubtype::Highlight
                )
            })
            .count();
        assert_eq!(
            highlight_count, 1,
            "duplicate evidence for the same bbox must collapse to one highlight"
        );
        assert!(
            annots.iter().any(|a| matches!(
                a.subtype_enum,
                pdf_oxide::annotation_types::AnnotationSubtype::Highlight
            )),
            "expected a highlight annotation"
        );
        assert!(
            annots.iter().any(|a| matches!(
                a.subtype_enum,
                pdf_oxide::annotation_types::AnnotationSubtype::Text
            )),
            "expected a text (sticky note) annotation"
        );
    }
}
