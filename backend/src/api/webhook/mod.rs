mod checksum;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{de::Visitor, Deserialize};
use std::sync::Arc;

use crate::{api::RequestState, config::AppConfig};

use checksum::VerifiedBody;
use domain::{Branch, Trigger, TriggerEvent};

#[derive(Clone)]
pub struct Callbacks {
    pub trigger: Arc<dyn Send + Sync + Fn(Trigger, AppConfig)>,
}

pub async fn handle_webhook(
    State(RequestState { config, callbacks }): State<RequestState>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    let body = match checksum::verify(&headers, body, &config.github_webhook_secret) {
        Ok(body) => body,
        Err(message) => return (StatusCode::BAD_REQUEST, message),
    };

    let trigger = parse_trigger(headers, body);

    match trigger {
        Ok(Some(trigger)) => {
            (*callbacks.trigger)(trigger, config);
            (StatusCode::CREATED, "OK")
        }
        Ok(None) => (StatusCode::NO_CONTENT, "OK"),
        Err(message) => (StatusCode::BAD_REQUEST, message),
    }
}

fn parse_trigger(headers: HeaderMap, body: VerifiedBody) -> Result<Option<Trigger>, &'static str> {
    let event = headers.get("x-github-event");
    let event = event.ok_or("Missing header x-github-event")?;
    let event = event.to_str().map_err(|_| "Failed to parse event")?;

    let supported_events = ["push", "pull_request"];

    if supported_events.contains(&event) {
        let payload = format!(
            r#"{{
                "event": "{event}",
                "payload": {body}
            }}"#,
        );

        let event = serde_json::from_str::<WebhookEvent>(&payload)
            .map_err(|_| "Failed to parse payload")?;

        Ok(event.extract_trigger())
    } else {
        Ok(None)
    }
}

#[derive(Deserialize)]
#[serde(tag = "event", content = "payload")]
enum WebhookEvent {
    #[serde(rename = "push")]
    Push(PushEventData),
    #[serde(rename = "pull_request")]
    PullRequest(PullRequestEvent),
}

impl WebhookEvent {
    fn extract_trigger(self) -> Option<Trigger> {
        match self {
            WebhookEvent::Push(data) => data.extract_trigger(),
            WebhookEvent::PullRequest(data) => data.extract_trigger(),
        }
    }
}

#[derive(Deserialize)]
struct PushEventData {
    r#ref: String,
    head_commit: Option<HeadCommit>,
    repository: Repository,
    installation: Installation,
}

impl PushEventData {
    fn extract_trigger(self) -> Option<Trigger> {
        let repository_owner = self.repository.owner.login;
        let repository_name = self.repository.name;
        let installation_id = self.installation.id;
        self.r#ref
            .strip_prefix("refs/heads/")
            .zip(self.head_commit)
            .map(move |(branch, commit)| {
                let branch = branch.to_owned();
                let commit = commit.id;
                let event = TriggerEvent::Push {
                    branch: Branch {
                        name: branch,
                        commit,
                    },
                };

                Trigger {
                    repository_owner,
                    repository_name,
                    installation_id,
                    event,
                }
            })
    }
}

#[derive(Deserialize)]
#[serde(tag = "action")]
enum PullRequestEvent {
    #[serde(rename = "opened")]
    Opened(PullRequestEventData),
    #[serde(rename = "reopened")]
    Reopened(PullRequestEventData),
    #[serde(rename = "synchronize")]
    Synchronize(PullRequestEventData),
    #[serde(other)]
    Other,
}

impl PullRequestEvent {
    fn extract_trigger(self) -> Option<Trigger> {
        let data = match self {
            PullRequestEvent::Opened(data)
            | PullRequestEvent::Reopened(data)
            | PullRequestEvent::Synchronize(data) => Some(data),
            PullRequestEvent::Other => None,
        }?;

        data.extract_trigger()
    }
}

#[derive(Deserialize)]
struct PullRequestEventData {
    installation: Installation,
    repository: Repository,
    pull_request: PullRequest,
}

impl PullRequestEventData {
    fn extract_trigger(self) -> Option<Trigger> {
        let event = TriggerEvent::PullRequest {
            source: Branch {
                name: self.pull_request.head.r#ref.get_name(),
                commit: self.pull_request.head.sha,
            },
            target: Branch {
                name: self.pull_request.base.r#ref.get_name(),
                commit: self.pull_request.base.sha,
            },
        };

        Some(Trigger {
            event,
            installation_id: self.installation.id,
            repository_name: self.repository.name,
            repository_owner: self.repository.owner.login,
        })
    }
}

#[derive(Deserialize)]
struct HeadCommit {
    id: String,
}

#[derive(Deserialize)]
struct Repository {
    name: String,
    owner: RepositoryOwner,
}

#[derive(Deserialize)]
struct RepositoryOwner {
    login: String,
}

#[derive(Deserialize)]
struct Installation {
    id: u64,
}

#[derive(Deserialize)]
struct PullRequest {
    head: PullRequestRef,
    base: PullRequestRef,
}

#[derive(Deserialize)]
struct PullRequestRef {
    r#ref: Ref,
    sha: String,
}

enum Ref {
    Head(String),
    Tag(String),
}

impl Ref {
    fn get_name(self) -> String {
        match self {
            Ref::Head(name) | Ref::Tag(name) => name,
        }
    }
}

impl<'de> Deserialize<'de> for Ref {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(RefVisitor)
    }
}

struct RefVisitor;

