use std::fmt::Display;

use serde::{de::Visitor, Deserialize, Serialize};

use super::trigger::TriggerConfiguration;

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

#[derive(Clone, Debug, PartialEq, Eq)]
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
        let parts = v.split_once('/');
        let (hostname, repository_and_tag) = parts
            .and_then(|(hostname, repository)| {
                if hostname.contains(['.', ':']) || hostname == "localhost" {
                    Some((Some(hostname.to_string()), repository))
                } else {
                    None
                }
            })
            .unwrap_or((None, v));

        let (repository, tag) = repository_and_tag
            .split_once(':')
            .map(|(repository, tag)| (repository.to_string(), Some(tag.to_string())))
            .unwrap_or_else(|| (repository_and_tag.to_string(), None));

        Ok(DockerImageReference {
            hostname,
            repository,
            tag,
        })
    }
}

impl Display for DockerImageReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hostname = self
            .hostname
            .as_ref()
            .map_or("".to_string(), |hostname| format!("{hostname}/"));
        let tag = self
            .tag
            .as_ref()
            .map_or("".to_string(), |tag| format!(":{tag}"));
        let repository = &self.repository;

        write!(f, "{hostname}{repository}{tag}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_image_reference_should_serialize_with_hostname_and_tag() {
        let value = DockerImageReference {
            hostname: Some("host.com".to_string()),
            repository: "repo/image".to_string(),
            tag: Some("1.0".to_string()),
        };

        assert_eq!(value.to_string(), "host.com/repo/image:1.0");
    }

    #[test]
    fn docker_image_reference_should_serialize_without_hostname_and_tag() {
        let value = DockerImageReference {
            hostname: None,
            repository: "repo/image".to_string(),
            tag: None,
        };

        assert_eq!(value.to_string(), "repo/image");
    }

    #[test]
    fn docker_image_reference_should_deserialize_with_hostname_and_tag() {
        let value: DockerImageReference =
            serde_json::from_str("\"host.com/repo/image:1.0\"").unwrap();

        assert_eq!(
            value,
            DockerImageReference {
                hostname: Some("host.com".to_string()),
                repository: "repo/image".to_string(),
                tag: Some("1.0".to_string())
            }
        );
    }

    #[test]
    fn docker_image_reference_should_deserialize_without_hostname_and_tag() {
        let value: DockerImageReference = serde_json::from_str("\"repo/image\"").unwrap();

        assert_eq!(
            value,
            DockerImageReference {
                hostname: None,
                repository: "repo/image".to_string(),
                tag: None
            }
        );
    }

    #[test]
    fn docker_image_reference_should_deserialize_with_hostname_and_no_tag() {
        let value: DockerImageReference = serde_json::from_str("\"host.com/repo/image\"").unwrap();

        assert_eq!(
            value,
            DockerImageReference {
                hostname: Some("host.com".to_string()),
                repository: "repo/image".to_string(),
                tag: None,
            }
        );
    }

    #[test]
    fn docker_image_reference_should_deserialize_without_hostname_and_with_tag() {
        let value: DockerImageReference = serde_json::from_str("\"repo/image:1.0\"").unwrap();

        assert_eq!(
            value,
            DockerImageReference {
                hostname: None,
                repository: "repo/image".to_string(),
                tag: Some("1.0".to_string())
            }
        );
    }
}
