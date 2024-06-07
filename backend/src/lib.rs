mod parser;
mod runner;
mod webhook;

use axum::{routing::post, Router};
use bollard::Docker;
use domain::{Branch, Pipeline, PipelineId, PipelineStatus, Trigger, TriggerEvent};
use secrecy::SecretString;
use source_control::{github::GitHub, CheckStatus, SourceControl, SourceControlInstallation};
use std::{io, sync::Arc};
use tokio::{
    signal::{self, unix::SignalKind},
    task::JoinSet,
};

use crate::{parser::parse_pipeline, webhook::handle_webhook};

#[derive(Clone)]
struct AppConfig {
    github_webhook_secret: SecretString,
    github_app_id: u64,
    github_private_key: SecretString,
}

#[derive(Clone)]
struct Callbacks {
    trigger: Arc<dyn Send + Sync + Fn(Trigger, AppConfig)>,
}

#[derive(Clone)]
struct RequestState {
    config: AppConfig,
    callbacks: Callbacks,
}

pub async fn main() -> Result<(), String> {
    let config = build_config()?;

    start_http_server(config)
        .await
        .map_err(|e| format!("Failed to start HTTP server {e}"))?;

    Ok(())
}

async fn start_http_server(config: AppConfig) -> Result<(), io::Error> {
    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(RequestState {
            config,
            callbacks: Callbacks {
                trigger: Arc::new(handle_trigger),
            },
        });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:42069").await?;

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

fn build_config() -> Result<AppConfig, String> {
    let github_webhook_secret = SecretString::new(
        std::env::var("GITHUB_WEBHOOK_SECRET")
            .map_err(|_| "Please provide the GITHUB_WEBHOOK_SECRET environment variable")?,
    );
    let github_app_id = std::env::var("GITHUB_APP_ID")
        .map_err(|_| "Please provide the GITHUB_APP_ID environment variable")?
        .parse()
        .map_err(|_| "GITHUB_APP_ID needs to be an integer")?;
    let github_private_key = SecretString::new(
        std::env::var("GITHUB_PRIVATE_KEY")
            .map_err(|_| "Please provide the GITHUB_PRIVATE_KEY environment variable")?,
    );

    Ok(AppConfig {
        github_webhook_secret,
        github_app_id,
        github_private_key,
    })
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

fn handle_trigger(trigger: Trigger, config: AppConfig) {
    tokio::spawn(async move {
        let commit = match &trigger.event {
            TriggerEvent::Push {
                branch: Branch { commit, .. },
            } => commit,
            TriggerEvent::PullRequest {
                source: Branch { commit, .. },
                ..
            } => commit,
        };

        let github = GitHub::build(config.github_app_id, &config.github_private_key).unwrap();
        let installation = github
            .get_installation(
                &trigger.repository_owner,
                &trigger.repository_name,
                trigger.installation_id,
            )
            .await
            .unwrap();

        let pipeline_files = installation.read_folder(".ci", commit).await.unwrap();
        let pipeline_files = pipeline_files
            .items
            .into_iter()
            .filter(|file| file.path.starts_with(".ci/pipelines/"));

        let mut join_set = JoinSet::new();

        for file in pipeline_files {
            let installation = installation.clone();
            let commit = commit.clone();
            let trigger = trigger.clone();

            let configuration = parse_pipeline(&file, &installation).await.unwrap();

            join_set.spawn(async move {
                if configuration
                    .trigger
                    .iter()
                    .any(|trigger_configuration| trigger_configuration.matches(&trigger))
                {
                    let pipeline_id = rand::random();
                    let mut pipeline = Pipeline::new(PipelineId::new(pipeline_id), configuration);

                    installation
                        .update_status_check(
                            &commit,
                            &pipeline.configuration.name,
                            pipeline.id.0,
                            CheckStatus::Running,
                        )
                        .await
                        .unwrap();

                    let docker = Docker::connect_with_socket_defaults().unwrap();
                    let mut runner = runner::PipelineRunner {
                        docker: &docker,
                        access_token: installation.get_access_token(),
                        pipeline: &mut pipeline,
                    };
                    runner.run().await.unwrap();

                    installation
                        .update_status_check(
                            &commit,
                            &pipeline.configuration.name,
                            pipeline.id.0,
                            match pipeline.status {
                                PipelineStatus::Passed => CheckStatus::Passed,
                                PipelineStatus::Failed => CheckStatus::Failed,
                                PipelineStatus::Pending => CheckStatus::Pending,
                                PipelineStatus::Running => CheckStatus::Running,
                            },
                        )
                        .await
                        .unwrap();
                }
            });
        }

        while join_set.join_next().await.is_some() {}
    });
}
