use sp_extract::extract_pdf;

#[test]
fn test_extract_pdf_minimal_extracts_pages() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    assert!(doc.pages.len() >= 1, "should have at least one page");
}

#[test]
fn test_extract_pdf_minimal_contains_text() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    let all_text: String = doc.pages.iter().map(|p| p.text.clone()).collect::<Vec<_>>().join(" ");
    assert!(all_text.contains("Introduction"), "should find heading text: {}", all_text);
    assert!(all_text.contains("Lorem"), "should find body text: {}", all_text);
}

#[test]
fn test_extract_pdf_detects_fonts() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    assert!(!doc.metadata.detected_fonts.is_empty(), "should detect at least one font");
}

#[test]
fn test_extract_pdf_has_correct_page_count() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    assert_eq!(doc.metadata.page_count_estimated, false);
    assert!(doc.metadata.page_count >= 1);
}

#[test]
fn test_extract_pdf_pages_have_dimensions() {
    let bytes = include_bytes!("fixtures/minimal.pdf");
    let doc = extract_pdf(bytes).expect("extraction should succeed");
    for page in &doc.pages {
        assert!(page.width > 0.0);
        assert!(page.height > 0.0);
    }
}
