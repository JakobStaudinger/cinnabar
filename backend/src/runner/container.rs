use super::error::RunnerError as Error;
use super::volume::Volume;
use crate::domain::{Pipeline, Step};

use bollard::{
    container::{Config, CreateContainerOptions, LogOutput, LogsOptions},
    secret::HostConfig,
    Docker,
};
use futures::TryStreamExt;

pub struct Container<'a> {
    pub name: String,
    docker: &'a Docker,
}

impl<'a> Container<'a> {
    pub async fn create(
        docker: &'a Docker,
        pipeline: &Pipeline,
        step: &Step,
        volume: &Volume<'a>,
    ) -> Result<Self, Error> {
        let commands = step
            .configuration
            .commands
            .as_ref()
            .map(|commands| commands.join("; "));

        let container = docker
            .create_container(
                Some(CreateContainerOptions {
                    name: format!("pipeline-{}-step-{}", pipeline.id, step.id),
                    platform: None,
                }),
                Config {
                    image: Some(step.configuration.image.as_str()),
                    working_dir: Some("/ci/src"),
                    tty: Some(true),
                    env: Some(vec!["PS4=> "]),
                    entrypoint: commands
                        .as_ref()
                        .map(|commands| vec!["sh", "-x", "-e", "-c", commands.as_str()]),
                    host_config: Some(HostConfig {
                        binds: Some(vec![format!("{}:/ci/src", volume.name)]),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await
            .map(|result| Self {
                name: result.id,
                docker,
            })?;

        Ok(container)
    }

    pub async fn run(&self) -> Result<(), Error> {
        self.docker
            .start_container::<String>(&self.name, None)
            .await?;

        let result = self
            .docker
            .wait_container::<String>(&self.name, None)
            .try_collect::<Vec<_>>()
            .await?;

        for result in result {
            println!("{result:?}");
        }

        let logs = self
            .docker
            .logs(
                &self.name,
                Some(LogsOptions::<&str> {
                    timestamps: true,
                    stdout: true,
                    stderr: true,
                    ..Default::default()
                }),
            )
            .try_collect::<Vec<_>>()
            .await?;

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

        Ok(())
    }

    pub async fn remove(&self) -> Result<(), Error> {
        Ok(self.docker.remove_container(&self.name, None).await?)
    }
}
