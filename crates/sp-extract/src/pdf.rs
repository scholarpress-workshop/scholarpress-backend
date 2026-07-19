use crate::document::*;
use pdf_oxide::layout::{FontWeight, TextChar};
use pdf_oxide::PdfDocument;

pub fn extract_pdf(bytes: &[u8]) -> Result<ParsedDocument, Box<dyn std::error::Error>> {
    let doc = PdfDocument::from_bytes(bytes.to_vec())?;
    let page_count = doc.page_count()?;

    let mut pages = Vec::new();
    let mut all_paragraphs = Vec::new();
    let mut all_fonts = std::collections::BTreeSet::new();

    for page_idx in 0..page_count {
        let (llx, _lly, urx, ury) = doc.get_page_media_box(page_idx)?;
        let width = urx - llx;
        let height = ury - _lly;

        let chars: Vec<TextChar> = doc.extract_chars(page_idx)?;
        let spans = build_spans(&chars, height);

        let images: Vec<(f32, f32, f32, f32)> = match doc.extract_images(page_idx) {
            Ok(imgs) => imgs
                .iter()
                .filter_map(|img| {
                    let bbox = img.bbox()?;
                    let img_top = height - (bbox.y + bbox.height);
                    let img_bottom = height - bbox.y;
                    let img_x0 = bbox.x;
                    let img_x1 = bbox.x + bbox.width;
                    Some((img_top.max(0.0), img_bottom, img_x0, img_x1))
                })
                .collect(),
            Err(_) => Vec::new(),
        };

        let paths: Vec<(f32, f32, f32, f32)> = match doc.extract_paths(page_idx) {
            Ok(ps) => ps
                .iter()
                .map(|p| {
                    let path_top = height - (p.bbox.y + p.bbox.height);
                    let path_bottom = height - p.bbox.y;
                    let path_x0 = p.bbox.x;
                    let path_x1 = p.bbox.x + p.bbox.width;
                    (path_top.max(0.0), path_bottom, path_x0, path_x1)
                })
                .collect(),
            Err(_) => Vec::new(),
        };

        for span in &spans {
            all_fonts.insert(span.font_name.clone());
        }

        let paragraphs = spans_to_paragraphs(&spans, page_idx + 1);
        let page_text: String = paragraphs
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        all_paragraphs.extend(paragraphs);

        pages.push(ParsedPage {
            page_number: page_idx + 1,
            text: page_text,
            width,
            height,
            spans,
            images,
            paths,
        });
    }

    let raw_text: String = pages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(ParsedDocument {
        raw_text,
        pages,
        paragraphs: all_paragraphs,
        headings: Vec::new(),
        markdown_text: None,
        metadata: ParsedMetadata {
            title: None,
            author: None,
            page_count,
            page_count_estimated: false,
            detected_fonts: all_fonts.into_iter().collect(),
        },
    })
}

fn build_spans(chars: &[TextChar], page_height: f32) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    if chars.is_empty() {
        return spans;
    }

    let char_refs: Vec<&TextChar> = chars.iter().collect();
    let mut current_word = Vec::new();
    let mut last: Option<&&TextChar> = None;

    for ch in &char_refs {
        if let Some(last_ch) = last {
            let y_delta = (ch.origin_y - last_ch.origin_y).abs();
            let gap = ch.bbox.x - (last_ch.bbox.x + last_ch.bbox.width);
            let font_changed = ch.font_name != last_ch.font_name;
            let size_delta = (ch.font_size - last_ch.font_size).abs();

            let is_new_line = y_delta > 3.0;
            let is_word_break = gap > 20.0 || font_changed || size_delta > 1.0;
            let should_flush = is_new_line || is_word_break;
            if should_flush && !current_word.is_empty() {
                spans.push(build_word_span(&current_word, page_height));
                current_word.clear();
            }
        }
        current_word.push(*ch);
        last = Some(ch);
    }

    if !current_word.is_empty() {
        spans.push(build_word_span(&current_word, page_height));
    }

    spans
}

