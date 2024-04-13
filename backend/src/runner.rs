use crate::domain::{Pipeline, Step};
use bollard::{
    container::{Config, CreateContainerOptions, LogsOptions},
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
        let runner_instance = PipelineRunnerInstance {
            docker: &self.docker,
            pipeline,
        };
        runner_instance.run().await
    }
}

struct PipelineRunnerInstance<'a> {
    docker: &'a Docker,
    pipeline: &'a Pipeline,
}

impl<'a> PipelineRunnerInstance<'a> {
    async fn run(&self) {
        for step in &self.pipeline.steps {
            self.run_step(step).await;
        }
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
        let result = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name: format!("pipeline-{}-step-{}", self.pipeline.id, step.id),
                    platform: None,
                }),
                Config {
                    image: Some(step.configuration.image.clone()),
                    entrypoint: step
                        .configuration
                        .commands
                        .clone()
                        .map(|commands| vec!["sh".into(), "-c".into(), commands.join("; ")]),
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

        for log in logs {
            print!("{log}")
        }
    }

    async fn remove_container_for_step(&self, _step: &Step, container_name: &str) {
        self.docker
            .remove_container(container_name, None)
            .await
            .unwrap()
    }
}