impl<'de> Visitor<'de> for RefVisitor {
    type Value = Ref;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A string of format refs/heads/<branch-name> or refs/tags/<tag-name>")
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

impl RefVisitor {
    fn parse_string<E>(self, v: &str) -> Result<Ref, E>
    where
        E: serde::de::Error,
    {
        if let Some(head) = v.strip_prefix("refs/heads/") {
            Ok(Ref::Head(head.to_owned()))
        } else if let Some(tag) = v.strip_prefix("refs/tags/") {
            Ok(Ref::Tag(tag.to_owned()))
        } else {
            Ok(Ref::Head(v.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;
    use domain::Branch;

    use super::*;

    #[test]
    fn parse_trigger_should_return_none_for_unknown_event() {
        let mut headers = HeaderMap::new();
        headers.insert("X-GitHub-Event", HeaderValue::from_static("pull"));

        let result = parse_trigger(headers, VerifiedBody::from_static(""));

        assert_eq!(result, Ok(None));
    }

    #[test]
    fn parse_trigger_should_return_error_for_missing_event_header() {
        let headers = HeaderMap::new();

        let result = parse_trigger(headers, VerifiedBody::from_static(""));

        assert_eq!(result, Err("Missing header x-github-event"));
    }

    #[test]
    fn parse_trigger_should_parse_push_event() {
        let mut headers = HeaderMap::new();
        headers.insert("X-GitHub-Event", HeaderValue::from_static("push"));

        let body = VerifiedBody::from_static(
            r#"{
                    "ref": "refs/heads/branch",
                    "head_commit": {
                        "id": "123"
                    },
                    "repository": {
                        "name": "Repo",
                        "owner": {
                            "login": "Owner"
                        }
                    },
                    "installation": {
                        "id": 789
                    }
                }"#,
        );

        let result = parse_trigger(headers, body);

        assert_eq!(
            result,
            Ok(Some(Trigger {
                event: TriggerEvent::Push {
                    branch: Branch {
                        name: "branch".to_owned(),
                        commit: "123".to_owned()
                    }
                },
                installation_id: 789,
                repository_name: "Repo".to_owned(),
                repository_owner: "Owner".to_owned()
            }))
        );
    }

    #[test]
    fn parse_trigger_should_parse_pull_request_opened_event() {
        let mut headers = HeaderMap::new();
        headers.insert("X-GitHub-Event", HeaderValue::from_static("pull_request"));

        let body = VerifiedBody::from_static(
            r#"{
                    "action": "opened",
                    "pull_request": {
                        "head": {
                            "sha": "123",
                            "ref": "refs/heads/head-branch"
                        },
                        "base": {
                            "sha": "456",
                            "ref": "refs/heads/base-branch"
                        }
                    },
                    "repository": {
                        "name": "Repo",
                        "owner": {
                            "login": "Owner"
                        }
                    },
                    "installation": {
                        "id": 789
                    }
                }"#,
        );

        let result = parse_trigger(headers, body);

        assert_eq!(
            result,
            Ok(Some(Trigger {
                event: TriggerEvent::PullRequest {
                    source: Branch {
                        name: "head-branch".to_owned(),
                        commit: "123".to_owned()
                    },
                    target: Branch {
                        name: "base-branch".to_owned(),
                        commit: "456".to_owned()
                    }
                },
                installation_id: 789,
                repository_name: "Repo".to_owned(),
                repository_owner: "Owner".to_owned()
            }))
        );
    }

    #[test]
    fn parse_trigger_should_parse_pull_request_reopened_event() {
        let mut headers = HeaderMap::new();
        headers.insert("X-GitHub-Event", HeaderValue::from_static("pull_request"));

        let body = VerifiedBody::from_static(
            r#"{
                    "action": "reopened",
                    "pull_request": {
                        "head": {
                            "sha": "123",
                            "ref": "refs/heads/head-branch"
                        },
                        "base": {
                            "sha": "456",
                            "ref": "refs/heads/base-branch"
                        }
                    },
                    "repository": {
                        "name": "Repo",
                        "owner": {
                            "login": "Owner"
                        }
                    },
                    "installation": {
                        "id": 789
                    }
                }"#,
        );

        let result = parse_trigger(headers, body);

        assert_eq!(
            result,
            Ok(Some(Trigger {
                event: TriggerEvent::PullRequest {
                    source: Branch {
                        name: "head-branch".to_owned(),
                        commit: "123".to_owned()
                    },
                    target: Branch {
                        name: "base-branch".to_owned(),
                        commit: "456".to_owned()
                    }
                },
                installation_id: 789,
                repository_name: "Repo".to_owned(),
                repository_owner: "Owner".to_owned()
            }))
        );
    }

    #[test]
    fn parse_trigger_should_parse_pull_request_synchronize_event() {
        let mut headers = HeaderMap::new();
        headers.insert("X-GitHub-Event", HeaderValue::from_static("pull_request"));

        let body = VerifiedBody::from_static(
            r#"{
                    "action": "synchronize",
                    "pull_request": {
                        "head": {
                            "sha": "123",
                            "ref": "refs/heads/head-branch"
                        },
                        "base": {
                            "sha": "456",
                            "ref": "refs/heads/base-branch"
                        }
                    },
                    "repository": {
                        "name": "Repo",
                        "owner": {
                            "login": "Owner"
                        }
                    },
                    "installation": {
                        "id": 789
                    }
                }"#,
        );

        let result = parse_trigger(headers, body);

        assert_eq!(
            result,
            Ok(Some(Trigger {
                event: TriggerEvent::PullRequest {
                    source: Branch {
                        name: "head-branch".to_owned(),
                        commit: "123".to_owned()
                    },
                    target: Branch {
                        name: "base-branch".to_owned(),
                        commit: "456".to_owned()
                    }
                },
                installation_id: 789,
                repository_name: "Repo".to_owned(),
                repository_owner: "Owner".to_owned()
            }))
        );
    }
}
