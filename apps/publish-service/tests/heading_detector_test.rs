use doc_service::extract::document::*;
use doc_service::extract::heading_detector;

#[test]
fn test_iu_heading_detection() {
    let mut paragraphs = vec![
        // Title case all-caps line without numbering or context — NOT a heading
        p(
            "BRAIDING EDUCATION, WORKFORCE, AND COMMUNITY",
            true,
            false,
            12.0,
        ),
        // CHAPTER with numbering and caps — IS heading level 1
        p("CHAPTER 1: INTRODUCTION", true, false, 14.0),
        // 1.1 with underline + numbering — IS heading level 2
        p_underline("1.1 Background", false, 12.0),
        // Body paragraph — NOT heading
        p("This is body text.", false, false, 12.0),
    ];

    let config = HeadingDetectionConfig::default();
    let headings = heading_detector::detect_headings(&mut paragraphs, &config);

    assert!(
        !paragraphs[0].is_heading,
        "title case should not be heading"
    );
    assert!(paragraphs[1].is_heading, "CHAPTER should be heading");
    assert_eq!(paragraphs[1].heading_level, Some(1));
    assert!(
        paragraphs[2].is_heading,
        "numbered underlined should be heading"
    );
    assert_eq!(paragraphs[2].heading_level, Some(2));
    assert!(!paragraphs[3].is_heading, "body should not be heading");
    assert_eq!(headings.len(), 2);
}

#[test]
fn test_context_keyword_with_caps() {
    let mut paragraphs = vec![p("ABSTRACT", true, false, 14.0)];
    let config = HeadingDetectionConfig {
        threshold: 0.5,
        signals: SignalWeights {
            caps: 0.35,
            underline: 0.0,
            bold: 0.15,
            size_jump: 0.0,
            numbering: 0.0,
            context: 0.15,
        },
        ..Default::default()
    };
    heading_detector::detect_headings(&mut paragraphs, &config);
    assert!(paragraphs[0].is_heading, "caps+context should reach 0.50");
}

fn p(text: &str, is_all_caps: bool, is_bold: bool, font_size: f32) -> ParsedParagraph {
    ParsedParagraph {
        text: text.into(),
        is_all_caps,
        is_bold,
        is_underline: false,
        is_italic: false,
        font_size: Some(font_size),
        font_name: None,
        is_heading: false,
        heading_level: None,
        page_number: None,
    }
}

fn p_underline(text: &str, is_all_caps: bool, font_size: f32) -> ParsedParagraph {
    ParsedParagraph {
        text: text.into(),
        is_all_caps,
        is_bold: false,
        is_underline: true,
        is_italic: false,
        font_size: Some(font_size),
        font_name: None,
        is_heading: false,
        heading_level: None,
        page_number: None,
    }
}
