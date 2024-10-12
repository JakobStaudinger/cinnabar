use serde::{Deserialize, Serialize};

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
    use super::*;

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
