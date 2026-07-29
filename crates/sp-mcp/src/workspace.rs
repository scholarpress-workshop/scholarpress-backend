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
    out.sort_by_key(|w| std::cmp::Reverse(w.mtime));
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
            let name = read_profile_name(&id_path.join("spec.yaml")).unwrap_or_else(|| id.clone());
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
        let available: Vec<String> = list_profiles(config)?.into_iter().map(|p| p.id).collect();
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

use sp_typst as typst;

pub fn compile_typst(
    config: &Config,
    workspace: &Path,
    entry_path: &Path,
    data: Option<&serde_json::Value>,
    out_name: Option<&str>,
) -> Result<PathBuf, SpMcpError> {
    if !workspace.is_dir() {
        return Err(SpMcpError::WorkspaceNotFound(
            workspace.display().to_string(),
            config.workspace_root.clone(),
        ));
    }
    let entry_abs = workspace.join(entry_path);
    if !entry_abs.is_file() {
        return Err(SpMcpError::Compilation(format!(
            "entry file not found: {}",
            entry_abs.display()
        )));
    }
    if let Some(d) = data {
        std::fs::write(workspace.join("data.json"), serde_json::to_string(d)?)?;
    }

    let source = std::fs::read_to_string(&entry_abs)?;
    // sp-typst compiles the entry file from --root=workspace.
    let bytes = typst::compile(&source, Some(workspace))
        .map_err(|e| SpMcpError::Compilation(format!("typst compile failed: {}", e)))?;

    let stem = entry_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "out".to_string());
    let name = out_name
        .map(String::from)
        .unwrap_or_else(|| format!("{}.pdf", stem));

    std::fs::create_dir_all(workspace.join("out"))?;
    let out_path = workspace.join("out").join(&name);
    std::fs::write(&out_path, bytes)?;
    Ok(out_path)
}

use sp_check as check;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckOutcome {
    pub id: String,
    pub status: String, // "PASS" | "FAIL" | "MANUAL" | "ERROR"
    pub message: String,
    pub page: Option<usize>,
}

pub fn check_pdf(
    _config: &Config,
    workspace: &Path,
    pdf_path: &Path,
) -> Result<Vec<CheckOutcome>, SpMcpError> {
    if !workspace.is_dir() {
        return Err(SpMcpError::WorkspaceNotFound(
            workspace.display().to_string(),
            workspace.to_path_buf(),
        ));
    }
    let spec_path = workspace.join("spec.yaml");
    if !spec_path.is_file() {
        return Err(SpMcpError::SpecMissing(spec_path));
    }
    let pdf_abs = if pdf_path.is_absolute() {
        pdf_path.to_path_buf()
    } else {
        workspace.join(pdf_path)
    };
    if !pdf_abs.is_file() {
        return Err(SpMcpError::Check(format!(
            "pdf not found: {}",
            pdf_abs.display()
        )));
    }

    let spec = check::spec::load_spec(&spec_path)
        .map_err(|e| SpMcpError::Check(format!("failed to load spec: {}", e)))?;
    let options = check::engine::CheckOptions::default();
    let results = check::engine::run_checks(&spec, &pdf_abs, &options)
        .map_err(|e| SpMcpError::Check(format!("check run failed: {}", e)))?;
    let report = check::report::build_report(results);

    let outcomes = report
        .results
        .into_iter()
        .map(|r| CheckOutcome {
            id: r.check_id,
            status: r.status.as_str().to_string(),
            message: r.detail,
            page: r.evidence.first().map(|e| e.page),
        })
        .collect();
    Ok(outcomes)
}

use sp_extract as extract;

pub fn extract_document(file_path: &Path) -> Result<serde_json::Value, SpMcpError> {
    if !file_path.is_file() {
        return Err(SpMcpError::Extraction(format!(
            "file not found: {}",
            file_path.display()
        )));
    }
    let bytes = std::fs::read(file_path)?;
    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let doc = match ext.as_str() {
        "pdf" => extract::extract_pdf(&bytes),
        "docx" => extract::extract_docx(&bytes),
        other => {
            return Err(SpMcpError::Extraction(format!(
                "unsupported extension: .{}",
                other
            )));
        }
    }
    .map_err(|e| SpMcpError::Extraction(e.to_string()))?;

    serde_json::to_value(doc).map_err(|e| SpMcpError::Extraction(e.to_string()))
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
        assert!(path
            .join("template")
            .join("sections")
            .join("ch.typ")
            .is_file());
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

    #[test]
    fn compile_typst_produces_pdf_or_skips_if_no_typst_binary() {
        let ws = local_tempdir();
        let tmpl = ws.join("template.typ");
        fs::write(&tmpl, "= Hello, world!\n").unwrap();
        let cfg = Config::new(PathBuf::from("/c"), PathBuf::from("/w"));

        let result = compile_typst(&cfg, &ws, Path::new("template.typ"), None, None);
        match result {
            Ok(p) => {
                assert!(p.is_file(), "pdf should exist");
                let bytes = fs::read(&p).unwrap();
                assert!(bytes.starts_with(b"%PDF"), "should be a valid PDF");
            }
            Err(SpMcpError::Compilation(msg)) if msg.contains("typst") => {
                eprintln!("SKIP: typst not on PATH ({})", msg);
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn check_pdf_against_iu_baseline() {
        // Walk up from CARGO_MANIFEST_DIR until we find a directory containing
        // a sibling scholarpress-catalog. This works in both regular and worktree
        // layouts.
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let catalog = (0..6)
            .map(|i| {
                let mut p = manifest_dir.clone();
                for _ in 0..=i {
                    p.pop();
                }
                p.join("scholarpress-catalog")
            })
            .find(|p| p.is_dir());
        let catalog = match catalog {
            Some(p) => p,
            None => {
                eprintln!(
                    "SKIP: scholarpress-catalog not found near {}",
                    manifest_dir.display()
                );
                return;
            }
        };
        let baseline = catalog.join("institutions/iu/tests/fixtures/baseline.pdf");
        if !baseline.is_file() {
            eprintln!("SKIP: IU baseline fixture not present (run bash compile.sh)");
            return;
        }

        let ws = local_tempdir();
        fs::write(
            ws.join("spec.yaml"),
            fs::read_to_string(catalog.join("institutions/iu/spec.yaml")).unwrap(),
        )
        .unwrap();

        let cfg = Config::new(catalog, PathBuf::from("/w"));
        let outcomes = check_pdf(&cfg, &ws, &baseline).unwrap();
        assert!(!outcomes.is_empty(), "expected at least one check result");
    }

    #[test]
    fn extract_document_on_garbage_returns_error() {
        let f = local_tempdir().join("bad.pdf");
        fs::write(&f, b"not a real pdf").unwrap();
        let result = extract_document(&f);
        assert!(matches!(result, Err(SpMcpError::Extraction(_))));
    }

    #[test]
    fn extract_document_unsupported_extension_errors() {
        let f = local_tempdir().join("foo.xyz");
        fs::write(&f, b"whatever").unwrap();
        let result = extract_document(&f);
        assert!(matches!(result, Err(SpMcpError::Extraction(_))));
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
