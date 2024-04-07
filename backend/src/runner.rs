use crate::domain::pipeline::{Pipeline, StepConfiguration};
use bollard::{
    container::{Config, CreateContainerOptions},
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
        for step in &pipeline.configuration.steps {
            self.run_step(step).await;
        }
    }

    async fn run_step(&self, step: &StepConfiguration) {
        self.pull_image_for_step(step).await;
        let container_id = self.create_container_for_step(step).await;
        self.run_container_for_step(step, container_id).await;
    }

    async fn pull_image_for_step(&self, step: &StepConfiguration) {
        let mut iter = step.image.split(":");
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

        for image in image {
            println!("{image:?}");
        }
    }

    async fn create_container_for_step(&self, step: &StepConfiguration) -> String {
        let result = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name: "rust-hello-world",
                    platform: None,
                }),
                Config {
                    image: Some(step.image.clone()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        result.id
    }

    async fn run_container_for_step(&self, _step: &StepConfiguration, container_id: String) {
        self.docker
            .start_container::<String>(&container_id, None)
            .await
            .unwrap();
    }
}
