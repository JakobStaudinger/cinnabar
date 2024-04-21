pub mod error;

use crate::source_control::{SourceControl, SourceControlInstallation};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use jsonwebtoken::EncodingKey;
use octocrab::{
    models::{AppId, InstallationToken},
    params::apps::CreateInstallationAccessToken,
    Octocrab,
};
use url::Url;

use self::error::GitHubError;

pub struct GitHub {
    octocrab: Octocrab,
}

impl GitHub {
    pub fn build(app_id: u64, private_key: &str) -> Result<Self, GitHubError> {
        let octocrab = Octocrab::builder()
            .app(
                AppId(app_id),
                EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap(),
            )
            .build()?;

        Ok(Self { octocrab })
    }
}

impl SourceControl for GitHub {
    async fn get_installation(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<GitHubInstallation, GitHubError> {
        let installation = self
            .octocrab
            .apps()
            .get_repository_installation(owner, repo)
            .await?;

        let mut create_access_token = CreateInstallationAccessToken::default();
        create_access_token.repositories = vec![repo.to_owned()];

        let access_token_url =
            Url::parse(installation.access_tokens_url.as_ref().unwrap()).unwrap();
        let access: InstallationToken = self
            .octocrab
            .post(access_token_url.path(), Some(&create_access_token))
            .await?;

        let octocrab = Octocrab::builder().personal_token(access.token).build()?;

        let owner = owner.to_owned();
        let repo = repo.to_owned();

        Ok(GitHubInstallation {
            octocrab,
            owner,
            repo,
        })
    }
}

pub struct GitHubInstallation {
    octocrab: Octocrab,
    owner: String,
    repo: String,
}

impl SourceControlInstallation for GitHubInstallation {
    async fn read_file_contents(&self, path: &str) -> Result<String, GitHubError> {
        let content = self
            .octocrab
            .repos(&self.owner, &self.repo)
            .get_content()
            .path(path)
            .send()
            .await?;

        let content = String::from_utf8_lossy(
            STANDARD
                .decode(
                    content.items[0]
                        .content
                        .as_ref()
                        .ok_or(GitHubError::Generic(format!(
                            "could not get content of {path}"
                        )))?
                        .split('\n')
                        .collect::<String>(),
                )
                .map_err(|_| GitHubError::Generic(format!("could not decode contents of {path}")))?
                .as_ref(),
        )
        .to_string();

        Ok(content)
    }

    async fn update_status_check(
        &self,
        commit: &str,
        status: crate::source_control::CheckStatus,
    ) -> Result<(), GitHubError> {
        self.octocrab
            .checks(&self.owner, &self.repo)
            .create_check_run("rust ci", commit)
            .external_id("1")
            .status(match status {
                crate::source_control::CheckStatus::Pending => {
                    octocrab::params::checks::CheckRunStatus::InProgress
                }
                _ => octocrab::params::checks::CheckRunStatus::Completed,
            })
            .conclusion(match status {
                crate::source_control::CheckStatus::Failed => {
                    octocrab::params::checks::CheckRunConclusion::Failure
                }
                crate::source_control::CheckStatus::Passed => {
                    octocrab::params::checks::CheckRunConclusion::Success
                }
                crate::source_control::CheckStatus::Pending => {
                    octocrab::params::checks::CheckRunConclusion::Neutral
                }
            })
            .send()
            .await?;

        Ok(())
    }
}
