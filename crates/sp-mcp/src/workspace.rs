use crate::config::Config;
use crate::error::SpMcpError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceInfo {
    pub name: String,
    pub path: PathBuf,
    pub profile_id: Option<String>,
    pub mtime: SystemTime,
}

pub fn list_workspaces(config: &Config) -> Result<Vec<WorkspaceInfo>, SpMcpError> {
    if !config.workspace_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&config.workspace_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        let meta = entry.metadata()?;
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let profile_id = read_profile_id_from_spec(&path.join("spec.yaml"));
        out.push(WorkspaceInfo {
            name,
            path,
            profile_id,
            mtime,
        });
    }
    out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    Ok(out)
}

fn read_profile_id_from_spec(spec_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(spec_path).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("institution:") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
        if let Some(rest) = line.strip_prefix("server:") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
        if let Some(rest) = line.strip_prefix("journal:") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileInfo {
    pub id: String,
    pub scope: String,
    pub name: String,
}

pub fn list_profiles(config: &Config) -> Result<Vec<ProfileInfo>, SpMcpError> {
    if !config.catalog_path.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for scope_entry in std::fs::read_dir(&config.catalog_path)? {
        let scope_entry = scope_entry?;
        let scope_path = scope_entry.path();
        if !scope_path.is_dir() {
            continue;
        }
        let scope = match scope_entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        for id_entry in std::fs::read_dir(&scope_path)? {
            let id_entry = id_entry?;
            let id_path = id_entry.path();
            if !id_path.is_dir() {
                continue;
            }
            if !id_path.join("spec.yaml").is_file() {
                continue;
            }
            if !id_path.join("template").join("template.typ").is_file() {
                continue;
            }
            let id = match id_entry.file_name().into_string() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let name = read_profile_name(&id_path.join("spec.yaml"))
                .unwrap_or_else(|| id.clone());
            out.push(ProfileInfo {
                id: format!("{}/{}", scope, id),
                scope: scope.clone(),
                name,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn read_profile_name(spec_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(spec_path).ok()?;
    for line in text.lines() {
        for key in &["institution:", "server:", "journal:", "grant:"] {
            if let Some(rest) = line.strip_prefix(key) {
                let v = rest.trim().trim_matches('"').to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

pub fn create_workspace(
    config: &Config,
    name: &str,
    profile_id: &str,
) -> Result<PathBuf, SpMcpError> {
    validate_name(name)?;

    let profile_dir = config.catalog_path.join(profile_id);
    if !profile_dir.is_dir() {
        let available: Vec<String> = list_profiles(config)?
            .into_iter()
            .map(|p| p.id)
            .collect();
        return Err(SpMcpError::ProfileNotFound(
            profile_id.to_string(),
            available,
        ));
    }
    if !profile_dir.join("spec.yaml").is_file() {
        return Err(SpMcpError::SpecMissing(profile_dir.join("spec.yaml")));
    }

    let target = config.workspace_root.join(name);
    if target.exists() {
        return Err(SpMcpError::WorkspaceExists(target));
    }
    std::fs::create_dir_all(target.join("data"))?;
    std::fs::create_dir_all(target.join("out"))?;

    copy_tree(&profile_dir.join("spec.yaml"), &target.join("spec.yaml"))?;
    let template_src = profile_dir.join("template");
    if template_src.is_dir() {
        copy_dir_recursive(&template_src, &target.join("template"))?;
    }

    Ok(target)
}

fn validate_name(name: &str) -> Result<(), SpMcpError> {
    if name.is_empty() {
        return Err(SpMcpError::BadWorkspaceName(name.to_string()));
    }
    if name.contains('/') || name.contains("..") || name.contains('\0') {
        return Err(SpMcpError::BadWorkspaceName(name.to_string()));
    }
    Ok(())
}

fn copy_tree(src: &Path, dst: &Path) -> Result<(), SpMcpError> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dst)?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), SpMcpError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            // skip tests/corpus — large calibration PDFs
            if from.file_name() == Some(std::ffi::OsStr::new("tests")) {
                continue;
            }
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread::sleep;
    use std::time::Duration;

    fn make_workspace(root: &Path, name: &str, spec: Option<&str>) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        if let Some(text) = spec {
            fs::write(dir.join("spec.yaml"), text).unwrap();
        }
        sleep(Duration::from_millis(10));
    }

    #[test]
    fn list_workspaces_returns_sorted_by_mtime_desc() {
        let tmp = local_tempdir();
        make_workspace(&tmp, "alpha", Some("institution: Alpha University\n"));
        make_workspace(&tmp, "beta", Some("server: arxiv\n"));
        make_workspace(&tmp, "gamma", None);

        let cfg = Config::new(PathBuf::from("/c"), tmp.clone());
        let result = list_workspaces(&cfg).unwrap();

        let names: Vec<&str> = result.iter().map(|w| w.name.as_str()).collect();
        assert_eq!(names, vec!["gamma", "beta", "alpha"], "newest first");

        let alpha = result.iter().find(|w| w.name == "alpha").unwrap();
        assert_eq!(alpha.profile_id.as_deref(), Some("Alpha University"));
        let beta = result.iter().find(|w| w.name == "beta").unwrap();
        assert_eq!(beta.profile_id.as_deref(), Some("arxiv"));
        let gamma = result.iter().find(|w| w.name == "gamma").unwrap();
        assert_eq!(gamma.profile_id, None);
    }

    #[test]
    fn list_workspaces_empty_when_root_missing() {
        let cfg = Config::new(PathBuf::from("/c"), PathBuf::from("/nonexistent-xyz-12345"));
        let result = list_workspaces(&cfg).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_profiles_finds_scoped_profiles() {
        let catalog = local_tempdir();
        let iu = catalog.join("institutions").join("iu");
        fs::create_dir_all(iu.join("template")).unwrap();
        fs::write(iu.join("spec.yaml"), "institution: Indiana University\n").unwrap();
        fs::write(iu.join("template").join("template.typ"), "= Hi\n").unwrap();

        let arxiv = catalog.join("servers").join("arxiv");
        fs::create_dir_all(arxiv.join("template")).unwrap();
        fs::write(arxiv.join("spec.yaml"), "server: arxiv\n").unwrap();
        fs::write(arxiv.join("template").join("template.typ"), "= Hi\n").unwrap();

        // incomplete profile: missing template.typ — should be skipped
        let incomplete = catalog.join("institutions").join("partial");
        fs::create_dir_all(&incomplete).unwrap();
        fs::write(incomplete.join("spec.yaml"), "institution: Partial\n").unwrap();

        let cfg = Config::new(catalog.clone(), PathBuf::from("/w"));
        let profiles = list_profiles(&cfg).unwrap();
        let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["institutions/iu", "servers/arxiv"]);

        let iu_info = profiles.iter().find(|p| p.id == "institutions/iu").unwrap();
        assert_eq!(iu_info.scope, "institutions");
        assert_eq!(iu_info.name, "Indiana University");
    }

    #[test]
    fn create_workspace_copies_spec_and_template() {
        let catalog = local_tempdir();
        let iu = catalog.join("institutions").join("iu");
        fs::create_dir_all(iu.join("template").join("sections")).unwrap();
        fs::write(iu.join("spec.yaml"), "institution: IU\n").unwrap();
        fs::write(iu.join("template").join("template.typ"), "= Hi\n").unwrap();
        fs::write(
            iu.join("template").join("sections").join("ch.typ"),
            "= Chapter\n",
        )
        .unwrap();
        // create a tests/ dir to verify it is skipped
        fs::create_dir_all(iu.join("tests").join("corpus")).unwrap();
        fs::write(iu.join("tests").join("ignored.typ"), "ignored\n").unwrap();

        let ws_root = local_tempdir();
        let cfg = Config::new(catalog.clone(), ws_root.clone());

        let path = create_workspace(&cfg, "iu-job-1", "institutions/iu").unwrap();
        assert!(path.is_dir());
        assert!(path.join("spec.yaml").is_file());
        assert!(path.join("template").join("template.typ").is_file());
        assert!(path.join("template").join("sections").join("ch.typ").is_file());
        assert!(path.join("data").is_dir());
        assert!(path.join("out").is_dir());
        // tests/ skipped
        assert!(!path.join("template").join("tests").exists());
    }

    #[test]
    fn create_workspace_rejects_bad_name() {
        let cfg = Config::new(PathBuf::from("/c"), PathBuf::from("/w"));
        assert!(create_workspace(&cfg, "../escape", "institutions/iu").is_err());
        assert!(create_workspace(&cfg, "a/b", "institutions/iu").is_err());
        assert!(create_workspace(&cfg, "", "institutions/iu").is_err());
    }

    #[test]
    fn create_workspace_unknown_profile_lists_available() {
        let catalog = local_tempdir();
        let iu = catalog.join("institutions").join("iu");
        fs::create_dir_all(iu.join("template")).unwrap();
        fs::write(iu.join("spec.yaml"), "institution: IU\n").unwrap();
        fs::write(iu.join("template").join("template.typ"), "= Hi\n").unwrap();

        let cfg = Config::new(catalog, PathBuf::from("/w"));
        let err = create_workspace(&cfg, "x", "institutions/missing").unwrap_err();
        match err {
            SpMcpError::ProfileNotFound(id, avail) => {
                assert_eq!(id, "institutions/missing");
                assert_eq!(avail, vec!["institutions/iu".to_string()]);
            }
            other => panic!("expected ProfileNotFound, got {:?}", other),
        }
    }

    fn local_tempdir() -> PathBuf {
        let base = std::env::temp_dir();
        let unique = format!(
            "sp-mcp-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let p = base.join(unique);
        fs::create_dir_all(&p).unwrap();
        p
    }
}
