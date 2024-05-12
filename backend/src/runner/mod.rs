use bollard::Docker;
use domain::{Pipeline, PipelineStatus, Step};
use futures::TryStreamExt;

use self::container::ContainerExitCode;
use self::error::RunnerError as Error;
use self::{container::Container, volume::Volume};
use secrecy::SecretString;

mod container;
pub mod error;
mod volume;

pub struct PipelineRunner<'a> {
    docker: &'a Docker,
    access_token: &'a SecretString,
}

impl<'a> PipelineRunner<'a> {
    pub fn new(docker: &'a Docker, access_token: &'a SecretString) -> Self {
        Self {
            docker,
            access_token,
        }
    }

    pub async fn run_pipeline(&self, pipeline: &mut Pipeline) -> Result<(), Error> {
        let mut runner_instance = PipelineRunnerInstance::new(self.docker, pipeline);
        runner_instance.run(self.access_token).await
    }
}

struct PipelineRunnerInstance<'a> {
    docker: &'a Docker,
    pipeline: &'a mut Pipeline,
}

impl<'a> PipelineRunnerInstance<'a> {
    fn new(docker: &'a Docker, pipeline: &'a mut Pipeline) -> Self {
        Self { docker, pipeline }
    }

    async fn run(&mut self, access_token: &SecretString) -> Result<(), Error> {
        let volume_name = format!("workspace-pipeline-{}", self.pipeline.id);
        let volume = Volume::create(self.docker, volume_name).await?;

        let mut pipeline_status = PipelineStatus::Running;

        for step in &self.pipeline.steps {
            let exit_code = self.run_step(step, &volume, &access_token).await?;

            if exit_code.is_err() {
                pipeline_status = PipelineStatus::Failed;
                break;
            }
        }

        if pipeline_status != PipelineStatus::Failed {
            pipeline_status = PipelineStatus::Passed;
        }

        volume.remove().await?;

        self.pipeline.status = pipeline_status;

        Ok(())
    }

    async fn run_step(
        &self,
        step: &Step,
        volume: &Volume<'a>,
        access_token: &SecretString,
    ) -> Result<ContainerExitCode, Error> {
        self.pull_image_for_step(step).await?;

        let container =
            Container::create(self.docker, self.pipeline, step, volume, access_token).await?;
        let exit_code = container.run().await?;
        container.remove().await?;

        Ok(exit_code)
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
