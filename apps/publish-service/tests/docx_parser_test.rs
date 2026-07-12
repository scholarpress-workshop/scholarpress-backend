use doc_service::extract::docx_parser;

#[test]
fn test_parse_docx_text() {
    let docx_bytes = include_bytes!("../fixtures/minimal.docx");
    let doc = docx_parser::parse_docx(docx_bytes).expect("Failed to parse DOCX");
    assert!(!doc.raw_text.is_empty());
    assert!(doc.paragraphs.len() > 0);
    assert!(doc.metadata.page_count > 0);
    assert!(doc.metadata.page_count_estimated);
}
