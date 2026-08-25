use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[tokio::test]
async fn stdio_transport_accepts_mcp_initialize() {
    let root = tempfile::tempdir().unwrap();
    let catalog = root.path().join("catalog");
    let workspaces = root.path().join("workspaces");
    std::fs::create_dir_all(&catalog).unwrap();
    std::fs::create_dir_all(&workspaces).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_sp-mcp"))
        .env("SCHOLARPRESS_CATALOG_PATH", &catalog)
        .env("SCHOLARPRESS_WORKSPACE_ROOT", &workspaces)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"0\"}}}\n")
        .await
        .unwrap();
    stdin.flush().await.unwrap();

    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line).await.unwrap();
    assert!(line.contains("serverInfo"), "unexpected response: {line}");
    child.kill().await.unwrap();
}
