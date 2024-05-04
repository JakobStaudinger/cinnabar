use bollard::Docker;
use futures::TryStreamExt;
use shared::domain::{Pipeline, Step};

use self::error::RunnerError as Error;
use self::{container::Container, volume::Volume};

mod container;
pub mod error;
mod volume;

pub struct PipelineRunner<'a> {
    docker: &'a Docker,
}

impl<'a> PipelineRunner<'a> {
    pub fn new(docker: &'a Docker) -> Self {
        Self { docker }
    }

    pub async fn run_pipeline(&self, pipeline: &Pipeline) -> Result<(), Error> {
        let runner_instance = PipelineRunnerInstance::new(self.docker, pipeline);
        runner_instance.run().await
    }
}

struct PipelineRunnerInstance<'a> {
    docker: &'a Docker,
    pipeline: &'a Pipeline,
}

impl<'a> PipelineRunnerInstance<'a> {
    fn new(docker: &'a Docker, pipeline: &'a Pipeline) -> Self {
        Self { docker, pipeline }
    }

    async fn run(&self) -> Result<(), Error> {
        let volume_name = format!("workspace-pipeline-{}", self.pipeline.id);
        let volume = Volume::create(self.docker, volume_name).await?;

        for step in &self.pipeline.steps {
            self.run_step(step, &volume).await?;
        }

        volume.remove().await?;

        Ok(())
    }

    async fn run_step(&self, step: &Step, volume: &Volume<'a>) -> Result<(), Error> {
        self.pull_image_for_step(step).await?;

        let container = Container::create(self.docker, self.pipeline, step, volume).await?;
        container.run().await?;
        container.remove().await?;

        Ok(())
    }

    async fn pull_image_for_step(&self, step: &Step) -> Result<(), Error> {
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
            .await?;

        let image_status = image.last().unwrap().status.as_ref().unwrap();

        println!("{image_status}");

        Ok(())
    }
}
