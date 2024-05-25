use std::{future::Future, path::PathBuf};

use secrecy::SecretString;

pub trait SourceControl {
    type Installation: SourceControlInstallation;
    type Error: std::error::Error;

    fn get_installation(
        &self,
        owner: &str,
        repo: &str,
        installation_id: u64,
    ) -> impl Future<Output = Result<Self::Installation, Self::Error>> + Send;
}

pub trait SourceControlInstallation {
    type Error: std::error::Error;

    fn get_access_token(&self) -> &SecretString;
    fn read_file_contents(
        &self,
        sha: &str,
    ) -> impl Future<Output = Result<String, Self::Error>> + Send;
    fn read_folder(
        &self,
        path: &str,
        r#ref: &str,
    ) -> impl Future<Output = Result<Folder, Self::Error>> + Send;
    fn print_rate_limit(&self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn update_status_check(
        &self,
        commit: &str,
        name: &str,
        id: usize,
        status: CheckStatus,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

#[derive(Debug)]
pub struct Folder {
    pub items: Vec<File>,
}

#[derive(Debug)]
pub struct File {
    pub sha: String,
    pub path: PathBuf,
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
