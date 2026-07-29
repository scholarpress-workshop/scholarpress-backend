pub mod config;
pub mod error;
pub mod tools;
pub mod workspace;

use crate::config::Config;
use crate::tools::ScholarPressService;
use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

pub async fn run(config: Config) -> Result<()> {
    let service = ScholarPressService::new(config).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
