use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PipelineConfiguration {
    pub name: String,
    pub steps: Vec<StepConfiguration>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StepConfiguration {
    pub name: String,
    pub image: String,
    pub commands: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct Pipeline {
    pub id: PipelineId,
    pub configuration: PipelineConfiguration,
    pub steps: Vec<Step>,
    pub status: PipelineStatus,
}

#[derive(Serialize, Deserialize)]
pub struct PipelineId(usize);

#[derive(Serialize, Deserialize)]
pub struct Step {
    pub id: StepId,
    pub configuration: StepConfiguration,
    pub status: PipelineStatus,
}

#[derive(Serialize, Deserialize)]
pub struct StepId(usize);

#[derive(Serialize, Deserialize)]
pub enum PipelineStatus {
    Pending,
    Running,
    Passed,
    Failed,
}

impl Pipeline {
    pub fn new(id: PipelineId, configuration: PipelineConfiguration) -> Self {
        let steps = configuration
            .steps
            .iter()
            .enumerate()
            .map(|(id, step_configuration)| {
                Step::new(StepId::new(id + 1), step_configuration.clone())
            })
            .collect();
        Self {
            id,
            configuration,
            steps,
            status: PipelineStatus::Pending,
        }
    }
}

impl PipelineId {
    pub fn new(i: usize) -> Self {
        Self(i)
    }
}

impl Display for PipelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Step {
    pub fn new(id: StepId, configuration: StepConfiguration) -> Self {
        Self {
            id,
            configuration,
            status: PipelineStatus::Pending,
        }
    }
}

impl StepId {
    pub fn new(i: usize) -> Self {
        Self(i)
    }
}

impl Display for StepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
