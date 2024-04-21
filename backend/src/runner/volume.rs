use super::error::RunnerError as Error;
use bollard::{volume::CreateVolumeOptions, Docker};

pub struct Volume<'a> {
    pub name: String,
    docker: &'a Docker,
}

impl<'a> Volume<'a> {
    pub async fn create(docker: &'a Docker, name: String) -> Result<Self, Error> {
        let volume = docker
            .create_volume(CreateVolumeOptions {
                name: name.as_str(),
                ..Default::default()
            })
            .await
            .map(|_| Self { docker, name })?;

        Ok(volume)
    }

    pub async fn remove(&self) -> Result<(), Error> {
        Ok(self.docker.remove_volume(&self.name, None).await?)
    }
}
