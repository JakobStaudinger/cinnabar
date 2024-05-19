use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use domain::{Trigger, TriggerEvent};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sha2::Sha256;

use crate::RequestState;

pub async fn handle_webhook(
    State(RequestState { config, callbacks }): State<RequestState>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    if let Err(message) = verify_checksum(&headers, &body, &config.github_webhook_secret) {
        return (StatusCode::BAD_REQUEST, message);
    }

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

fn verify_checksum(
    headers: &HeaderMap,
    body: &String,
    secret: &SecretString,
) -> Result<(), &'static str> {
    let expected_signature = headers
        .get("x-hub-signature-256")
        .ok_or("Missing header x-hub-signature-256")?
        .to_str()
        .map_err(|_| "Failed to parse x-hub-signature-256 header")?;

    let expected_signature = expected_signature
        .strip_prefix("sha256=")
        .ok_or("Malformed sha256 header")?;

    let expected_signature =
        hex::decode(expected_signature).map_err(|_| "Failed to parse sha256 signature")?;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.expose_secret().as_bytes())
        .map_err(|_| "Failed to hash payload")?;

    mac.update(body.as_bytes());

    mac.verify_slice(expected_signature.as_slice())
        .map_err(|_| "Failed to verify sha256 checksum")
}

fn parse_trigger(headers: HeaderMap, body: String) -> Result<Option<Trigger>, &'static str> {
    let event = headers.get("x-github-event");
    let event = event.ok_or("Missing header x-github-event")?;

    match event.to_str() {
        Ok("push") => {
            #[derive(Deserialize)]
            struct PushEvent {
                r#ref: String,
                head_commit: Option<HeadCommit>,
                repository: Repository,
                installation: Installation,
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
                name: String,
            }

            #[derive(Deserialize)]
            struct Installation {
                id: u64,
            }

            let body = serde_json::from_str::<PushEvent>(&body);
            let body = body.map_err(|_| "Failed to parse payload")?;

            let repository_owner = body.repository.owner.name;
            let repository_name = body.repository.name;
            let installation_id = body.installation.id;
            body.r#ref
                .strip_prefix("refs/heads/")
                .zip(body.head_commit)
                .map_or(Ok(None), move |(branch, commit)| {
                    let branch = branch.to_string();
                    let commit = commit.id;
                    let event = TriggerEvent::Push { branch, commit };

                    Ok(Some(Trigger {
                        repository_owner,
                        repository_name,
                        installation_id,
                        event,
                    }))
                })
        }
        Ok(_) => Ok(None),
        Err(_) => Err("Failed to parse event"),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};
    use secrecy::SecretString;

    use super::verify_checksum;

    #[test]
    fn verify_checksum_should_return_ok() {
        let secret = SecretString::new("It's a Secret to Everybody".to_string());
        let body = "Hello, World!".to_string();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_static(
                "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17",
            ),
        );

        assert_eq!(verify_checksum(&headers, &body, &secret), Ok(()));
    }

    #[test]
    fn verify_checksum_should_return_err_if_header_is_missing() {
        let secret = SecretString::new("It's a Secret to Everybody".to_string());
        let body = "Hello, World!".to_string();
        let headers = HeaderMap::new();

        assert_eq!(
            verify_checksum(&headers, &body, &secret),
            Err("Missing header x-hub-signature-256")
        );
    }

    #[test]
    fn verify_checksum_should_return_err_if_checksum_differs() {
        let secret = SecretString::new("It's a Secret to Everybody".to_string());
        let body = "Hello, World!".to_string();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_static(
                "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e16",
            ),
        );
        assert_eq!(
            verify_checksum(&headers, &body, &secret),
            Err("Failed to verify sha256 checksum")
        );
    }
}
