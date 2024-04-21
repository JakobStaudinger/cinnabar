use bollard::errors::Error as DockerError;

#[derive(Debug)]
pub enum RunnerError {
    Docker(DockerError),
}

impl From<DockerError> for RunnerError {
    fn from(value: DockerError) -> Self {
        RunnerError::Docker(value)
    }
}
