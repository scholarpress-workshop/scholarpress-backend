use std::io::Write;
use std::path::Path;
use std::process::Command;

pub mod template;

/// Compile Typst source to PDF.
///
/// Currently shells out to the external `typst` binary (must be on PATH).
/// Tracked for in-process embedding once the toolset stabilizes.
pub fn compile(source: &str, root: Option<&Path>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // ponytail: shells out to typst binary (external dep on PATH).
    // Upgrade: swap for `typst` crate once sp-typst API churn settles and
    // per-iteration build cost matters less. See
    // docs/superpowers/specs/2026-07-29-scholarpress-mcp-design.md
    let mut cmd = Command::new("typst");
    cmd.arg("compile")
        .arg("--format")
        .arg("pdf")
        .arg("-")
        .arg("-");

    if let Some(r) = root {
        cmd.arg("--root").arg(r);
    }

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(source.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.into())
    }
}
