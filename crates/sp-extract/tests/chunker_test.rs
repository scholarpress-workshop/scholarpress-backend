use sp_extract::chunker::chunk_text;

#[test]
fn test_chunk_text_empty() {
    let chunks = chunk_text("", 100, 20);
    assert!(chunks.is_empty());
}

#[test]
fn test_chunk_text_smaller_than_chunk_size() {
    let chunks = chunk_text("hello world", 100, 20);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].text, "hello world");
}

#[test]
fn test_chunk_text_larger_with_overlap() {
    let text = "a".repeat(500);
    let chunks = chunk_text(&text, 100, 20);
    assert!(chunks.len() >= 4, "should produce multiple chunks, got {}", chunks.len());
    assert!(chunks[0].text.len() <= 100);
    assert!(chunks[1].start_char < chunks[0].end_char);
}

#[test]
fn test_chunk_text_paragraph_boundary() {
    let para1 = "First paragraph with some text. And another sentence.";
    let para2 = "Second paragraph here. More text.";
    let text = format!("{}\n\n{}", para1, para2);
    let chunks = chunk_text(&text, 40, 10);
    for chunk in &chunks {
        assert!(!chunk.text.ends_with('w'), "should not break mid-word: {:?}", chunk);
    }
}