fn build_word_span(chars: &[&TextChar], page_height: f32) -> TextSpan {
    let first = chars[0];
    let text: String = chars
        .iter()
        .map(|c| c.char.to_string())
        .collect::<Vec<_>>()
        .join("");
    let max_y = chars
        .iter()
        .map(|c| c.origin_y)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = chars
        .iter()
        .map(|c| c.origin_y - c.font_size)
        .fold(f32::INFINITY, f32::min);
    let min_x = chars.iter().map(|c| c.bbox.x).fold(f32::INFINITY, f32::min);
    let max_x = chars
        .iter()
        .map(|c| c.bbox.x + c.bbox.width)
        .fold(f32::NEG_INFINITY, f32::max);

    TextSpan {
        text,
        font_name: first.font_name.clone(),
        font_size: first.font_size,
        bbox: (
            (page_height - max_y).max(0.0),
            (page_height - min_y).max(0.0),
            min_x.max(0.0),
            max_x.max(0.0),
        ),
        is_bold: matches!(first.font_weight, FontWeight::Bold),
        is_italic: first.is_italic,
        color: Some((first.color.r, first.color.g, first.color.b)),
    }
}

fn spans_to_paragraphs(spans: &[TextSpan], page_number: usize) -> Vec<ParsedParagraph> {
    let mut paragraphs = Vec::new();
    if spans.is_empty() {
        return paragraphs;
    }

    let line_heights: Vec<f32> = spans
        .windows(2)
        .filter_map(|w| {
            let gap = (w[1].bbox.0 - w[0].bbox.1).abs();
            if gap > 0.0 {
                Some(gap)
            } else {
                None
            }
        })
        .collect();

    let median_line_gap = median(&line_heights).unwrap_or(12.0);
    let para_threshold = median_line_gap * 1.5;

    let mut current_text = String::new();
    let mut current_bold = false;
    let mut current_italic = false;
    let mut current_font_size = None;
    let mut current_font_name: Option<String> = None;

    for (i, span) in spans.iter().enumerate() {
        let is_new_para = if i == 0 {
            false
        } else {
            let prev = &spans[i - 1];
            let y_gap = (span.bbox.0 - prev.bbox.1).abs();
            y_gap > para_threshold
        };

        if is_new_para && !current_text.is_empty() {
            paragraphs.push(ParsedParagraph {
                text: current_text.trim().to_string(),
                page_number,
                is_bold: current_bold,
                is_italic: current_italic,
                is_underline: false,
                is_all_caps: current_text
                    .trim()
                    .chars()
                    .all(|c| !c.is_alphabetic() || c.is_uppercase()),
                is_heading: false,
                heading_level: None,
                font_size: current_font_size,
                font_name: current_font_name.clone(),
            });
            current_text.clear();
        }

        if !current_text.is_empty() {
            current_text.push(' ');
        }
        current_text.push_str(&span.text);
        current_bold = current_bold || span.is_bold;
        current_italic = current_italic || span.is_italic;
        if current_font_size.is_none() {
            current_font_size = Some(span.font_size);
        }
        if current_font_name.is_none() {
            current_font_name = Some(span.font_name.clone());
        }
    }

    if !current_text.is_empty() {
        paragraphs.push(ParsedParagraph {
            text: current_text.trim().to_string(),
            page_number,
            is_bold: current_bold,
            is_italic: current_italic,
            is_underline: false,
            is_all_caps: current_text
                .trim()
                .chars()
                .all(|c| !c.is_alphabetic() || c.is_uppercase()),
            is_heading: false,
            heading_level: None,
            font_size: current_font_size,
            font_name: current_font_name,
        });
    }

    paragraphs
}

fn median(values: &[f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted: Vec<f32> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_odd() {
        assert_eq!(median(&[1.0, 3.0, 2.0]), Some(2.0));
    }

    #[test]
    fn test_median_even() {
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), Some(2.5));
    }

    #[test]
    fn test_median_empty() {
        assert_eq!(median(&[]), None);
    }
}
