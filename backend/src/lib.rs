use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use bollard::Docker;
use domain::{Pipeline, PipelineId, PipelineStatus};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha256;
use source_control::{github::GitHub, CheckStatus, SourceControl, SourceControlInstallation};
use std::io;
use tokio::signal::{self, unix::SignalKind};

mod runner;

pub async fn main() -> Result<(), io::Error> {
    start_http_server().await?;

    Ok(())
}

async fn start_http_server() -> Result<(), io::Error> {
    let app = Router::new().route("/webhook", post(handle_webhook));

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

async fn handle_webhook(headers: HeaderMap, body: String) -> impl IntoResponse {
    let webhook_secret = SecretString::new(std::env::var("GITHUB_WEBHOOK_SECRET").unwrap());

    if let Err(message) = verify_checksum(&headers, &body, &webhook_secret) {
        return (StatusCode::BAD_REQUEST, message);
    }

    if let Some(event) = headers.get("x-github-event") {
        match event.to_str() {
            Ok("push") => {
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
                    let mut runner = runner::PipelineRunner {
                        docker: &docker,
                        access_token: installation.get_access_token(),
                        pipeline: &mut pipeline,
                    };
                    runner.run().await.unwrap();

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
            Ok(_) => (StatusCode::NO_CONTENT, "OK"),
            Err(_) => (StatusCode::BAD_REQUEST, "Failed to parse event"),
        }
    } else {
        (StatusCode::BAD_REQUEST, "Missing header x-github-event")
    }
}

fn verify_checksum(
    headers: &HeaderMap,
    body: &String,
    secret: &SecretString,
) -> Result<(), &'static str> {
    let expected_signature = headers
        .get("x-hub-signature-256")
        .ok_or("Missing header x-hub-signature-256")?
        .to_str()
        .map_err(|_| "Failed to parse x-hub-signature-256 header")?;

    let expected_signature = expected_signature
        .strip_prefix("sha256=")
        .ok_or("Malformed sha256 header")?;

    let expected_signature =
        hex::decode(expected_signature).map_err(|_| "Failed to parse sha256 signature")?;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.expose_secret().as_bytes())
        .map_err(|_| "Failed to hash payload")?;

    mac.update(body.as_bytes());

    mac.verify_slice(expected_signature.as_slice())
        .map_err(|_| "Failed to verify sha256 checksum")
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};
    use secrecy::SecretString;

    use crate::verify_checksum;

    #[test]
    fn verify_checksum_should_return_ok() {
        let secret = SecretString::new("It's a Secret to Everybody".to_string());
        let body = "Hello, World!".to_string();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_static(
                "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17",
            ),
        );

        assert_eq!(verify_checksum(&headers, &body, &secret), Ok(()));
    }

    #[test]
    fn verify_checksum_should_return_err_if_header_is_missing() {
        let secret = SecretString::new("It's a Secret to Everybody".to_string());
        let body = "Hello, World!".to_string();
        let headers = HeaderMap::new();

        assert_eq!(
            verify_checksum(&headers, &body, &secret),
            Err("Missing header x-hub-signature-256")
        );
    }

    #[test]
    fn verify_checksum_should_return_err_if_checksum_differs() {
        let secret = SecretString::new("It's a Secret to Everybody".to_string());
        let body = "Hello, World!".to_string();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_static(
                "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e16",
            ),
        );
        assert_eq!(
            verify_checksum(&headers, &body, &secret),
            Err("Failed to verify sha256 checksum")
        );
    }
}
