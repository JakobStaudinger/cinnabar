use super::error::RunnerError as Error;
use super::volume::Volume;
use domain::{Pipeline, Step};

use bollard::{
    container::{Config, CreateContainerOptions, LogOutput, LogsOptions},
    errors::Error::DockerContainerWaitError,
    secret::{ContainerWaitResponse, HostConfig},
    Docker,
};
use futures::TryStreamExt;
use secrecy::{ExposeSecret, SecretString};

pub struct Container<'a> {
    pub name: String,
    docker: &'a Docker,
}

pub struct ContainerExitCode(pub i64);

impl ContainerExitCode {
    pub fn is_ok(&self) -> bool {
        self.0 == 0
    }

    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }
}

impl<'a> Container<'a> {
    pub async fn create(
        docker: &'a Docker,
        pipeline: &Pipeline,
        step: &Step,
        volume: &Volume<'a>,
        access_token: &SecretString,
    ) -> Result<Self, Error> {
        let commands = step
            .configuration
            .commands
            .as_ref()
            .map(|commands| commands.join("; "));

        let entrypoint = include_str!("./entrypoint.sh");
        let workspace_directory = "/ci/src";

        let container = docker
            .create_container(
                Some(CreateContainerOptions {
                    name: format!("pipeline-{}-step-{}", pipeline.id, step.id),
                    platform: None,
                }),
                Config {
                    image: Some(step.configuration.image.as_str()),
                    working_dir: Some(workspace_directory),
                    tty: Some(true),
                    env: Some(vec![
                        format!(
                            "NETRC_CONTENT=machine github.com login x-oauth-token password {}",
                            access_token.expose_secret()
                        )
                        .as_str(),
                        format!("SCRIPT={}", entrypoint).as_str(),
                        format!("COMMANDS={}", commands.unwrap_or_default()).as_str(),
                    ]),
                    entrypoint: Some(vec![
                        "/bin/sh",
                        "-c",
                        "echo \"$SCRIPT\" \"$COMMANDS\" | /bin/sh",
                    ]),
                    host_config: Some(HostConfig {
                        binds: Some(vec![format!("{}:{}", volume.name, workspace_directory)]),
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

    pub async fn run(&self) -> Result<ContainerExitCode, Error> {
        self.docker
            .start_container::<String>(&self.name, None)
            .await?;

        let result = self
            .docker
            .wait_container::<String>(&self.name, None)
            .try_collect::<Vec<_>>()
            .await;

        let exit_code = match result.as_deref() {
            Ok([ContainerWaitResponse { status_code, .. }, ..]) => ContainerExitCode(*status_code),
            Err(DockerContainerWaitError { code, .. }) => ContainerExitCode(*code),
            _ => {
                return Err(Error::Generic(
                    "Failed to get container exit_code".to_owned(),
                ))
            }
        };

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

        Ok(exit_code)
    }

    pub async fn remove(&self) -> Result<(), Error> {
        Ok(self.docker.remove_container(&self.name, None).await?)
    }
}
