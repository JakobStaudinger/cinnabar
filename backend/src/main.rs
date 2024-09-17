use api::Server;
use config::AppConfig;

mod api;
mod config;
mod orchestrator;
mod parser;
mod runner;

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = AppConfig::from_environment()?;

    let server = Server::new(config);

    server
        .start()
        .await
        .map_err(|e| format!("Failed to start HTTP server {e}"))?;

    Ok(())
}
