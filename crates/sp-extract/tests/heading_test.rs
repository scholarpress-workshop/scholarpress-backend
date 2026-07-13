use sp_extract::document::*;
use sp_extract::heading::*;

fn make_para(text: &str, bold: bool, all_caps: bool, font_size: f32) -> ParsedParagraph {
    ParsedParagraph {
        text: text.to_string(),
        page_number: 1,
        is_bold: bold,
        is_italic: false,
        is_underline: false,
        is_all_caps: all_caps,
        is_heading: false,
        heading_level: None,
        font_size: Some(font_size),
        font_name: Some("Times New Roman".to_string()),
    }
}

#[test]
fn test_detect_headings_bold_all_caps() {
    let mut paragraphs = vec![
        make_para("CHAPTER ONE", true, true, 14.0),
        make_para("Regular body text", false, false, 12.0),
    ];
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert_eq!(headings.len(), 1);
    assert!(paragraphs[0].is_heading);
    assert_eq!(paragraphs[0].heading_level, Some(1));
    assert!(!paragraphs[1].is_heading);
}

#[test]
fn test_detect_headings_numbered_section() {
    let mut paragraphs = vec![
        make_para("2.1 Background and Motivation", true, false, 12.0),
        make_para("Body text here", false, false, 12.0),
    ];
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert_eq!(headings.len(), 1);
    assert!(paragraphs[0].is_heading);
    assert_eq!(paragraphs[0].heading_level, Some(2));
}

#[test]
fn test_detect_headings_below_threshold() {
    let mut paragraphs = vec![make_para(
        "just some regular text that is not a heading",
        false,
        false,
        12.0,
    )];
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert!(headings.is_empty());
    assert!(!paragraphs[0].is_heading);
}

#[test]
fn test_median_font_size_detect_heading_above_median() {
    let mut paragraphs = vec![
        make_para("text_a", false, false, 10.0),
        make_para("text_b", true, false, 14.0),
    ];
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert_eq!(headings.len(), 1);
}

#[test]
fn test_median_font_size_single_no_heading() {
    let mut paragraphs = vec![make_para("only paragraph", false, false, 12.0)];
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut paragraphs, &config);
    assert!(headings.is_empty());
}
