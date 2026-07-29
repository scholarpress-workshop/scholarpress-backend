use anyhow::Context;
use sp_mcp::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env().context("loading sp-mcp config")?;
    sp_mcp::run(config).await
}
