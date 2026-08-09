use anyhow::Context;
use sp_mcp::config::{Config, ServerOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env().context("loading sp-mcp config")?;
    let options = ServerOptions::from_env()
        .and_then(|options| options.apply_args(std::env::args().skip(1)))
        .context("loading sp-mcp server options")?;
    sp_mcp::run_with_options(config, options).await
}
