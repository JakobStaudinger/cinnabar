use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PipelineConfiguration {
    pub name: String,
    pub trigger: Vec<TriggerConfiguration>,
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
pub struct PipelineId(pub usize);

#[derive(Serialize, Deserialize)]
pub struct Step {
    pub id: StepId,
    pub configuration: StepConfiguration,
    pub status: PipelineStatus,
}

#[derive(Serialize, Deserialize)]
pub struct StepId(usize);

#[derive(Serialize, Deserialize, PartialEq)]
pub enum PipelineStatus {
    Pending,
    Running,
    Passed,
    Failed,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "event")]
pub enum TriggerConfiguration {
    #[serde(rename = "push")]
    Push { branch: Option<String> },
}

#[derive(Clone)]
pub struct Trigger {
    pub repository_owner: String,
    pub repository_name: String,
    pub installation_id: u64,
    pub event: TriggerEvent,
}

#[derive(Clone)]
pub enum TriggerEvent {
    Push { branch: String, commit: String },
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

impl TriggerConfiguration {
    pub fn matches(&self, trigger: &Trigger) -> bool {
        match self {
            Self::Push { branch } => match (branch, &trigger.event) {
                (Some(expected_branch), TriggerEvent::Push { branch, .. }) => {
                    expected_branch == branch
                }
                (None, TriggerEvent::Push { .. }) => true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::TriggerConfiguration;

    #[test]
    fn deserialize_push_trigger_configuration() {
        let json = r#"
            {
                "event": "push",
                "branch": "main"
            }
        "#;

        let trigger: TriggerConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(
            trigger,
            TriggerConfiguration::Push {
                branch: Some("main".to_owned())
            }
        )
    }

    #[test]
    fn deserialize_push_trigger_configuration_without_branch() {
        let json = r#"
        {
            "event": "push"
        }
        "#;

        let trigger: TriggerConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(trigger, TriggerConfiguration::Push { branch: None })
    }

    #[test]
    #[should_panic = "unknown variant `pull`"]
    fn deserialize_unknown_trigger_configuration() {
        let json = r#"
        {
            "event": "pull"
        }
        "#;

        serde_json::from_str::<TriggerConfiguration>(json).unwrap();
    }
}
