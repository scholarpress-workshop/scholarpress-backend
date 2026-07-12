use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Institution {
    pub id: String,
    pub name: String,
    pub spec: serde_yaml::Value,
    pub template_dir: std::path::PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_config: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_config: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone)]
pub struct Registry {
    pub institutions: HashMap<String, Institution>,
}

impl Registry {
    pub fn load(catalog_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let institutions_dir = catalog_path.join("institutions");
        let mut institutions = HashMap::new();

        if !institutions_dir.exists() {
            return Err(format!(
                "Institutions directory not found: {}",
                institutions_dir.display()
            )
            .into());
        }

        for entry in std::fs::read_dir(&institutions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let id = entry.file_name().to_string_lossy().to_string();
            let spec_path = path.join("spec.yaml");
            let template_dir = path.join("template");

            if !spec_path.exists() {
                continue;
            }

            let spec_yaml = std::fs::read_to_string(&spec_path)?;
            let spec: serde_yaml::Value = serde_yaml::from_str(&spec_yaml)?;

            let name = spec
                .get("institution")
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .to_string();

            let llm_config = path.join("llm.yaml");
            let llm = if llm_config.exists() {
                let s = std::fs::read_to_string(&llm_config)?;
                Some(serde_yaml::from_str(&s)?)
            } else {
                None
            };

            let ui_config = path.join("ui.yaml");
            let ui = if ui_config.exists() {
                let s = std::fs::read_to_string(&ui_config)?;
                Some(serde_yaml::from_str(&s)?)
            } else {
                None
            };

            institutions.insert(
                id.clone(),
                Institution {
                    id,
                    name,
                    spec,
                    template_dir,
                    llm_config: llm,
                    ui_config: ui,
                },
            );
        }

        Ok(Self { institutions })
    }

    pub fn get(&self, id: &str) -> Option<&Institution> {
        self.institutions.get(id)
    }

    pub fn list(&self) -> Vec<&Institution> {
        self.institutions.values().collect()
    }
}
