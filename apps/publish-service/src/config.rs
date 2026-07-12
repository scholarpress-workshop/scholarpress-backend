use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub catalog_path: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4000);

        let catalog_path = std::env::var("CATALOG_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                let cwd = std::env::current_dir().ok()?;
                let sibling = cwd.parent()?.join("scholarpress-catalog");
                if sibling.exists() {
                    Some(sibling)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| PathBuf::from("../scholarpress-catalog"));

        Self { port, catalog_path }
    }
}
