use std::fmt::Display;

use serde::{de::Visitor, Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PipelineConfiguration {
    pub name: String,
    pub trigger: Vec<TriggerConfiguration>,
    pub steps: Vec<StepConfiguration>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StepConfiguration {
    pub name: String,
    pub image: DockerImageReference,
    pub commands: Option<Vec<String>>,
    pub cache: Option<Vec<String>>,
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(tag = "event")]
pub enum TriggerConfiguration {
    #[serde(rename = "push")]
    Push { branch: Option<String> },
    #[serde(rename = "pull_request")]
    PullRequest {
        target: Option<String>,
        source: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trigger {
    pub repository_owner: String,
    pub repository_name: String,
    pub installation_id: u64,
    pub event: TriggerEvent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TriggerEvent {
    Push { branch: Branch },
    PullRequest { source: Branch, target: Branch },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub commit: String,
}

#[derive(Clone)]
pub struct DockerImageReference {
    pub hostname: Option<String>,
    pub repository: String,
    pub tag: Option<String>,
}

impl Serialize for DockerImageReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for DockerImageReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(DockerImageReferenceVisitor)
    }
}

struct DockerImageReferenceVisitor;

impl<'de> Visitor<'de> for DockerImageReferenceVisitor {
    type Value = DockerImageReference;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A string of format [<hostname>/]<repository>[/<image>]*[:<tag>]")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.parse_string(v)
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.parse_string(v)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.parse_string(&v)
    }
}

impl DockerImageReferenceVisitor {
    fn parse_string<E>(self, v: &str) -> Result<DockerImageReference, E>
    where
        E: serde::de::Error,
    {
        let parts = v.split_once("/");
        match parts {
            Some((hostname, repository))
                if hostname.contains(['.', ':']) || hostname == "localhost" =>
            {
                let (repository, tag) = repository
                    .split_once(":")
                    .map(|(repository, tag)| (repository.to_string(), Some(tag.to_string())))
                    .unwrap_or_else(|| (v.to_string(), None));

                Ok(DockerImageReference {
                    hostname: Some(hostname.to_string()),
                    repository,
                    tag,
                })
            }
            None | Some(_) => {
                let (repository, tag) = v
                    .split_once(":")
                    .map(|(repository, tag)| (repository.to_string(), Some(tag.to_string())))
                    .unwrap_or_else(|| (v.to_string(), None));

                Ok(DockerImageReference {
                    hostname: None,
                    repository,
                    tag,
                })
            }
        }
    }
}

impl Display for DockerImageReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.hostname, &self.repository, &self.tag) {
            (Some(hostname), repository, Some(tag)) => {
                write!(f, "{hostname}/{repository}:{tag}")
            }
            (Some(hostname), repository, None) => {
                write!(f, "{hostname}/{repository}")
            }
            (None, repository, Some(tag)) => {
                write!(f, "{repository}:{tag}")
            }
            (None, repository, None) => {
                write!(f, "{repository}")
            }
        }
    }
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
            Self::Push {
                branch: expected_branch,
            } => match &trigger.event {
                TriggerEvent::Push {
                    branch: Branch { name: branch, .. },
                    ..
                } => match expected_branch {
                    None => true,
                    Some(expected_branch) => expected_branch == branch,
                },
                _ => false,
            },
            Self::PullRequest {
                target: expected_target,
                source: expected_source,
            } => match &trigger.event {
                TriggerEvent::PullRequest {
                    source: Branch { name: source, .. },
                    target: Branch { name: target, .. },
                } => {
                    expected_target
                        .as_ref()
                        .map_or(true, |expected_target| expected_target == target)
                        && expected_source
                            .as_ref()
                            .map_or(true, |expected_source| expected_source == source)
                }
                _ => false,
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
