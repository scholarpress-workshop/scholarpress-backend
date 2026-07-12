#[derive(Debug, Clone)]
pub struct HeadingDetectionConfig {
    pub signal_weights: SignalWeights,
    pub threshold: f32,
    pub size_jump_threshold: f32,
    pub context_keywords: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SignalWeights {
    pub caps: f32,
    pub underline: f32,
    pub bold: f32,
    pub size_jump: f32,
    pub numbering: f32,
    pub context: f32,
}

use crate::document::*;

pub fn detect_headings(paragraphs: &mut [ParsedParagraph], config: &HeadingDetectionConfig) -> Vec<Heading> {
    let body_font_size = median_font_size(paragraphs);
    let mut headings = Vec::new();

    let numbering_re = regex::Regex::new(r"^(\d+(?:\.\d+)*)\s").unwrap();
    let section_re = regex::Regex::new(r"^\d+\.\d+").unwrap();
    let sub_section_re = regex::Regex::new(r"^\d+\.\d+\.\d+").unwrap();
    let chapter_re = regex::Regex::new(r"(?i)^(chapter|section)\s+\d+").unwrap();

    for (i, para) in paragraphs.iter_mut().enumerate() {
        let mut score: f32 = 0.0;

        if para.is_all_caps { score += config.signal_weights.caps; }
        if para.is_underline { score += config.signal_weights.underline; }
        if para.is_bold { score += config.signal_weights.bold; }

        if let Some(fs) = para.font_size {
            if let Some(bfs) = body_font_size {
                if fs - bfs >= config.size_jump_threshold {
                    score += config.signal_weights.size_jump;
                }
            }
        }

        if numbering_re.is_match(&para.text) { score += config.signal_weights.numbering; }

        let lower = para.text.to_lowercase();
        if config.context_keywords.iter().any(|kw| lower.contains(kw)) {
            score += config.signal_weights.context;
        }

        if score >= config.threshold {
            let level = if sub_section_re.is_match(&para.text) {
                3
            } else if section_re.is_match(&para.text) {
                2
            } else if chapter_re.is_match(&para.text) {
                1
            } else if para.is_all_caps {
                1
            } else if let (Some(fs), Some(bfs)) = (para.font_size, body_font_size) {
                if fs - bfs >= 4.0 { 1 } else if fs - bfs >= 2.0 { 2 } else { 2 }
            } else {
                2
            };

            para.is_heading = true;
            para.heading_level = Some(level);

            headings.push(Heading {
                text: para.text.clone(),
                level,
                page_number: para.page_number,
                raw_text_position: i,
            });
        }
    }

    headings
}

fn median_font_size(paragraphs: &[ParsedParagraph]) -> Option<f32> {
    let mut sizes: Vec<f32> = paragraphs.iter().filter_map(|p| p.font_size).collect();
    if sizes.is_empty() { return None; }
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(sizes[sizes.len() / 2])
}

impl Default for HeadingDetectionConfig {
    fn default() -> Self {
        Self {
            signal_weights: SignalWeights {
                caps: 0.35, underline: 0.35, bold: 0.15,
                size_jump: 0.0, numbering: 0.10, context: 0.05,
            },
            threshold: 0.5,
            size_jump_threshold: 2.0,
            context_keywords: vec![
                "introduction".into(), "background".into(), "method".into(),
                "result".into(), "discussion".into(), "conclusion".into(),
                "chapter".into(), "appendix".into(), "reference".into(),
                "bibliography".into(), "abstract".into(), "acknowledgment".into(),
                "preface".into(), "dedication".into(), "contents".into(),
                "summary".into(),
            ],
        }
    }
}
