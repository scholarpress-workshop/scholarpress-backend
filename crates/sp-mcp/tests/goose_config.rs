use sp_mcp::{update_config, GooseSetup};
use std::path::Path;

fn test_setup(config_path: &Path) -> GooseSetup {
    let root = config_path.parent().unwrap();
    GooseSetup {
        config_path: config_path.to_path_buf(),
        command: root.join("sp-mcp.exe"),
        catalog_path: root.join("catalog"),
        workspace_root: root
            .join("project")
            .join(".scholarpress")
            .join("workspaces"),
        typst_path: root.join("bin").join("typst.exe"),
        pandoc_path: root.join("bin").join("pandoc.exe"),
    }
}

#[test]
fn update_preserves_other_extensions_and_writes_scholarpress() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    std::fs::write(
        &config,
        "extensions:\n  other:\n    cmd: other-tool\n    enabled: true\nsettings:\n  keep: true\n",
    )
    .unwrap();
    let setup = test_setup(&config);

    let backup = update_config(&setup).unwrap();
    assert!(backup.is_file());
    let value: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(value["extensions"]["other"]["cmd"], "other-tool");
    assert_eq!(value["settings"]["keep"], true);
    assert_eq!(value["extensions"]["scholarpress"]["type"], "stdio");
    assert_eq!(value["extensions"]["scholarpress"]["enabled"], true);
    assert_eq!(value["extensions"]["scholarpress"]["timeout"], 300);
    assert_eq!(
        value["extensions"]["scholarpress"]["envs"]["SCHOLARPRESS_WORKSPACE_ROOT"],
        setup.workspace_root.to_string_lossy().as_ref()
    );
}

#[test]
fn invalid_yaml_is_rejected_without_changing_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    let original = "extensions: [not: valid\n";
    std::fs::write(&config, original).unwrap();

    let error = update_config(&test_setup(&config)).unwrap_err();
    assert!(error.to_string().contains("invalid Goose YAML"));
    assert_eq!(std::fs::read_to_string(&config).unwrap(), original);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn non_mapping_extensions_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    std::fs::write(&config, "extensions: []\n").unwrap();

    let error = update_config(&test_setup(&config)).unwrap_err();
    assert!(error.to_string().contains("non-mapping extensions"));
}

#[test]
fn missing_config_is_created_and_backup_records_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("nested").join("config.yaml");
    let backup = update_config(&test_setup(&config)).unwrap();
    assert!(config.is_file());
    assert!(backup.is_file());
    assert_eq!(std::fs::read_to_string(backup).unwrap(), "");
}
