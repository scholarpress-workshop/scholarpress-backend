use doc_service::extract::pdf_parser;

#[test]
fn test_parse_pdf_basic() {
    let pdf_bytes = include_bytes!("../../fixtures/test-dissertation.pdf");
    let doc = pdf_parser::parse_pdf(pdf_bytes).unwrap();
    assert!(!doc.raw_text.is_empty());
    assert!(doc.pages.len() > 0);
    assert!(doc.paragraphs.len() > 0);
    assert!(!doc.paragraphs[0].text.is_empty());
}
