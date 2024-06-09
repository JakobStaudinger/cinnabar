use config::build_config;
use server::Server;

mod config;
mod parser;
mod runner;
mod server;
mod webhook;

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
