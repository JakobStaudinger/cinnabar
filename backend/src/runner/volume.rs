use bollard::{errors::Error, volume::CreateVolumeOptions, Docker};

pub struct Volume<'a> {
    pub name: String,
    docker: &'a Docker,
}

impl<'a> Volume<'a> {
    pub async fn create(docker: &'a Docker, name: String) -> Result<Self, Error> {
        docker
            .create_volume(CreateVolumeOptions {
                name: name.as_str(),
                ..Default::default()
            })
            .await
            .map(|_| Self { docker, name })
    }

    pub async fn remove(&self) -> Result<(), Error> {
        self.docker.remove_volume(&self.name, None).await
    }
}
