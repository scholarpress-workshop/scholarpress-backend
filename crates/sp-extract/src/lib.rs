pub mod document;
pub mod chunker;
pub mod heading;
pub mod pdf;
pub mod docx;

use heading::{detect_headings, HeadingDetectionConfig};

pub fn extract_pdf(bytes: &[u8]) -> Result<document::ParsedDocument, Box<dyn std::error::Error>> {
    let mut doc = pdf::extract_pdf(bytes)?;
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut doc.paragraphs, &config);
    doc.headings = headings;
    Ok(doc)
}

pub fn extract_docx(bytes: &[u8]) -> Result<document::ParsedDocument, Box<dyn std::error::Error>> {
    let mut doc = docx::extract_docx(bytes)?;
    let config = HeadingDetectionConfig::default();
    let headings = detect_headings(&mut doc.paragraphs, &config);
    doc.headings = headings;
    Ok(doc)
}
