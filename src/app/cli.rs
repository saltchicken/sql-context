use clap::Parser;


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Optional database connection string. If not provided, looks for DB_URL env var.
    #[arg(short, long)]
    pub db_url: Option<String>,
}