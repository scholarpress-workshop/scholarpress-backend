use anyhow::Context;
use sp_mcp::config::{Config, ServerOptions};
use sp_mcp::{update_config, GooseSetup};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("setup-goose") {
        let setup = GooseSetup {
            config_path: required_setup_arg(&mut args, "--config")?,
            command: required_setup_arg(&mut args, "--command")?,
            catalog_path: required_setup_arg(&mut args, "--catalog")?,
            workspace_root: required_setup_arg(&mut args, "--workspace-root")?,
            typst_path: required_setup_arg(&mut args, "--typst")?,
            pandoc_path: required_setup_arg(&mut args, "--pandoc")?,
        };
        if let Some(unexpected) = args.next() {
            anyhow::bail!("unexpected setup-goose argument: {unexpected}");
        }
        let backup = update_config(&setup).context("updating Goose config")?;
        println!("Goose config updated: {}", setup.config_path.display());
        println!("Backup: {}", backup.display());
        println!("Workspace: {}", setup.workspace_root.display());
        return Ok(());
    }

    let config = Config::from_env().context("loading sp-mcp config")?;
    let options = ServerOptions::from_env()
        .and_then(|options| options.apply_args(std::env::args().skip(1)))
        .context("loading sp-mcp server options")?;
    sp_mcp::run_with_options(config, options).await
}

fn required_setup_arg<I>(args: &mut I, expected: &str) -> anyhow::Result<PathBuf>
where
    I: Iterator<Item = String>,
{
    let actual = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing {expected} for setup-goose"))?;
    if actual != expected {
        anyhow::bail!("expected {expected}, got {actual}");
    }
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("missing value after {expected}"))
}
