#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub start_char: usize,
    pub end_char: usize,
}

pub fn chunk_text(raw_text: &str, max_chars: usize, overlap: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < raw_text.len() {
        let end = (start + max_chars).min(raw_text.len());
        let slice = &raw_text[start..end];
        let break_point = slice.rfind("\n\n").map(|p| start + p + 2).unwrap_or(end);
        let final_end = break_point.min(end);
        let text = raw_text[start..final_end].to_string();
        chunks.push(Chunk { text, start_char: start, end_char: final_end });
        start = final_end.saturating_sub(overlap);
    }
    chunks
}
