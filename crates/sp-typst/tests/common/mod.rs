use std::sync::OnceLock;

static TYPST_AVAILABLE: OnceLock<bool> = OnceLock::new();

pub fn has_typst_binary() -> bool {
    *TYPST_AVAILABLE.get_or_init(|| {
        std::process::Command::new("typst")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}
