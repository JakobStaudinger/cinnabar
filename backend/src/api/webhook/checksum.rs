use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha256;
use std::fmt::Display;

pub fn verify(
    headers: &HeaderMap,
    body: String,
    secret: &SecretString,
) -> Result<VerifiedBody, &'static str> {
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
        .map_err(|_| "Failed to verify sha256 checksum")?;

    Ok(VerifiedBody { body })
}

#[derive(PartialEq, Eq, Debug)]
pub struct VerifiedBody {
    body: String,
}

impl Display for VerifiedBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.body.fmt(f)
    }
}

#[cfg(test)]
impl VerifiedBody {
    pub fn from_static(str: &'static str) -> VerifiedBody {
        VerifiedBody {
            body: str.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::http::{HeaderMap, HeaderValue};
    use secrecy::SecretString;

    #[test]
    fn verify_should_return_ok() {
        let secret = SecretString::new("It's a Secret to Everybody".to_owned());
        let body = "Hello, World!".to_owned();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_static(
                "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17",
            ),
        );

        assert_eq!(
            verify(&headers, body.clone(), &secret),
            Ok(VerifiedBody { body })
        );
    }

    #[test]
    fn verify_should_return_err_if_header_is_missing() {
        let secret = SecretString::new("It's a Secret to Everybody".to_owned());
        let body = "Hello, World!".to_owned();
        let headers = HeaderMap::new();

        assert_eq!(
            verify(&headers, body, &secret),
            Err("Missing header x-hub-signature-256")
        );
    }

    #[test]
    fn verify_should_return_err_if_checksum_differs() {
        let secret = SecretString::new("It's a Secret to Everybody".to_owned());
        let body = "Hello, World!".to_owned();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_static(
                "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e16",
            ),
        );
        assert_eq!(
            verify(&headers, body, &secret),
            Err("Failed to verify sha256 checksum")
        );
    }

    #[test]
    fn verify_should_return_err_if_header_is_malformed() {
        let secret = SecretString::new("It's a Secret to Everybody".to_owned());
        let body = "Hello, World!".to_owned();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_static(
                "757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17",
            ),
        );
        assert_eq!(
            verify(&headers, body, &secret),
            Err("Malformed sha256 header")
        );
    }

    #[test]
    fn verify_should_return_err_if_sha_is_no_hex_string() {
        let secret = SecretString::new("It's a Secret to Everybody".to_owned());
        let body = "Hello, World!".to_owned();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_static("sha256=wxyz"),
        );
        assert_eq!(
            verify(&headers, body, &secret),
            Err("Failed to parse sha256 signature")
        );
    }

    #[test]
    fn verify_should_return_err_if_header_is_wrongly_encoded() {
        let secret = SecretString::new("It's a Secret to Everybody".to_owned());
        let body = "Hello, World!".to_owned();
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_str("héllò").unwrap(),
        );

        assert_eq!(
            verify(&headers, body, &secret),
            Err("Failed to parse x-hub-signature-256 header")
        );
    }
}
