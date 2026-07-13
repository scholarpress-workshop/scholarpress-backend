mod common;
use common::has_typst_binary;
use sp_typst::compile;

#[test]
fn test_compile_valid_typst_produces_pdf() {
    if !has_typst_binary() {
        eprintln!("skipping: typst binary not found");
        return;
    }
    let source = r#"#set page(width: 100pt, height: 100pt); "hello""#;
    let pdf = compile(source, None).expect("compilation should succeed");
    assert!(!pdf.is_empty(), "PDF should not be empty");
    assert_eq!(&pdf[0..5], b"%PDF-", "should start with PDF header");
}

#[test]
fn test_compile_invalid_typst_returns_error() {
    if !has_typst_binary() {
        eprintln!("skipping: typst binary not found");
        return;
    }
    let result = compile(r"#notarealfunction", None);
    assert!(result.is_err(), "invalid Typst should return Err");
}
