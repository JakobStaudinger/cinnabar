use super::{error::ParserError, PipelineParser};

pub struct JsonnetParser;

impl PipelineParser for JsonnetParser {
    async fn parse<I>(
        &self,
        file: &source_control::File,
        installation: &I,
    ) -> super::error::Result<domain::PipelineConfiguration>
    where
        I: source_control::SourceControlInstallation,
    {
        let content = installation
            .read_file_contents(&file.sha)
            .await
            .map_err(|err| ParserError::File(format!("Could not read file contents: {err}")))?;

        let mut program = rsjsonnet_lang::program::Program::new();
        let (span_context, _) = program
            .span_manager_mut()
            .insert_source_context(content.len());

        let thunk = program
            .load_source(
                span_context,
                content.as_bytes(),
                true,
                &file.path.to_string_lossy(),
            )
            .map_err(|err| ParserError::Generic(format!("Could not interpret jsonnet: {err:?}")))?;

        let value = program
            .eval_value(&thunk, &mut Callbacks)
            .map_err(|err| ParserError::Generic(format!("Could not interpret jsonnet: {err:?}")))?;
        let value = program
            .manifest_json(&value, false)
            .map_err(|err| ParserError::Generic(format!("Could not manifest json: {err:?}")))?;

        Ok(serde_json::from_str(&value)
            .map_err(|err| ParserError::Generic(format!("Could not parse json: {err}")))?)
    }
}

struct Callbacks;

impl rsjsonnet_lang::program::Callbacks for Callbacks {
    fn import(
        &mut self,
        program: &mut rsjsonnet_lang::program::Program,
        from: rsjsonnet_lang::span::SpanId,
        path: &str,
    ) -> Result<rsjsonnet_lang::program::Thunk, rsjsonnet_lang::program::ImportError> {
        unimplemented!();
    }

    fn import_str(
        &mut self,
        program: &mut rsjsonnet_lang::program::Program,
        from: rsjsonnet_lang::span::SpanId,
        path: &str,
    ) -> Result<String, rsjsonnet_lang::program::ImportError> {
        unimplemented!();
    }

    fn import_bin(
        &mut self,
        program: &mut rsjsonnet_lang::program::Program,
        from: rsjsonnet_lang::span::SpanId,
        path: &str,
    ) -> Result<Vec<u8>, rsjsonnet_lang::program::ImportError> {
        unimplemented!();
    }

    fn trace(
        &mut self,
        program: &mut rsjsonnet_lang::program::Program,
        message: &str,
        stack: &[rsjsonnet_lang::program::EvalStackTraceItem],
    ) {
        unimplemented!();
    }

    fn native_call(
        &mut self,
        program: &mut rsjsonnet_lang::program::Program,
        name: &rsjsonnet_lang::interner::InternedStr,
        args: &[rsjsonnet_lang::program::Value],
    ) -> Result<rsjsonnet_lang::program::Value, rsjsonnet_lang::program::NativeError> {
        unimplemented!();
    }
}
