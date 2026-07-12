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
pub struct Page {
    pub page_number: usize,
    pub width: f32,
    pub height: f32,
    pub spans: Vec<TextSpan>,
    pub images: Vec<(f32, f32, f32, f32)>,
    pub paths: Vec<(f32, f32, f32, f32)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Document {
    pub pages: Vec<Page>,
}
