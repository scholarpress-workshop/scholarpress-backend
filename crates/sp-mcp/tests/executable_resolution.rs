use sp_mcp::config::ToolResolver;
use std::path::{Path, PathBuf};

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn write_tool(root: &Path, relative: &str) -> PathBuf {
    let path = root.join(relative).join(executable_name("typst"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"tool").unwrap();
    path
}

#[test]
fn explicit_tool_override_wins_over_bundle_and_path() {
    let bundle = tempfile::tempdir().unwrap();
    let path_dir = tempfile::tempdir().unwrap();
    let override_dir = tempfile::tempdir().unwrap();
    let bundled = write_tool(bundle.path(), "bin");
    let from_path = write_tool(path_dir.path(), "");
    let override_path = write_tool(override_dir.path(), "custom");
    let resolver = ToolResolver::new(
        bundle.path().to_path_buf(),
        vec![path_dir.path().to_path_buf()],
    );

    let resolved = resolver.resolve("typst", Some(&override_path)).unwrap();

    assert_eq!(resolved, override_path);
    assert_ne!(resolved, bundled);
    assert_ne!(resolved, from_path);
}

#[test]
fn bundle_tool_is_used_before_path() {
    let bundle = tempfile::tempdir().unwrap();
    let path_dir = tempfile::tempdir().unwrap();
    let bundled = write_tool(bundle.path(), "bin");
    write_tool(path_dir.path(), "");
    let resolver = ToolResolver::new(
        bundle.path().to_path_buf(),
        vec![path_dir.path().to_path_buf()],
    );

    assert_eq!(resolver.resolve("typst", None).unwrap(), bundled);
}

#[test]
fn path_tool_is_used_when_bundle_tool_is_missing() {
    let bundle = tempfile::tempdir().unwrap();
    let path_dir = tempfile::tempdir().unwrap();
    let from_path = write_tool(path_dir.path(), "");
    let resolver = ToolResolver::new(
        bundle.path().to_path_buf(),
        vec![path_dir.path().to_path_buf()],
    );

    assert_eq!(resolver.resolve("typst", None).unwrap(), from_path);
}

#[test]
fn missing_tool_error_lists_override_and_bundle_locations() {
    let bundle = tempfile::tempdir().unwrap();
    let resolver = ToolResolver::new(bundle.path().to_path_buf(), Vec::new());

    let error = resolver.resolve("pandoc", None).unwrap_err().to_string();

    assert!(error.contains("SCHOLARPRESS_PANDOC_PATH"));
    assert!(error.contains("bin"));
}
