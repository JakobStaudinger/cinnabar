use bollard::errors::Error as DockerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error(transparent)]
    Docker(#[from] DockerError),
}
