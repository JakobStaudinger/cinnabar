use bollard::Docker;
use domain::{
    Branch, Pipeline, PipelineConfiguration, PipelineId, PipelineStatus, Trigger, TriggerEvent,
};
use itertools::Itertools;
use source_control::{
    github::{error::GitHubError, GitHub, GitHubInstallation},
    CheckStatus, File, SourceControl, SourceControlInstallation,
};
use tokio::task::JoinSet;

use crate::{
    config::AppConfig,
    parser::{error::ParserError, parse_pipeline},
    runner,
};

pub async fn handle_trigger(trigger: Trigger, config: AppConfig) -> Result<(), ()> {
    let commit = extract_commit(&trigger);
    let installation = get_installation(&trigger, &config).await.map_err(|_| ())?;

    let pipeline_files = find_pipeline_files(commit, &installation)
        .await
        .map_err(|_| ())?;

    let parse_results = parse_pipeline_files(&trigger, &installation, pipeline_files).await;
    let (pipelines, parser_errors): (Vec<_>, Vec<_>) = parse_results.into_iter().partition_result();

    if !parser_errors.is_empty() {
        return Err(());
    }

    let matched_pipelines: Vec<_> = pipelines.into_iter().flatten().collect();
    if matched_pipelines.is_empty() {
        return Ok(());
    }

    for configuration in matched_pipelines {
        tokio::spawn(process_pipeline(
            installation.clone(),
            commit.clone(),
            configuration,
        ));
    }

    Ok(())
}

fn extract_commit(trigger: &Trigger) -> &String {
    match &trigger.event {
        TriggerEvent::Push {
            branch: Branch { commit, .. },
        } => commit,
        TriggerEvent::PullRequest {
            source: Branch { commit, .. },
            ..
        } => commit,
    }
}

async fn get_installation(
    trigger: &Trigger,
    config: &AppConfig,
) -> Result<GitHubInstallation, GitHubError> {
    let github = GitHub::build(config.github.app_id, &config.github.private_key)?;

    github
        .get_installation(
            &trigger.repository_owner,
            &trigger.repository_name,
            trigger.installation_id,
        )
        .await
}

async fn find_pipeline_files(
    commit: &str,
    installation: &GitHubInstallation,
) -> Result<impl Iterator<Item = File>, GitHubError> {
    Ok(installation
        .read_folder(".cinnabar", commit)
        .await?
        .items
        .into_iter()
        .filter(|file| file.path.starts_with(".cinnabar/pipelines/")))
}

async fn parse_pipeline_files(
    trigger: &Trigger,
    installation: &GitHubInstallation,
    pipeline_files: impl Iterator<Item = File>,
) -> Vec<Result<Option<PipelineConfiguration>, ParserError>> {
    let mut join_set = JoinSet::new();

    for file in pipeline_files {
        let installation = installation.clone();
        let trigger = trigger.clone();

        join_set.spawn(async move {
            let configuration = parse_pipeline(&file, &installation).await?;

            if configuration
                .trigger
                .iter()
                .any(|trigger_configuration| trigger_configuration.matches(&trigger))
            {
                Ok::<_, ParserError>(Some(configuration))
            } else {
                Ok(None)
            }
        });
    }

    let mut results = Vec::with_capacity(join_set.len());

    while let Some(result) = join_set.join_next().await {
        let configuration = result.unwrap();
        results.push(configuration);
    }

    results
}

async fn process_pipeline(
    installation: GitHubInstallation,
    commit: String,
    configuration: PipelineConfiguration,
) {
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
