use secrecy::SecretString;

pub trait SourceControl {
    type Installation: SourceControlInstallation;
    type Error: std::error::Error;

    fn get_installation(
        &self,
        owner: &str,
        repo: &str,
        installation_id: u64,
    ) -> impl std::future::Future<Output = Result<Self::Installation, Self::Error>> + Send;
}

pub trait SourceControlInstallation {
    type Error: std::error::Error;

    fn get_access_token(&self) -> &SecretString;
    fn read_file_contents(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<String, Self::Error>> + Send;
    fn update_status_check(
        &self,
        commit: &str,
        status: CheckStatus,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}

pub enum CheckStatus {
    Pending,
    Running,
    Failed,
    Passed,
}

impl CheckStatus {
    pub fn is_completed(&self) -> bool {
        match &self {
            CheckStatus::Pending | CheckStatus::Running => false,
            CheckStatus::Failed | CheckStatus::Passed => true,
        }
    }
}

pub mod github;
