pub trait SourceControl {
    async fn get_installation(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<impl SourceControlInstallation, impl std::error::Error>;
}

pub trait SourceControlInstallation {
    async fn read_file_contents(&self, path: &str) -> Result<String, impl std::error::Error>;
}
