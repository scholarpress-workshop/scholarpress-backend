use doc_service::extract::chunker;

#[test]
fn test_chunk_paragraph_boundaries() {
    let text = "Para 1 first sentence. Para 1 second sentence. Para 1 third sentence.\n\nPara 2 first sentence. Para 2 second sentence.\n\nPara 3 first sentence.";
    let chunks = chunker::chunk_text(text, 60, 10);
    assert!(chunks.len() >= 2);
    assert!(chunks[0].text.contains("Para 1"));
    assert!(chunks[1].start_char < chunks[0].end_char);
}

#[test]
fn test_chunk_short_text() {
    let chunks = chunker::chunk_text("Short.", 1000, 200);
    assert_eq!(chunks.len(), 1);
}

#[test]
fn test_chunk_respects_max() {
    let text = "x".repeat(100);
    let chunks = chunker::chunk_text(&text, 50, 10);
    assert!(chunks.len() >= 2);
    for c in &chunks {
        assert!(c.text.len() <= 50);
    }
}
