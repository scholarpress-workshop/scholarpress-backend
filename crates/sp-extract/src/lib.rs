pub mod document;
pub mod chunker;
pub mod heading;

pub fn extract_pdf(_bytes: &[u8]) -> Result<document::ParsedDocument, Box<dyn std::error::Error>> {
    unimplemented!("Phase 3")
}

pub fn extract_docx(_bytes: &[u8]) -> Result<document::ParsedDocument, Box<dyn std::error::Error>> {
    unimplemented!("Phase 3")
}
