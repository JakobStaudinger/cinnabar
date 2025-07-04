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
    pub docker: &'a Docker,
    pub access_token: &'a SecretString,
    pub pipeline: &'a mut Pipeline,
}

impl<'a> PipelineRunner<'a> {
    pub async fn run(&mut self) -> Result<(), Error> {
        self.create_cache_volumes().await?;

        let workspace_volume = format!("workspace-pipeline-{}", self.pipeline.id);
        let workspace_volume = Volume::create(self.docker, workspace_volume).await?;

        let pipeline_status = self.run_pipeline(&workspace_volume).await;

        workspace_volume.remove().await?;

        self.pipeline.status = pipeline_status?;

        Ok(())
    }

    async fn create_cache_volumes(&self) -> Result<(), Error> {
        let cache_volumes =
            self.pipeline
                .configuration
                .steps
                .iter()
                .flat_map(|step| match &step.cache {
                    Some(vec) => vec.clone(),
                    None => vec![],
                });

        for cache in cache_volumes {
            Volume::create(self.docker, cache.clone()).await?;
        }

        Ok(())
    }

    async fn run_pipeline(&self, workspace_volume: &Volume<'a>) -> Result<PipelineStatus, Error> {
        for step in &self.pipeline.steps {
            let exit_code = self.run_step(step, workspace_volume).await?;

            if exit_code.is_err() {
                return Ok(PipelineStatus::Failed);
            }
        }

        Ok(PipelineStatus::Passed)
    }

    async fn run_step(&self, step: &Step, volume: &Volume<'a>) -> Result<ContainerExitCode, Error> {
        self.pull_image_for_step(step).await?;

        let container =
            Container::create(self.docker, self.pipeline, step, volume, self.access_token).await?;
        let exit_code = container.run().await;
        container.remove().await?;

        exit_code
    }

    async fn pull_image_for_step(&self, step: &Step) -> Result<(), Error> {
        let image =
            self.docker
                .create_image(
                    Some(bollard::image::CreateImageOptions {
                        from_image: step.configuration.image.to_string().as_str(),
                        tag: step.configuration.image.tag.as_deref().unwrap_or("latest"),
                        ..Default::default()
                    }),
                    None,
                    step.configuration.image.hostname.as_deref().and_then(
                        |hostname| match hostname {
                            _ => None,
                        },
                    ),
                )
                .try_collect::<Vec<_>>()
                .await?;

        let image_status = image.last().unwrap().status.as_ref().unwrap();

        println!("{image_status}");

        Ok(())
    }
}
