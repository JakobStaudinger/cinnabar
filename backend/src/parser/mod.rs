pub mod error;
mod json;
mod jsonnet;

use self::{
    error::{ParserError, Result},
    json::JsonParser,
    jsonnet::JsonnetParser,
};
use domain::PipelineConfiguration;
use source_control::{File, SourceControlInstallation};

pub async fn parse_pipeline<I>(file: &File, installation: &I) -> Result<PipelineConfiguration>
where
    I: SourceControlInstallation,
{
    let file_extension = file.path.extension().unwrap_or_default();
    match file_extension.to_str() {
        Some("jsonnet") | Some("libsonnet") => {
            let parser = JsonnetParser;
            Ok(parser.parse(file, installation).await?)
        }
        Some("json") => {
            let parser = JsonParser;
            Ok(parser.parse(file, installation).await?)
        }
        extension => Err(ParserError::File(format!(
            "Unknown extension \"{}\"",
            extension.unwrap_or_default()
        ))),
    }
}

trait PipelineParser {
    async fn parse<I>(&self, file: &File, installation: &I) -> Result<PipelineConfiguration>
    where
        I: SourceControlInstallation;
}
