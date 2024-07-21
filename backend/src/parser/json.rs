use super::{error::ParserError, PipelineParser};

pub struct JsonParser;

impl PipelineParser for JsonParser {
    async fn parse<I>(
        &self,
        file: &source_control::File,
        installation: &I,
    ) -> super::error::Result<domain::build::PipelineConfiguration>
    where
        I: source_control::SourceControlInstallation,
    {
        let content = installation
            .read_file_contents(&file.sha)
            .await
            .map_err(|err| ParserError::File(format!("Could not read file contents: {err:?}")))?;

        serde_json::from_str(&content)
            .map_err(|err| ParserError::Generic(format!("Could not parse json file: {err}")))
    }
}
