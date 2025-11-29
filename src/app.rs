pub mod cli;
pub mod config;
pub mod formatter;
pub mod inspector;
pub mod models;

use anyhow::{Context, Result};
use clap::Parser;
use sqlx::postgres::PgPoolOptions;

use self::cli::Cli;
use self::config::{AppConfig, resolve_config};
use self::formatter::OutputGenerator;
use self::inspector::Inspector;

// Connects, Scans, and Formats in one go.
pub async fn generate_report(config: &AppConfig) -> Result<String> {
    // 1. Connect
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.db_url)
        .await
        .context("Failed to connect to database")?;

    // 2. Scan (Inspector)
    // ‼️ Pass the collect_samples config option here
    let inspector = Inspector::new(&pool, config.collect_samples);
    let table_data = inspector.scan().await?;

    // 3. Format (OutputGenerator)
    let output = OutputGenerator::generate_markdown(&config.db_name, &table_data)?;

    Ok(output)
}

pub async fn run() -> Result<()> {
    // 1. Parse Args
    let args = Cli::parse();

    // 2. Resolve Config
    let config = resolve_config(args)?;

    // 3. Generate
    let output = generate_report(&config).await?;

    // 4. Output
    print!("{}", output);

    Ok(())
}

