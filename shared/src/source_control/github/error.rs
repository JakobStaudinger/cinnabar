use jsonwebtoken::errors::Error as JwtError;
use octocrab::{Error as OctocrabError, GitHubError as OctocrabGitHubError};
use thiserror::Error;
use url::ParseError;

#[derive(Debug, Error)]
pub enum GitHubError {
    #[error(transparent)]
    GitHub(#[from] OctocrabGitHubError),
    #[error(transparent)]
    Octocrab(#[from] OctocrabError),
    #[error(transparent)]
    JWT(#[from] JwtError),
    #[error(transparent)]
    UrlParse(#[from] ParseError),
    #[error("{0}")]
    Generic(String),
}
