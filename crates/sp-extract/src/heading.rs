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
