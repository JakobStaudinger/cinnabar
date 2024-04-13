use crate::domain::{Pipeline, Step};
use bollard::{
    container::{Config, CreateContainerOptions, LogOutput, LogsOptions},
    secret::HostConfig,
    volume::CreateVolumeOptions,
    Docker,
};
use futures::TryStreamExt;

pub struct PipelineRunner {
    docker: Docker,
}

impl PipelineRunner {
    pub fn new() -> Self {
        let docker = Docker::connect_with_socket_defaults().unwrap();
        Self { docker }
    }

    pub async fn run_pipeline(&self, pipeline: &Pipeline) {
        let runner_instance = PipelineRunnerInstance::new(&self.docker, pipeline);
        runner_instance.run().await
    }
}

struct PipelineRunnerInstance<'a> {
    docker: &'a Docker,
    pipeline: &'a Pipeline,
    workspace_volume_name: String,
}

impl<'a> PipelineRunnerInstance<'a> {
    fn new(docker: &'a Docker, pipeline: &'a Pipeline) -> Self {
        let workspace_volume_name = format!("workspace-pipeline-{}", pipeline.id);

        Self {
            docker,
            pipeline,
            workspace_volume_name,
        }
    }

    async fn run(&self) {
        self.create_workspace_volume().await;

        for step in &self.pipeline.steps {
            self.run_step(step).await;
        }

        self.clean_up_workspace_volume().await
    }

    async fn create_workspace_volume(&self) {
        self.docker
            .create_volume(CreateVolumeOptions {
                name: self.workspace_volume_name.as_str(),
                ..Default::default()
            })
            .await
            .unwrap();
    }

    async fn run_step(&self, step: &Step) {
        self.pull_image_for_step(step).await;
        let container_id = self.create_container_for_step(step).await;
        self.run_container_for_step(step, &container_id).await;
        self.remove_container_for_step(step, &container_id).await;
    }

    async fn pull_image_for_step(&self, step: &Step) {
        let mut iter = step.configuration.image.split(':');
        let image_name = iter.next().unwrap();
        let image_tag = iter.next().unwrap_or("latest");

        let image = self
            .docker
            .create_image(
                Some(bollard::image::CreateImageOptions {
                    from_image: image_name,
                    tag: image_tag,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        let image_status = image.last().unwrap().status.as_ref().unwrap();

        println!("{image_status}");
    }

    async fn create_container_for_step(&self, step: &Step) -> String {
        let commands = step
            .configuration
            .commands
            .as_ref()
            .map(|commands| commands.join("; "));

        let result = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name: format!("pipeline-{}-step-{}", self.pipeline.id, step.id),
                    platform: None,
                }),
                Config {
                    image: Some(step.configuration.image.as_str()),
                    working_dir: Some("/ci/src"),
                    tty: Some(true),
                    entrypoint: commands
                        .as_ref()
                        .map(|commands| vec!["sh", "-x", "-e", "-c", commands.as_str()]),
                    host_config: Some(HostConfig {
                        binds: Some(vec![format!(
                            "workspace-pipeline-{}:/ci/src",
                            self.pipeline.id
                        )]),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        result.id
    }

    async fn run_container_for_step(&self, _step: &Step, container_name: &str) {
        self.docker
            .start_container::<String>(container_name, None)
            .await
            .unwrap();

        let result = self
            .docker
            .wait_container::<String>(container_name, None)
            .try_collect::<Vec<_>>()
            .await;

        match result {
            Ok(result) => {
                for result in result {
                    println!("{result:?}");
                }
            }
            Err(err) => println!("{err:?}"),
        };

        let logs = self
            .docker
            .logs(
                container_name,
                Some(LogsOptions::<&str> {
                    timestamps: true,
                    stdout: true,
                    stderr: true,
                    ..Default::default()
                }),
            )
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        let mut logs: Vec<_> = logs
            .into_iter()
            .map(|log| match log {
                LogOutput::Console { message } => message,
                LogOutput::StdOut { message } => message,
                LogOutput::StdErr { message } => message,
                LogOutput::StdIn { message } => message,
            })
            .map(|message| {
                let mut iter = message.splitn(2, |char| *char == b' ');
                let timestamp = iter.next().unwrap();
                let message = iter.next().unwrap();
                (
                    String::from_utf8_lossy(timestamp).to_string(),
                    String::from_utf8_lossy(message).to_string(),
                )
            })
            .collect();
        logs.sort_by_key(|(t, _m)| t.to_string());

        for log in logs {
            print!("{}", log.1)
        }
    }

    async fn remove_container_for_step(&self, _step: &Step, container_name: &str) {
        self.docker
            .remove_container(container_name, None)
            .await
            .unwrap()
    }

    async fn clean_up_workspace_volume(&self) {
        self.docker
            .remove_volume(&self.workspace_volume_name, None)
            .await
            .unwrap();
    }
}
