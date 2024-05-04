use octocrab::{Error as OctocrabError, GitHubError as OctocrabGitHubError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitHubError {
    #[error(transparent)]
    GitHub(#[from] OctocrabGitHubError),
    #[error(transparent)]
    Octocrab(#[from] OctocrabError),
    #[error("{0}")]
    Generic(String),
}
