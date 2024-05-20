pub mod error;

use crate::{CheckStatus, SourceControl, SourceControlInstallation};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use jsonwebtoken::EncodingKey;
use octocrab::{
    models::{AppId, InstallationId},
    params::checks::{CheckRunConclusion, CheckRunStatus},
    Octocrab,
};
use secrecy::{ExposeSecret, SecretString};

use self::error::GitHubError;

pub struct GitHub {
    octocrab: Octocrab,
}

impl GitHub {
    pub fn build(app_id: u64, private_key: &SecretString) -> Result<Self, GitHubError> {
        let octocrab = Octocrab::builder()
            .app(
                AppId(app_id),
                EncodingKey::from_rsa_pem(private_key.expose_secret().as_bytes())?,
            )
            .build()?;

        Ok(Self { octocrab })
    }
}

impl SourceControl for GitHub {
    type Installation = GitHubInstallation;
    type Error = GitHubError;

    async fn get_installation(
        &self,
        owner: &str,
        repo: &str,
        installation_id: u64,
    ) -> Result<Self::Installation, Self::Error> {
        let (octocrab, token) = self
            .octocrab
            .installation_and_token(InstallationId(installation_id))
            .await?;

        let owner = owner.to_owned();
        let repo = repo.to_owned();

        Ok(GitHubInstallation {
            octocrab,
            owner,
            repo,
            token,
        })
    }
}

#[derive(Clone)]
pub struct GitHubInstallation {
    octocrab: Octocrab,
    owner: String,
    repo: String,
    token: SecretString,
}

impl SourceControlInstallation for GitHubInstallation {
    type Error = GitHubError;

    fn get_access_token(&self) -> &SecretString {
        &self.token
    }

    async fn read_file_contents(&self, path: &str, r#ref: &str) -> Result<String, Self::Error> {
        let content = self
            .octocrab
            .repos(&self.owner, &self.repo)
            .get_content()
            .path(path)
            .r#ref(r#ref)
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
        name: &str,
        id: usize,
        status: CheckStatus,
    ) -> Result<(), Self::Error> {
        let checks = self.octocrab.checks(&self.owner, &self.repo);
        let mut check_run = checks.create_check_run(name, commit);

        check_run = check_run.external_id(id.to_string()).status(match status {
            CheckStatus::Pending => CheckRunStatus::Queued,
            CheckStatus::Running => CheckRunStatus::InProgress,
            _ => CheckRunStatus::Completed,
        });

        if status.is_completed() {
            check_run = check_run.conclusion(match status {
                CheckStatus::Failed => CheckRunConclusion::Failure,
                CheckStatus::Passed => CheckRunConclusion::Success,
                CheckStatus::Pending | CheckStatus::Running => CheckRunConclusion::Neutral,
            });
        }

        check_run.send().await?;

        Ok(())
    }
}
