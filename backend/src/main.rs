use api::Server;
use config::build_config;

mod api;
mod config;
mod orchestrator;
mod parser;
mod runner;

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = build_config()?;

    let server = Server::new(config);

    server
        .start()
        .await
        .map_err(|e| format!("Failed to start HTTP server {e}"))?;

    Ok(())
}
