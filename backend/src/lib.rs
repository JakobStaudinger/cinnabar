use base64::{engine::general_purpose::STANDARD, Engine as _};

use bollard::Docker;
use domain::{Pipeline, PipelineId};
use jsonwebtoken::EncodingKey;
use octocrab::{
    models::{AppId, InstallationToken},
    params::apps::CreateInstallationAccessToken,
    Octocrab,
};
use url::Url;

mod domain;
mod runner;

pub async fn main() {
    let github_app_id = std::env::var("GITHUB_APP_ID").unwrap().parse().unwrap();
    let github_private_key = std::env::var("GITHUB_PRIVATE_KEY").unwrap();

    let octocrab = Octocrab::builder()
        .app(
            AppId(github_app_id),
            EncodingKey::from_rsa_pem(github_private_key.as_bytes()).unwrap(),
        )
        .build()
        .unwrap();

    let installation = octocrab
        .apps()
        .get_repository_installation("JakobStaudinger", "rust-ci")
        .await
        .unwrap();

    let mut create_access_token = CreateInstallationAccessToken::default();
    create_access_token.repositories = vec!["rust-ci".to_owned()];

    let access_token_url = Url::parse(installation.access_tokens_url.as_ref().unwrap()).unwrap();
    let access: InstallationToken = octocrab
        .post(access_token_url.path(), Some(&create_access_token))
        .await
        .unwrap();

    let octocrab = Octocrab::builder()
        .personal_token(access.token)
        .build()
        .unwrap();

    let content = octocrab
        .repos("JakobStaudinger", "rust-ci")
        .get_content()
        .path("assets/test-pipeline.json")
        .send()
        .await
        .unwrap();

    let configuration = String::from_utf8_lossy(
        STANDARD
            .decode(
                content.items[0]
                    .content
                    .as_ref()
                    .unwrap()
                    .split('\n')
                    .collect::<String>(),
            )
            .unwrap()
            .as_ref(),
    )
    .to_string();

    let configuration = serde_json::from_str(&configuration).unwrap();

    let pipeline = Pipeline::new(PipelineId::new(1), configuration);

    let docker = Docker::connect_with_socket_defaults().unwrap();
    let runner = runner::PipelineRunner::new(&docker);
    runner.run_pipeline(&pipeline).await.unwrap();
}
