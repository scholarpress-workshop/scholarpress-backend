use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub catalog_path: PathBuf,
    pub workspace_root: PathBuf,
}

impl Config {
    pub fn new(catalog_path: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            catalog_path,
            workspace_root,
        }
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        let catalog_path = std::env::var("SCHOLARPRESS_CATALOG_PATH")
            .map_err(|_| ConfigError::Missing("SCHOLARPRESS_CATALOG_PATH"))?;
        let workspace_root = std::env::var("SCHOLARPRESS_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_workspace_root());

        let catalog_path = PathBuf::from(catalog_path);

        if !catalog_path.is_dir() {
            return Err(ConfigError::NotADirectory(
                "SCHOLARPRESS_CATALOG_PATH",
                catalog_path,
            ));
        }
        if let Some(parent) = workspace_root.parent() {
            if !parent.is_dir() && !parent.as_os_str().is_empty() {
                return Err(ConfigError::NotADirectory(
                    "SCHOLARPRESS_WORKSPACE_ROOT parent",
                    parent.to_path_buf(),
                ));
            }
        }

        Ok(Self {
            catalog_path,
            workspace_root,
        })
    }
}

fn default_workspace_root() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".scholarpress").join("workspaces")
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("environment variable {0} is not set")]
    Missing(&'static str),
    #[error("{0} does not point to an existing directory: {1}")]
    NotADirectory(&'static str, PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_workspace_root_uses_home() {
        let root = default_workspace_root();
        assert!(root.ends_with(".scholarpress/workspaces"));
    }

    #[test]
    fn new_stores_values() {
        let cfg = Config::new(PathBuf::from("/c"), PathBuf::from("/w"));
        assert_eq!(cfg.catalog_path, PathBuf::from("/c"));
        assert_eq!(cfg.workspace_root, PathBuf::from("/w"));
    }
}
