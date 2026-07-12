use doc_service::extract::document::*;

#[test]
fn test_parsed_document_serialization() {
    let doc = ParsedDocument {
        raw_text: "CHAPTER 1\n\nBody text.".into(),
        pages: vec![ParsedPage {
            number: 1,
            text: "CHAPTER 1\n\nBody text.".into(),
            width: Some(612.0),
            height: Some(792.0),
        }],
        paragraphs: vec![
            ParsedParagraph {
                text: "CHAPTER 1".into(),
                page_number: Some(1),
                is_bold: false,
                is_italic: false,
                is_underline: false,
                is_all_caps: true,
                is_heading: false,
                heading_level: None,
                font_size: Some(12.0),
                font_name: Some("Times New Roman".into()),
            },
            ParsedParagraph {
                text: "Body text.".into(),
                page_number: Some(1),
                is_bold: false,
                is_italic: false,
                is_underline: false,
                is_all_caps: false,
                is_heading: false,
                heading_level: None,
                font_size: Some(12.0),
                font_name: Some("Times New Roman".into()),
            },
        ],
        headings: vec![],
        metadata: ParsedMetadata {
            title: None,
            author: None,
            page_count: 1,
            page_count_estimated: false,
            detected_fonts: vec!["Times New Roman".into()],
        },
    };

    let json = serde_json::to_string(&doc).unwrap();
    let parsed: ParsedDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.raw_text, "CHAPTER 1\n\nBody text.");
    assert_eq!(parsed.pages.len(), 1);
    assert_eq!(parsed.paragraphs.len(), 2);
    assert!(parsed.paragraphs[0].is_all_caps);
}

#[test]
fn test_heading_detection_config_defaults() {
    let config = HeadingDetectionConfig::default();
    assert_eq!(config.threshold, 0.5);
    assert_eq!(config.signals.caps, 0.35);
    assert_eq!(config.signals.underline, 0.35);
}
