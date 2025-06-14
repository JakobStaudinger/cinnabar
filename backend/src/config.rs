use secrecy::SecretString;

#[derive(Clone)]
pub struct AppConfig {
    pub github: GitHubConfig,
    pub database: DatabaseConfig,
}

#[derive(Clone)]
pub struct GitHubConfig {
    pub app_id: u64,
    pub private_key: SecretString,
    pub webhook_secret: SecretString,
}

#[derive(Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

impl AppConfig {
    pub fn from_environment() -> Result<AppConfig, String> {
        Ok(AppConfig {
            github: GitHubConfig::from_environment()?,
            database: DatabaseConfig::from_environment()?,
        })
    }
}

impl GitHubConfig {
    fn from_environment() -> Result<GitHubConfig, String> {
        let webhook_secret = SecretString::new(
            std::env::var("GITHUB_WEBHOOK_SECRET")
                .map_err(|_| "Please provide the GITHUB_WEBHOOK_SECRET environment variable")?,
        );
        let app_id = std::env::var("GITHUB_APP_ID")
            .map_err(|_| "Please provide the GITHUB_APP_ID environment variable")?
            .parse()
            .map_err(|_| "GITHUB_APP_ID needs to be an integer")?;
        let private_key = SecretString::new(
            std::env::var("GITHUB_PRIVATE_KEY")
                .map_err(|_| "Please provide the GITHUB_PRIVATE_KEY environment variable")?,
        );

        Ok(GitHubConfig {
            app_id,
            private_key,
            webhook_secret,
        })
    }
}

impl DatabaseConfig {
    fn from_environment() -> Result<DatabaseConfig, String> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| "Please provide the DATABASE_URL environment variable")?;

        Ok(DatabaseConfig { url })
    }
}
