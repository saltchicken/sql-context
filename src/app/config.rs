use crate::app::cli::Cli;
use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub db_url: String,
    pub db_name: String,
    pub collect_samples: bool,
    pub ignore_tables: Vec<String>,
}

pub fn resolve_config(cli: Cli) -> Result<AppConfig> {
    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    let db_url = cli
        .db_url
        .or_else(|| env::var("DB_URL").ok())
        .context("DB_URL must be set via --db-url or in .env/environment variables")?;

    let db_name = db_url.split('/').last().unwrap_or("Unknown").to_string();


    let ignore_tables = cli.ignore.unwrap_or_default();

    Ok(AppConfig {
        db_url,
        db_name,
        collect_samples: true,
        ignore_tables,
    })
}