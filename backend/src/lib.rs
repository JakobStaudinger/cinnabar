use axum::{http::StatusCode, response::IntoResponse, routing::post, Router};
use bollard::Docker;
use domain::{Pipeline, PipelineId};
use github::GitHub;
use source_control::{CheckStatus, SourceControl, SourceControlInstallation};
use std::io;

mod domain;
mod github;
mod runner;
mod source_control;

pub async fn main() -> Result<(), io::Error> {
    let app = Router::new().route("/api/webhook", post(handle_webhook));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:42069")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn handle_webhook() -> impl IntoResponse {
    tokio::spawn(async {
        let github_app_id = std::env::var("GITHUB_APP_ID").unwrap().parse().unwrap();
        let github_private_key = std::env::var("GITHUB_PRIVATE_KEY").unwrap();
        let commit = std::env::var("GITHUB_COMMIT").unwrap();

        let github = GitHub::build(github_app_id, &github_private_key).unwrap();
        let installation = github
            .get_installation("JakobStaudinger", "rust-ci")
            .await
            .unwrap();

        installation
            .update_status_check(&commit, CheckStatus::Pending)
            .await
            .unwrap();

        let configuration = installation
            .read_file_contents("assets/test-pipeline.json")
            .await
            .unwrap();
        let configuration = serde_json::from_str(&configuration).unwrap();

        let pipeline = Pipeline::new(PipelineId::new(1), configuration);

        let docker = Docker::connect_with_socket_defaults().unwrap();
        let runner = runner::PipelineRunner::new(&docker);
        runner.run_pipeline(&pipeline).await.unwrap();

        installation
            .update_status_check(&commit, CheckStatus::Passed)
            .await
            .unwrap();
    });

    (StatusCode::CREATED, "OK")
}
