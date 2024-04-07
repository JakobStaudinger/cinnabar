use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PipelineConfiguration {
    pub name: String,
    pub steps: Vec<StepConfiguration>,
}

#[derive(Serialize, Deserialize)]
pub struct StepConfiguration {
    pub name: String,
    pub image: String,
    pub commands: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct Pipeline {
    pub id: PipelineId,
    pub configuration: PipelineConfiguration,
}

#[derive(Serialize, Deserialize)]
pub struct PipelineId(usize);

#[derive(Serialize, Deserialize)]
pub struct Step {
    pub id: StepId,
    pub configuration: StepConfiguration,
}

#[derive(Serialize, Deserialize)]
pub struct StepId(usize);

impl PipelineId {
    pub fn new(i: usize) -> Self {
        Self(i)
    }
}

impl StepId {
    pub fn new(i: usize) -> Self {
        Self(i)
    }
}
