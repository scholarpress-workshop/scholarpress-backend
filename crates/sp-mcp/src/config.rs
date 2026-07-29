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
        // workspace_root is not pre-validated: list_workspaces handles a missing
        // dir by returning an empty list, and create_workspace surfaces a clear
        // error if the path is unwritable. Pre-validating the parent would block
        // the common first-run case where ~/.scholarpress doesn't exist yet.

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

    #[test]
    fn from_env_works_when_workspace_root_parent_missing() {
        // Regression: prior versions of from_env refused to start if the
        // workspace root's parent dir didn't exist yet, blocking the common
        // first-run scenario. The fix is to skip that check.
        let unique = format!(
            "sp-mcp-from-env-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let tmp = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&tmp).unwrap();
        let fake_catalog = tmp.join("catalog");
        std::fs::create_dir_all(&fake_catalog).unwrap();

        // workspace_root is inside tmp (parent exists), but points at a deeper
        // path whose intermediate parent does not.
        let workspace_root = tmp.join("nested").join("workspaces");

        // SAFETY: this test sets process-wide env vars; it must run sequentially.
        // We save and restore to avoid bleeding into other tests.
        let saved_catalog = std::env::var("SCHOLARPRESS_CATALOG_PATH").ok();
        let saved_workspace = std::env::var("SCHOLARPRESS_WORKSPACE_ROOT").ok();
        std::env::set_var("SCHOLARPRESS_CATALOG_PATH", &fake_catalog);
        std::env::set_var("SCHOLARPRESS_WORKSPACE_ROOT", &workspace_root);

        let result = Config::from_env();

        // restore
        match saved_catalog {
            Some(v) => std::env::set_var("SCHOLARPRESS_CATALOG_PATH", v),
            None => std::env::remove_var("SCHOLARPRESS_CATALOG_PATH"),
        }
        match saved_workspace {
            Some(v) => std::env::set_var("SCHOLARPRESS_WORKSPACE_ROOT", v),
            None => std::env::remove_var("SCHOLARPRESS_WORKSPACE_ROOT"),
        }
        std::fs::remove_dir_all(&tmp).ok();

        let cfg = result.expect("from_env should succeed when workspace root parent is missing");
        assert_eq!(cfg.catalog_path, fake_catalog);
        assert_eq!(cfg.workspace_root, workspace_root);
    }
}
