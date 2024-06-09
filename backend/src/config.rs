use secrecy::SecretString;

#[derive(Clone)]
pub struct AppConfig {
    pub github_webhook_secret: SecretString,
    pub github_app_id: u64,
    pub github_private_key: SecretString,
}

pub fn build_config() -> Result<AppConfig, String> {
    let github_webhook_secret = SecretString::new(
        std::env::var("GITHUB_WEBHOOK_SECRET")
            .map_err(|_| "Please provide the GITHUB_WEBHOOK_SECRET environment variable")?,
    );
    let github_app_id = std::env::var("GITHUB_APP_ID")
        .map_err(|_| "Please provide the GITHUB_APP_ID environment variable")?
        .parse()
        .map_err(|_| "GITHUB_APP_ID needs to be an integer")?;
    let github_private_key = SecretString::new(
        std::env::var("GITHUB_PRIVATE_KEY")
            .map_err(|_| "Please provide the GITHUB_PRIVATE_KEY environment variable")?,
    );

    Ok(AppConfig {
        github_webhook_secret,
        github_app_id,
        github_private_key,
    })
}
