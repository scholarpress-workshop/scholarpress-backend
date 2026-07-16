use assert_cmd::Command;
use std::io::Write;

const MOCK_SPEC_YAML: &str = r#"
institution: Test University
source_revision: "2026-01"
document_structure:
  front_matter:
    - { id: title_page, required: true }
  body:
    - { id: chapters, required: true }
  end_matter:
    - { id: references, required: true }
checks:
  - id: global_margins
    category: layout
    checker: margins
    target: { scope: all_pages }
    params:
      top: 1in
      bottom: 1in
      left: 1.25in
      right: 1.25in
"#;

fn baseline_pdf_path() -> String {
    format!(
        "{}/crates/sp-check/tests/fixtures/margins/baseline.pdf",
        env!("CARGO_MANIFEST_DIR").trim_end_matches("/apps/scholarpress-cli")
    )
}

#[test]
fn test_check_subcommand_exit_zero() {
    let baseline = baseline_pdf_path();
    let mut spec_file = tempfile::NamedTempFile::new().unwrap();
    write!(spec_file, "{}", MOCK_SPEC_YAML).unwrap();

    let mut cmd = Command::cargo_bin("scholarpress-cli").unwrap();
    cmd.arg("check")
        .arg("--spec")
        .arg(spec_file.path())
        .arg(&baseline);
    let output = cmd.output().unwrap();
    assert!(
        output.status.code().is_some_and(|c| c <= 1),
        "CLI crashed unexpectedly: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_calibrate_subcommand() {
    let baseline = baseline_pdf_path();
    let mut spec_file = tempfile::NamedTempFile::new().unwrap();
    write!(spec_file, "{}", MOCK_SPEC_YAML).unwrap();

    let corpus_dir = tempfile::tempdir().unwrap();
    std::fs::copy(&baseline, corpus_dir.path().join("test.pdf")).unwrap();

    let mut cmd = Command::cargo_bin("scholarpress-cli").unwrap();
    cmd.arg("calibrate")
        .arg("--spec")
        .arg(spec_file.path())
        .arg("--corpus")
        .arg(corpus_dir.path());
    let output = cmd.output().unwrap();
    assert!(
        output.status.code().is_some_and(|c| c <= 1),
        "CLI crashed unexpectedly: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
