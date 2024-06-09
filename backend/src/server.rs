use std::{io, sync::Arc};

use axum::{extract::State, http::HeaderMap, routing::post, Router};
use bollard::Docker;
use domain::{Branch, Pipeline, PipelineId, PipelineStatus, Trigger, TriggerEvent};
use source_control::{github::GitHub, CheckStatus, SourceControl, SourceControlInstallation};
use tokio::signal::{self, unix::SignalKind};

use crate::{
    config::AppConfig,
    parser::parse_pipeline,
    runner,
    webhook::{handle_webhook, Callbacks},
};

pub struct Server {
    app: Router,
}

#[derive(Clone)]
struct RequestState {
    config: AppConfig,
    callbacks: Callbacks,
}

impl Server {
    pub fn new(config: AppConfig) -> Self {
        let app = Router::new()
            .route(
                "/webhook",
                post(
                    |State(RequestState { config, callbacks }): State<RequestState>,
                     headers: HeaderMap,
                     body: String| {
                        handle_webhook(config, callbacks, headers, body)
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

        println!("listening on {}", listener.local_addr().unwrap());

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

        for file in pipeline_files {
            let installation = installation.clone();
            let commit = commit.clone();
            let trigger = trigger.clone();

            let configuration = parse_pipeline(&file, &installation).await.unwrap();

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
        }
    });
}
