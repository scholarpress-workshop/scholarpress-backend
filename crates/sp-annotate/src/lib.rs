pub mod annotate;

use sp_check::report::Report;

#[derive(Debug, thiserror::Error)]
pub enum AnnotateError {
    #[error("failed to parse report JSON: {0}")]
    Report(#[from] serde_json::Error),

    #[error("pdf error: {0}")]
    Pdf(#[from] pdf_oxide::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Parse a `sp-check --json` report back into a `Report`.
pub fn parse_report(json: &str) -> Result<Report, AnnotateError> {
    Ok(serde_json::from_str(json)?)
}

/// Annotate a PDF (given as bytes) and return the annotated copy as bytes.
pub fn annotate_bytes(input: &[u8], report: &Report) -> Result<Vec<u8>, AnnotateError> {
    annotate::annotate_bytes(input, report)
}

/// Annotate `input` and write the result to `output`.
pub fn annotate_file(
    input: &std::path::Path,
    output: &std::path::Path,
    report: &Report,
) -> Result<(), AnnotateError> {
    let bytes = std::fs::read(input)?;
    let annotated = annotate_bytes(&bytes, report)?;
    std::fs::write(output, annotated)?;
    Ok(())
}
