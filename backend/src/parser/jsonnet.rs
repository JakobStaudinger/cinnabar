use super::{error::ParserError, PipelineParser};

pub struct JsonnetParser;

impl PipelineParser for JsonnetParser {
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

        serde_json::from_str(&value)
            .map_err(|err| ParserError::Generic(format!("Could not parse json: {err}")))
    }
}

struct Callbacks;

impl rsjsonnet_lang::program::Callbacks for Callbacks {
    fn import(
        &mut self,
        _program: &mut rsjsonnet_lang::program::Program,
        _from: rsjsonnet_lang::span::SpanId,
        _path: &str,
    ) -> Result<rsjsonnet_lang::program::Thunk, rsjsonnet_lang::program::ImportError> {
        Err(rsjsonnet_lang::program::ImportError)
    }

    fn import_str(
        &mut self,
        _program: &mut rsjsonnet_lang::program::Program,
        _from: rsjsonnet_lang::span::SpanId,
        _path: &str,
    ) -> Result<String, rsjsonnet_lang::program::ImportError> {
        Err(rsjsonnet_lang::program::ImportError)
    }

    fn import_bin(
        &mut self,
        _program: &mut rsjsonnet_lang::program::Program,
        _from: rsjsonnet_lang::span::SpanId,
        _path: &str,
    ) -> Result<Vec<u8>, rsjsonnet_lang::program::ImportError> {
        Err(rsjsonnet_lang::program::ImportError)
    }

    fn trace(
        &mut self,
        _program: &mut rsjsonnet_lang::program::Program,
        _message: &str,
        _stack: &[rsjsonnet_lang::program::EvalStackTraceItem],
    ) {
    }

    fn native_call(
        &mut self,
        _program: &mut rsjsonnet_lang::program::Program,
        _name: &rsjsonnet_lang::interner::InternedStr,
        _args: &[rsjsonnet_lang::program::Value],
    ) -> Result<rsjsonnet_lang::program::Value, rsjsonnet_lang::program::NativeError> {
        Err(rsjsonnet_lang::program::NativeError)
    }
}
