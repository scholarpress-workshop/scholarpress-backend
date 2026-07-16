use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TextSpan {
    pub text: String,
    pub font_name: String,
    pub font_size: f32,
    pub bbox: (f32, f32, f32, f32),
    pub is_bold: bool,
    pub is_italic: bool,
    pub color: Option<(f32, f32, f32)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedPage {
    pub page_number: usize,
    pub text: String,
    pub width: f32,
    pub height: f32,
    /// Word-level glyph spans on this page. Bbox tuple is (top, bottom, x0, x1)
    /// in page-space coordinates (origin at top-left, Y increases downward).
    pub spans: Vec<TextSpan>,
    /// Image bounding boxes as (top, bottom, x0, x1) in page-space coordinates.
    pub images: Vec<(f32, f32, f32, f32)>,
    /// Path/vector bounding boxes as (top, bottom, x0, x1) in page-space coordinates.
    pub paths: Vec<(f32, f32, f32, f32)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedParagraph {
    pub text: String,
    pub page_number: usize,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underline: bool,
    pub is_all_caps: bool,
    pub is_heading: bool,
    pub heading_level: Option<usize>,
    pub font_size: Option<f32>,
    pub font_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Heading {
    pub text: String,
    pub level: usize,
    pub page_number: usize,
    pub raw_text_position: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: usize,
    pub page_count_estimated: bool,
    pub detected_fonts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedDocument {
    pub raw_text: String,
    pub pages: Vec<ParsedPage>,
    pub paragraphs: Vec<ParsedParagraph>,
    pub headings: Vec<Heading>,
    pub metadata: ParsedMetadata,
}
