use axum::{http::StatusCode, response::IntoResponse, routing::post, Router};
use bollard::Docker;
use domain::{Pipeline, PipelineId, PipelineStatus};
use source_control::{github::GitHub, CheckStatus, SourceControl, SourceControlInstallation};
use std::io;
use tokio::signal::{self, unix::SignalKind};

mod runner;

pub async fn main() -> Result<(), io::Error> {
    start_http_server().await?;

    Ok(())
}

async fn start_http_server() -> Result<(), io::Error> {
    let app = Router::new().route("/api/webhook", post(handle_webhook));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:42069").await?;

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler")
    };

    let terminate = async {
        signal::unix::signal(SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {}
    }
}

async fn handle_webhook() -> impl IntoResponse {
    tokio::spawn(async {
        let github_app_id = std::env::var("GITHUB_APP_ID").unwrap().parse().unwrap();
        let github_private_key = std::env::var("GITHUB_PRIVATE_KEY").unwrap();
        let commit = "HEAD";

        let github = GitHub::build(github_app_id, &github_private_key).unwrap();
        let installation = github
            .get_installation("JakobStaudinger", "rust-ci")
            .await
            .unwrap();

        installation
            .update_status_check(commit, CheckStatus::Running)
            .await
            .unwrap();

        let configuration = installation
            .read_file_contents(".ci/lint-and-test.json")
            .await
            .unwrap();
        let configuration = serde_json::from_str(&configuration).unwrap();

        let mut pipeline = Pipeline::new(PipelineId::new(1), configuration);

        let docker = Docker::connect_with_socket_defaults().unwrap();
        let runner = runner::PipelineRunner::new(&docker);
        runner.run_pipeline(&mut pipeline).await.unwrap();

        installation
            .update_status_check(
                commit,
                match pipeline.status {
                    PipelineStatus::Passed => CheckStatus::Passed,
                    PipelineStatus::Failed => CheckStatus::Failed,
                    PipelineStatus::Pending => CheckStatus::Pending,
                    PipelineStatus::Running => CheckStatus::Running,
                },
            )
            .await
            .unwrap();
    });

    (StatusCode::CREATED, "OK")
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_test() {
        assert_eq!(1, 2);
    }
}
