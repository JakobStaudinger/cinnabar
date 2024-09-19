use crate::PipelineId;

pub mod implementation;

pub trait PipelinesRepository {
    fn create_new(&mut self) -> Result<PipelineId, ()>;
}
