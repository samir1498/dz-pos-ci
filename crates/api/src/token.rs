//! The launch token: what a caller shows to be believed on loopback.
//!
//! 127.0.0.1 is reachable by every process and every browser tab on the
//! machine, so the socket alone says nothing about who is asking. The
//! desktop makes a fresh random token each time it starts, hands it only to
//! its own webview, and the router refuses every other caller. The
//! standalone server takes the same token from its environment
//! (`DZPOS_API_TOKEN`), never from the command line, where `ps` would show
//! it to every user on the box. The design for the other links is in
//! docs/architecture.md, "Transport and auth".

use std::fmt;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::ApiError;

/// Bytes of randomness in a generated token; shown as 64 hex characters.
const RANDOM_BYTES: usize = 32;

/// A secret with no `Debug` output: it must not land in a log by accident.
#[derive(Clone)]
pub struct LaunchToken(Arc<str>);

impl fmt::Debug for LaunchToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LaunchToken(..)")
    }
}

impl LaunchToken {
    /// A fresh token from the operating system's randomness.
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0u8; RANDOM_BYTES];
        getrandom::fill(&mut bytes)?;
        let mut hex = String::with_capacity(RANDOM_BYTES * 2);
        for b in bytes {
            hex.push_str(&format!("{b:02x}"));
        }
        Ok(LaunchToken(Arc::from(hex)))
    }

    /// A token the operator chose (`DZPOS_API_TOKEN`). Refused when empty,
    /// shorter than 16 characters, or carrying anything a header cannot: a
    /// token that the middleware could never match is a server nobody can
    /// call, and that is better found at start than at the first request.
    pub fn from_secret(secret: &str) -> Result<Self, String> {
        if secret.chars().count() < 16 {
            return Err("the launch token must be at least 16 characters".to_owned());
        }
        if !secret.bytes().all(|b| b.is_ascii_graphic() || b == b' ') || secret.trim() != secret {
            return Err(
                "the launch token must be printable ASCII with no leading or trailing space"
                    .to_owned(),
            );
        }
        Ok(LaunchToken(Arc::from(secret)))
    }

    /// The secret itself, for the one place that hands it to the webview.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Whether `shown` is this token. Every byte is compared whatever the
    /// first mismatch, so the answer's timing does not narrow a guess.
    pub fn matches(&self, shown: &str) -> bool {
        let mine = self.0.as_bytes();
        let theirs = shown.as_bytes();
        if mine.len() != theirs.len() {
            return false;
        }
        mine.iter()
            .zip(theirs)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
    }
}

/// The middleware on every guarded route: `Authorization: Bearer <token>`
/// or a 401 in the usual envelope.
pub async fn require(
    State(token): State<LaunchToken>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let shown = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match shown {
        Some(shown) if token.matches(shown) => Ok(next.run(req).await),
        _ => Err(ApiError::Unauthorized),
    }
}

#[cfg(test)]
mod tests {
    use super::LaunchToken;

    #[test]
    fn a_generated_token_is_64_hex_characters_and_never_the_same_twice() {
        let a = LaunchToken::generate().map_err(|e| e.to_string());
        let b = LaunchToken::generate().map_err(|e| e.to_string());
        let (a, b) = match (a, b) {
            (Ok(a), Ok(b)) => (a, b),
            (a, b) => panic!("{a:?} {b:?}"),
        };
        assert_eq!(a.expose().len(), 64);
        assert!(a.expose().bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a.expose(), b.expose());
        assert!(a.matches(a.expose()) && !a.matches(b.expose()));
    }

    #[test]
    fn a_chosen_secret_is_checked_before_the_server_binds() {
        assert!(LaunchToken::from_secret("").is_err());
        assert!(LaunchToken::from_secret("short").is_err());
        assert!(LaunchToken::from_secret(" sixteen-chars-x ").is_err());
        assert!(LaunchToken::from_secret("sixteen\tchars-x-").is_err());
        assert!(LaunchToken::from_secret("sixteen-chars-ok").is_ok());
    }

    #[test]
    fn debug_output_keeps_the_secret() {
        let t = LaunchToken::from_secret("sixteen-chars-ok").map_err(|e| e.to_string());
        assert_eq!(format!("{t:?}"), "Ok(LaunchToken(..))");
    }
}
