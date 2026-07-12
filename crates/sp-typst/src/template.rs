use serde_json::Value;
use std::path::Path;

pub struct TemplateSet {
    pub entry: String,
    pub files: Vec<TemplateFile>,
}

pub struct TemplateFile {
    pub path: String,
    pub content: String,
}

pub fn load_template(template_dir: &Path) -> Result<TemplateSet, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_typ_files(template_dir, template_dir, &mut files)?;

    if files.is_empty() {
        return Err("No .typ files found in template directory".into());
    }

    let entry = if files.iter().any(|f| f.path == "template.typ") {
        "template.typ".to_string()
    } else {
        files[0].path.clone()
    };

    Ok(TemplateSet { entry, files })
}

fn collect_typ_files(
    base: &Path,
    dir: &Path,
    files: &mut Vec<TemplateFile>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_typ_files(base, &path, files)?;
        } else if path.extension().map_or(false, |e| e == "typ") {
            let relative = path.strip_prefix(base)?.to_string_lossy().to_string();
            let content = std::fs::read_to_string(&path)?;
            files.push(TemplateFile {
                path: relative,
                content,
            });
        }
    }
    Ok(())
}

pub fn render_template(code: &str, variables: &std::collections::HashMap<String, Value>) -> String {
    let mut result = code.to_string();
    for (key, value) in variables {
        let placeholder = format!("{{{}}}", key.to_uppercase());
        let replacement = match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template_substitutes_variables() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("TITLE".to_string(), serde_json::Value::String("My Dissertation".to_string()));
        let result = render_template("#let title = \"{TITLE}\"", &vars);
        assert_eq!(result, "#let title = \"My Dissertation\"");
    }

    #[test]
    fn test_load_template_empty_dir() {
        let tmp = std::env::temp_dir().join("sp-typst-empty-test");
        std::fs::create_dir_all(&tmp).ok();
        let result = load_template(&tmp);
        assert!(result.is_err());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
