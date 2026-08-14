//! Current-learner cookie: identity derivation for Learner-scoped
//! operations. The cookie stores the durable Learner ID directly and must
//! never be superseded by a Learner ID supplied in a request body
//! (grilled-spec.md sec. 2, sec. 5).
//!
//! Implemented directly against `axum::http` headers, deliberately avoiding
//! a typed cookie-jar crate: the `cookie` crate's builder only accepts
//! `time`-typed durations/timestamps, and `Max-Age` is plain relative
//! seconds, so hand-rolling the header keeps `chrono` as the project's only
//! date/time dependency.

use axum::http::HeaderValue;

use crate::learners::LearnerId;

pub const LEARNER_COOKIE_NAME: &str = "learner_id";
const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

/// Builds the `Set-Cookie` header value for a newly created or selected
/// Learner. Host-only (no `Domain` attribute), `HttpOnly`, `SameSite=Lax`,
/// with an explicit long expiry expressed as a relative `Max-Age`
/// (grilled-spec.md sec. 2).
pub fn learner_cookie_header(learner_id: LearnerId, lifetime_days: i64) -> HeaderValue {
    let max_age_seconds = lifetime_days * SECONDS_PER_DAY;
    build_header(&learner_id.to_string(), max_age_seconds)
}

/// The `Set-Cookie` header value that clears the current-learner cookie.
/// Used both for explicit sign-out and for clearing an invalid or
/// deleted-profile cookie (grilled-spec.md sec. 9).
pub fn clear_learner_cookie_header() -> HeaderValue {
    build_header("", 0)
}

fn build_header(value: &str, max_age_seconds: i64) -> HeaderValue {
    let raw = format!(
        "{LEARNER_COOKIE_NAME}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_seconds}"
    );
    // `value` is always either empty or a `LearnerId`'s decimal digits, and
    // the rest of the header is a fixed ASCII template, so this can never
    // contain characters that would make it an invalid header value.
    HeaderValue::from_str(&raw).unwrap_or_else(|source| {
        unreachable!("cookie header value {raw:?} was not valid ASCII: {source}")
    })
}

/// Extracts the raw `learner_id` cookie value out of a `Cookie` request
/// header, if that cookie is present (regardless of whether its value turns
/// out to be well-formed).
pub fn learner_cookie_value(cookie_header: &str) -> Option<&str> {
    cookie_header.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name.trim() == LEARNER_COOKIE_NAME).then(|| value.trim())
    })
}

/// Parses the current-learner ID out of a raw `Cookie` request header
/// value. A missing cookie, or one holding a non-numeric value, is treated
/// as absent, the same as a deleted profile.
pub fn parse_learner_cookie(cookie_header: &str) -> Option<LearnerId> {
    parse_learner_cookie_presence(cookie_header).flatten()
}

/// Parses the current-learner ID out of a raw `Cookie` request header value,
/// distinguishing "no `learner_id` cookie was sent" (`None`) from "one was
/// sent but held a non-numeric value" (`Some(None)`). Callers that need to
/// react differently to a malformed cookie (e.g. clearing it) should use
/// this instead of [`parse_learner_cookie`], which collapses both cases.
pub fn parse_learner_cookie_presence(cookie_header: &str) -> Option<Option<LearnerId>> {
    let raw_value = learner_cookie_value(cookie_header)?;
    Some(raw_value.parse::<i64>().ok().map(LearnerId))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The current-learner cookie is host-only, HttpOnly, SameSite=Lax, and
    // has an explicit long expiry (grilled-spec.md sec. 2).
    #[test]
    fn learner_cookie_header_is_host_only_http_only_and_lax_with_explicit_expiry() {
        let header = learner_cookie_header(LearnerId(42), 365);
        let value = header.to_str().unwrap();

        assert!(value.starts_with("learner_id=42;"));
        assert!(value.contains("HttpOnly"));
        assert!(value.contains("SameSite=Lax"));
        assert!(value.contains("Path=/"));
        assert!(value.contains(&format!("Max-Age={}", 365 * SECONDS_PER_DAY)));
        assert!(!value.contains("Domain="));
    }

    #[test]
    fn clear_header_expires_immediately() {
        let header = clear_learner_cookie_header();
        let value = header.to_str().unwrap();

        assert!(value.starts_with("learner_id=;"));
        assert!(value.contains("Max-Age=0"));
    }

    #[test]
    fn parses_the_learner_cookie_out_of_a_multi_cookie_header() {
        assert_eq!(
            parse_learner_cookie("theme=dark; learner_id=42; other=1"),
            Some(LearnerId(42))
        );
    }

    #[test]
    fn returns_none_when_the_learner_cookie_is_absent() {
        assert_eq!(parse_learner_cookie("theme=dark"), None);
        assert_eq!(parse_learner_cookie(""), None);
    }

    #[test]
    fn rejects_a_non_numeric_cookie_value() {
        assert_eq!(parse_learner_cookie("learner_id=not-a-number"), None);
    }
}
