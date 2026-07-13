use std::io::Write;
use std::path::Path;
use std::process::Command;

pub mod template;

pub fn compile(source: &str, root: Option<&Path>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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
        Err(format!("Typst compilation failed: {}", stderr).into())
    }
}
