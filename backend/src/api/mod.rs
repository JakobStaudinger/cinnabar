mod state;
mod webhook;

use axum::{extract::State, http::HeaderMap, routing::post, Router};
use std::{io, sync::Arc};
use tokio::signal::{self, unix::SignalKind};

use crate::{config::AppConfig, orchestrator::handle_trigger};

use state::RequestState;
use webhook::{handle_webhook, Callbacks};

pub struct Server {
    app: Router,
}

impl Server {
    pub fn new(config: AppConfig) -> Self {
        let app = Router::new()
            .route(
                "/webhook",
                post(
                    |state: State<RequestState>, headers: HeaderMap, body: String| {
                        handle_webhook(state, headers, body)
                    },
                ),
            )
            .with_state(RequestState {
                config,
                callbacks: Callbacks {
                    trigger: Arc::new(handle_trigger),
                },
            });

        Self { app }
    }

    pub async fn start(self) -> Result<(), io::Error> {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:42069").await?;

        println!("listening on {}", listener.local_addr()?);

        axum::serve(listener, self.app)
            .with_graceful_shutdown(shutdown_signal())
            .await
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");

        println!("Received SIGINT, shutting down");
    };

    let terminate = async {
        signal::unix::signal(SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;

        println!("Received SIGTERM, shutting down");
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {}
    }
}
