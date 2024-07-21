use bollard::Docker;
use domain::build::{Branch, Pipeline, PipelineId, PipelineStatus, Trigger, TriggerEvent};
use source_control::{github::GitHub, CheckStatus, SourceControl, SourceControlInstallation};

use crate::{config::AppConfig, parser::parse_pipeline, runner};

pub fn handle_trigger(trigger: Trigger, config: AppConfig) {
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

        let pipeline_files = installation.read_folder(".cinnabar", commit).await.unwrap();
        let pipeline_files = pipeline_files
            .items
            .into_iter()
            .filter(|file| file.path.starts_with(".cinnabar/pipelines/"));

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
