//! Current-learner cookie: identity derivation for Learner-scoped
//! operations. The cookie stores the durable Learner ID directly and must
//! never be superseded by a Learner ID supplied in a request body
//! (grilled-spec.md sec. 2, sec. 5).

use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration;

use crate::learners::LearnerId;

pub const LEARNER_COOKIE_NAME: &str = "learner_id";

/// Builds the `Set-Cookie` for a newly created or selected Learner. Host-only
/// (no `Domain` attribute), `HttpOnly`, `SameSite=Lax`, with an explicit long
/// expiry (grilled-spec.md sec. 2).
pub fn learner_cookie(learner_id: LearnerId, lifetime_days: i64) -> Cookie<'static> {
    Cookie::build((LEARNER_COOKIE_NAME, learner_id.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(Duration::days(lifetime_days))
        .build()
}

/// The cookie identity (name + path) used to remove the current-learner
/// cookie via [`axum_extra::extract::cookie::CookieJar::remove`]. Used both
/// for explicit sign-out and for clearing an invalid or deleted-profile
/// cookie (grilled-spec.md sec. 9).
pub fn learner_cookie_key() -> Cookie<'static> {
    Cookie::build(LEARNER_COOKIE_NAME).path("/").build()
}

/// Parses a raw cookie value into a Learner ID. A non-numeric value is
/// treated as invalid, the same as a deleted profile.
pub fn parse_learner_id(raw: &str) -> Option<LearnerId> {
    raw.trim().parse::<i64>().ok().map(LearnerId)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The current-learner cookie is host-only, HttpOnly, SameSite=Lax, and
    // has an explicit long expiry (grilled-spec.md sec. 2).
    #[test]
    fn learner_cookie_is_host_only_http_only_and_lax_with_explicit_expiry() {
        let cookie = learner_cookie(LearnerId(42), 365);

        assert_eq!(cookie.name(), LEARNER_COOKIE_NAME);
        assert_eq!(cookie.value(), "42");
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.domain(), None);
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
        assert_eq!(cookie.max_age(), Some(Duration::days(365)));
    }

    #[test]
    fn parses_a_valid_numeric_cookie_value() {
        assert_eq!(parse_learner_id("42"), Some(LearnerId(42)));
    }

    #[test]
    fn rejects_a_non_numeric_cookie_value() {
        assert_eq!(parse_learner_id("not-a-number"), None);
    }

    #[test]
    fn rejects_an_empty_cookie_value() {
        assert_eq!(parse_learner_id(""), None);
    }
}
