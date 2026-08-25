use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct GooseSetup {
    pub config_path: PathBuf,
    pub command: PathBuf,
    pub catalog_path: PathBuf,
    pub workspace_root: PathBuf,
    pub typst_path: PathBuf,
    pub pandoc_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum GooseConfigError {
    #[error("failed to read Goose config {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid Goose YAML in {path}: {source}")]
    Yaml {
        path: PathBuf,
        source: serde_yaml::Error,
    },
    #[error("Goose config {path} must contain a YAML mapping at the top level")]
    TopLevel { path: PathBuf },
    #[error("Goose config {path} has a non-mapping extensions value")]
    Extensions { path: PathBuf },
    #[error("failed to serialize Goose config: {0}")]
    Serialize(serde_yaml::Error),
    #[error("failed to write Goose config: {0}")]
    Io(#[from] std::io::Error),
}

pub fn update_config(setup: &GooseSetup) -> Result<PathBuf, GooseConfigError> {
    let original = if setup.config_path.is_file() {
        fs::read_to_string(&setup.config_path).map_err(|source| GooseConfigError::Read {
            path: setup.config_path.clone(),
            source,
        })?
    } else {
        String::new()
    };
    let mut root = if original.trim().is_empty() {
        Value::Mapping(Mapping::new())
    } else {
        serde_yaml::from_str(&original).map_err(|source| GooseConfigError::Yaml {
            path: setup.config_path.clone(),
            source,
        })?
    };
    let root_map = root
        .as_mapping_mut()
        .ok_or_else(|| GooseConfigError::TopLevel {
            path: setup.config_path.clone(),
        })?;
    let extensions = root_map
        .entry(Value::String("extensions".into()))
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    let extensions = extensions
        .as_mapping_mut()
        .ok_or_else(|| GooseConfigError::Extensions {
            path: setup.config_path.clone(),
        })?;
    extensions.insert(Value::String("scholarpress".into()), extension(setup));

    let serialized = serde_yaml::to_string(&root).map_err(GooseConfigError::Serialize)?;
    if let Some(parent) = setup.config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let backup = backup_path(&setup.config_path);
    fs::write(&backup, original)?;
    let temp = setup.config_path.with_extension("yaml.tmp");
    fs::write(&temp, serialized)?;
    replace_file(&temp, &setup.config_path)?;
    Ok(backup)
}

fn extension(setup: &GooseSetup) -> Value {
    let mut envs = Mapping::new();
    envs.insert(
        string("SCHOLARPRESS_CATALOG_PATH"),
        path(&setup.catalog_path),
    );
    envs.insert(
        string("SCHOLARPRESS_WORKSPACE_ROOT"),
        path(&setup.workspace_root),
    );
    envs.insert(string("SCHOLARPRESS_TYPST_PATH"), path(&setup.typst_path));
    envs.insert(string("SCHOLARPRESS_PANDOC_PATH"), path(&setup.pandoc_path));

    let mut value = Mapping::new();
    value.insert(string("name"), string("ScholarPress"));
    value.insert(
        string("description"),
        string("Format and validate dissertation documents with ScholarPress"),
    );
    value.insert(string("cmd"), path(&setup.command));
    value.insert(string("args"), Value::Sequence(Vec::new()));
    value.insert(string("enabled"), Value::Bool(true));
    value.insert(string("type"), string("stdio"));
    value.insert(string("timeout"), Value::Number(300.into()));
    value.insert(string("envs"), Value::Mapping(envs));
    Value::Mapping(value)
}

fn string(value: impl Into<String>) -> Value {
    Value::String(value.into())
}

fn path(value: &Path) -> Value {
    string(value.to_string_lossy())
}

fn backup_path(config: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    PathBuf::from(format!("{}.bak-{stamp}", config.display()))
}

fn replace_file(temp: &Path, config: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    if config.exists() {
        fs::remove_file(config)?;
    }
    fs::rename(temp, config)
}
