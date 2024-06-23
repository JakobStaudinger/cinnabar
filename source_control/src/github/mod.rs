pub mod error;

use std::path::Path;

use crate::{CheckStatus, File, Folder, SourceControl, SourceControlInstallation};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use jsonwebtoken::EncodingKey;
use octocrab::{
    models::{AppId, InstallationId},
    params::checks::{CheckRunConclusion, CheckRunStatus},
    Octocrab,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

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

    async fn read_file_contents(&self, sha: &str) -> Result<String, Self::Error> {
        #[derive(Deserialize, Debug)]
        struct GitBlob {
            content: String,
        }

        let GitBlob { content } = self
            .octocrab
            .get(
                format!("/repos/{}/{}/git/blobs/{}", self.owner, self.repo, sha),
                None::<&()>,
            )
            .await?;

        let content = String::from_utf8_lossy(
            STANDARD
                .decode(content.split('\n').collect::<String>())
                .map_err(|_| GitHubError::Generic(format!("could not decode contents of {sha}")))?
                .as_ref(),
        )
        .to_string();

        Ok(content)
    }

    async fn read_folder(&self, path: &str, r#ref: &str) -> Result<Folder, Self::Error> {
        let path = Path::new(path);
        let r#ref = match path.parent() {
            None => r#ref.to_owned(),
            Some(parent) => {
                let parent = parent.to_str().ok_or(GitHubError::Generic(
                    "Could not convert path to str".to_owned(),
                ))?;

                let content = self
                    .octocrab
                    .repos(&self.owner, &self.repo)
                    .get_content()
                    .path(parent)
                    .r#ref(r#ref)
                    .send()
                    .await?;

                content
                    .items
                    .into_iter()
                    .find(|item| Path::new(item.path.as_str()) == path)
                    .ok_or(GitHubError::Generic(
                        "Could not find file in tree".to_owned(),
                    ))?
                    .sha
            }
        };

        let GitTree { tree, .. } = self.get_tree(&r#ref).await?;

        let items = tree
            .into_iter()
            .filter_map(|sub_tree| match &sub_tree.r#type[..] {
                "blob" => Some(File {
                    sha: sub_tree.sha,
                    path: path.join(sub_tree.path),
                }),
                _ => None,
            })
            .collect();

        Ok(Folder { items })
    }

    async fn print_rate_limit(&self) -> Result<(), Self::Error> {
        let limit = self.octocrab.ratelimit().get().await?;
        println!("{:?}", limit.resources.core);
        Ok(())
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

#[derive(Deserialize, Debug)]
struct GitTree {
    tree: Vec<GitSubTree>,
}

#[derive(Deserialize, Debug)]
struct GitSubTree {
    path: String,
    r#type: String,
    sha: String,
}

impl GitHubInstallation {
    async fn get_tree(&self, r#ref: &str) -> Result<GitTree, GitHubError> {
        #[derive(Serialize)]
        struct Params {
            recursive: bool,
        }

        let response = self
            .octocrab
            .get(
                format!("/repos/{}/{}/git/trees/{}", self.owner, self.repo, r#ref),
                Some(&Params { recursive: true }),
            )
            .await?;

        Ok(response)
    }
}
