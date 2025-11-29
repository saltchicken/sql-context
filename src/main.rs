use env_logger::Env;
use sql_context::app;


#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    if let Err(e) = app::run().await {
        log::error!("❌ Application error: {:?}", e);
        std::process::exit(1);
    }
}