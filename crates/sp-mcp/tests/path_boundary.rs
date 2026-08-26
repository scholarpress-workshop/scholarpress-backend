use sp_mcp::{config::Config, workspace};
use std::path::Path;

fn config(root: &tempfile::TempDir) -> Config {
    Config::new(root.path().join("catalog"), root.path().join("workspaces"))
}

#[test]
fn rejects_workspace_outside_root() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("catalog")).unwrap();
    std::fs::create_dir_all(root.path().join("workspaces")).unwrap();
    std::fs::write(outside.path().join("entry.typ"), "= Outside").unwrap();
    let result =
        workspace::compile_typst(&config(&root), outside.path(), Path::new("entry.typ"), None);
    assert!(result.unwrap_err().to_string().contains("workspace path"));
}

#[test]
fn rejects_entry_path_traversal() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspaces/job");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(root.path().join("catalog")).unwrap();
    std::fs::write(root.path().join("outside.typ"), "= Outside").unwrap();
    let result = workspace::compile_typst(
        &config(&root),
        &workspace,
        Path::new("../../outside.typ"),
        None,
    );
    assert!(result.unwrap_err().to_string().contains("entry path"));
}

#[test]
fn rejects_output_name_escape() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspaces/job");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(root.path().join("catalog")).unwrap();
    std::fs::write(workspace.join("entry.typ"), "= Job").unwrap();
    let result = workspace::compile_typst(
        &config(&root),
        &workspace,
        Path::new("entry.typ"),
        Some("../outside"),
    );
    assert!(result.unwrap_err().to_string().contains("output path"));
}

#[cfg(unix)]
#[test]
fn rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspaces/job");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(root.path().join("catalog")).unwrap();
    std::fs::write(outside.path().join("entry.typ"), "= Outside").unwrap();
    symlink(
        outside.path().join("entry.typ"),
        workspace.join("entry.typ"),
    )
    .unwrap();
    let result = workspace::compile_typst(&config(&root), &workspace, Path::new("entry.typ"), None);
    assert!(result.unwrap_err().to_string().contains("entry path"));
}

#[cfg(windows)]
#[test]
fn rejects_junction_escape() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspaces/job");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(root.path().join("catalog")).unwrap();
    std::fs::write(outside.path().join("entry.typ"), "= Outside").unwrap();
    let command = format!(
        "/C mklink /J \"{}\" \"{}\"",
        workspace.join("linked").display(),
        outside.path().display()
    );
    let status = std::process::Command::new("cmd")
        .arg(command)
        .status()
        .unwrap();
    assert!(status.success());
    let result = workspace::compile_typst(
        &config(&root),
        &workspace.join("linked"),
        Path::new("entry.typ"),
        None,
    );
    assert!(result.unwrap_err().to_string().contains("workspace path"));
}
