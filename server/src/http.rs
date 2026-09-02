//! Imperative shell for the HTTP layer: the axum router, the shared
//! [`AppState`], the request handlers ([`handlers`]), and the
//! error-to-status mapping ([`error`]). The pure validation logic the
//! handlers lean on lives in [`crate::core`].

pub mod error;
pub mod handlers;

use std::sync::Arc;

use axum::routing::{get, patch, post};

use crate::core::Scheduler;
use crate::store::Db;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    /// The scheduling strategy, resolved once in `main` (ADR-0001).
    /// Handlers hold it as a trait object so swapping Leitner for another
    /// impl is a one-line change there.
    pub scheduler: Arc<dyn Scheduler + Send + Sync>,
}

/// Builds the application router. `main` owns constructing the state;
/// this owns the URL-to-handler map.
///
/// Routes are as the binary sees them — nginx strips the
/// `/flashcards/api/` prefix before proxying, and serves
/// `/flashcards/openapi.yaml` from `/openapi.yaml` here.
pub fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/healthz", get(handlers::healthz))
        .route("/openapi.yaml", get(handlers::openapi))
        .route(
            "/terms",
            get(handlers::list_terms).post(handlers::create_term),
        )
        .route("/terms/import", post(handlers::import_terms))
        .route(
            "/terms/:id",
            patch(handlers::patch_term).delete(handlers::delete_term),
        )
        .route("/cards", get(handlers::list_cards))
        .route("/cards/:id/reviews", post(handlers::create_review))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use serde_json::{Value, json};
    use tempfile::TempDir;
    use tower::ServiceExt;

    /// A router over a throwaway on-disk database. The returned `TempDir`
    /// must be kept alive for the duration of the test.
    fn app() -> (TempDir, axum::Router) {
        let dir = TempDir::new().expect("tempdir");
        let db = Db::open(&dir.path().join("flashcards.db")).expect("open db");
        let state = AppState {
            db,
            scheduler: Arc::new(crate::core::Leitner),
        };
        (dir, router(state))
    }

    async fn send(app: &axum::Router, req: Request<Body>) -> (StatusCode, Value) {
        let res = app.clone().oneshot(req).await.expect("request");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("body");
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("json body")
        };
        (status, body)
    }

    fn get(uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("request")
    }

    fn post(uri: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request")
    }

    fn patch(uri: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method("PATCH")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request")
    }

    fn delete(uri: &str) -> Request<Body> {
        Request::builder()
            .method("DELETE")
            .uri(uri)
            .body(Body::empty())
            .expect("request")
    }

    fn new_term_body() -> Value {
        json!({
            "foreign_lang": "es",
            "foreign_text": "perro",
            "pivot_text": "dog",
            "notes": "el perro (m)"
        })
    }

    #[tokio::test]
    async fn create_then_list_a_term() {
        let (_dir, app) = app();

        let (status, created) = send(&app, post("/terms", new_term_body())).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(created["foreign_text"], "perro");
        let id = created["id"].as_str().expect("id string").to_string();
        assert_eq!(id.len(), 36, "a UUID id");

        let (status, list) = send(&app, get("/terms")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(list.as_array().expect("array").len(), 1);
        assert_eq!(list[0]["id"], id);
    }

    #[tokio::test]
    async fn re_posting_the_same_term_is_idempotent() {
        let (_dir, app) = app();

        let (_, first) = send(&app, post("/terms", new_term_body())).await;
        let (status, again) = send(&app, post("/terms", new_term_body())).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(first["id"], again["id"]);

        let (_, list) = send(&app, get("/terms")).await;
        assert_eq!(list.as_array().expect("array").len(), 1);
    }

    #[tokio::test]
    async fn empty_text_is_a_400_with_an_error_body() {
        let (_dir, app) = app();

        let (status, body) = send(
            &app,
            post(
                "/terms",
                json!({ "foreign_lang": "es", "foreign_text": "", "pivot_text": "x" }),
            ),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            body["error"]
                .as_str()
                .expect("error string")
                .contains("foreign_text")
        );
    }

    #[tokio::test]
    async fn a_malformed_body_is_a_400_with_an_error_body() {
        let (_dir, app) = app();

        // Valid JSON, but `pivot_text` is missing — the Json extractor
        // rejects it. The convention says 400 `{error}`, not axum's 422.
        let (status, body) = send(
            &app,
            post(
                "/terms",
                json!({ "foreign_lang": "es", "foreign_text": "perro" }),
            ),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].is_string());
    }

    #[tokio::test]
    async fn import_adds_every_term_and_its_two_cards() {
        let (_dir, app) = app();

        let (status, report) = send(
            &app,
            post(
                "/terms/import",
                json!([
                    { "foreign_lang": "es", "foreign_text": "perro", "pivot_text": "dog" },
                    { "foreign_lang": "es", "foreign_text": "gato", "pivot_text": "cat",
                      "notes": "el gato" },
                ]),
            ),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(report, json!({ "imported": 2, "skipped": 0 }));

        let (_, list) = send(&app, get("/terms")).await;
        assert_eq!(list.as_array().expect("array").len(), 2);
        let (_, cards) = send(&app, get("/cards")).await;
        assert_eq!(cards.as_array().expect("array").len(), 4);
    }

    #[tokio::test]
    async fn re_importing_an_overlapping_file_skips_the_terms_already_present() {
        let (_dir, app) = app();
        let batch = json!([
            { "foreign_lang": "es", "foreign_text": "perro", "pivot_text": "dog" },
            { "foreign_lang": "es", "foreign_text": "gato", "pivot_text": "cat" },
        ]);

        let (_, first) = send(&app, post("/terms/import", batch.clone())).await;
        assert_eq!(first, json!({ "imported": 2, "skipped": 0 }));

        let (status, second) = send(&app, post("/terms/import", batch)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second, json!({ "imported": 0, "skipped": 2 }));

        let (_, list) = send(&app, get("/terms")).await;
        assert_eq!(list.as_array().expect("array").len(), 2, "no duplicates");
    }

    #[tokio::test]
    async fn import_with_one_bad_element_is_a_400_naming_the_index_and_persists_nothing() {
        let (_dir, app) = app();

        let (status, body) = send(
            &app,
            post(
                "/terms/import",
                json!([
                    { "foreign_lang": "es", "foreign_text": "perro", "pivot_text": "dog" },
                    { "foreign_lang": "es", "foreign_text": "", "pivot_text": "x" },
                ]),
            ),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        let message = body["error"].as_str().expect("error string");
        assert!(message.contains('1'), "names the element index: {message}");
        assert!(message.contains("foreign_text"), "names the field: {message}");

        // All-or-nothing: the valid element 0 was not written either.
        let (_, list) = send(&app, get("/terms")).await;
        assert!(list.as_array().expect("array").is_empty());
        let (_, cards) = send(&app, get("/cards")).await;
        assert!(cards.as_array().expect("array").is_empty());
    }

    #[tokio::test]
    async fn patch_notes_on_a_known_term() {
        let (_dir, app) = app();
        let (_, created) = send(&app, post("/terms", new_term_body())).await;
        let id = created["id"].as_str().expect("id").to_string();

        let (status, patched) = send(
            &app,
            patch(&format!("/terms/{id}"), json!({ "notes": "el perro" })),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(patched["notes"], "el perro");

        // The change is persisted, not just echoed.
        let (_, list) = send(&app, get("/terms")).await;
        assert_eq!(list[0]["notes"], "el perro");
    }

    #[tokio::test]
    async fn patch_unknown_term_is_404() {
        let (_dir, app) = app();

        let (status, body) = send(
            &app,
            patch("/terms/does-not-exist", json!({ "notes": "x" })),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "term not found");
    }

    #[tokio::test]
    async fn delete_a_term_then_deleting_again_is_404() {
        let (_dir, app) = app();
        let (_, created) = send(&app, post("/terms", new_term_body())).await;
        let id = created["id"].as_str().expect("id").to_string();

        let (status, body) = send(&app, delete(&format!("/terms/{id}"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["deleted"], id);

        let (status, _) = send(&app, delete(&format!("/terms/{id}"))).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    /// Add the sample Term and return `(term_id, [PracticeCard, ...])`.
    async fn seed_term(app: &axum::Router) -> (String, Vec<Value>) {
        let (_, created) = send(app, post("/terms", new_term_body())).await;
        let term_id = created["id"].as_str().expect("id").to_string();
        let (status, cards) = send(app, get("/cards")).await;
        assert_eq!(status, StatusCode::OK);
        let cards = cards.as_array().expect("array").clone();
        (term_id, cards)
    }

    #[tokio::test]
    async fn creating_a_term_yields_two_practice_cards() {
        let (_dir, app) = app();
        let (term_id, cards) = seed_term(&app).await;

        assert_eq!(cards.len(), 2);
        let foreign = cards
            .iter()
            .find(|c| c["prompt_side"] == "foreign")
            .expect("foreign card");
        let pivot = cards
            .iter()
            .find(|c| c["prompt_side"] == "pivot")
            .expect("pivot card");

        assert_eq!(foreign["prompt"], "perro");
        assert_eq!(foreign["answer"], "dog");
        assert_eq!(foreign["term_id"], term_id);
        assert_eq!(foreign["box"], 1);
        assert_eq!(pivot["prompt"], "dog");
        assert_eq!(pivot["answer"], "perro");
        assert_eq!(pivot["notes"], "el perro (m)");
    }

    #[tokio::test]
    async fn due_before_filters_the_card_list() {
        let (_dir, app) = app();
        let (_, cards) = seed_term(&app).await;
        let first = cards[0]["id"].as_str().expect("id").to_string();

        // Pass one Card: its due date jumps ~2 days out.
        let (status, _) = send(
            &app,
            post(
                &format!("/cards/{first}/reviews"),
                json!({ "rating": "pass" }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        // A cut-off one day out drops the passed Card, keeps its sibling.
        // `Z` form so the `+` of a numeric offset needn't be URL-encoded.
        let cutoff = (Utc::now() + chrono::Duration::days(1))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let (status, due) = send(&app, get(&format!("/cards?due_before={cutoff}&limit=10"))).await;
        assert_eq!(status, StatusCode::OK);
        let due = due.as_array().expect("array");
        assert_eq!(due.len(), 1);
        assert_ne!(due[0]["id"], first);
    }

    #[tokio::test]
    async fn a_pass_then_a_fail_move_the_box_and_due_date() {
        let (_dir, app) = app();
        let (_, cards) = seed_term(&app).await;
        let id = cards[0]["id"].as_str().expect("id").to_string();
        let due_before = cards[0]["due_at"].as_str().expect("due_at").to_string();

        let (status, passed) = send(
            &app,
            post(&format!("/cards/{id}/reviews"), json!({ "rating": "pass" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(passed["box"], 2);
        assert!(
            passed["due_at"].as_str().unwrap() > due_before.as_str(),
            "a pass pushes due_at out",
        );

        let (status, failed) = send(
            &app,
            post(&format!("/cards/{id}/reviews"), json!({ "rating": "fail" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(failed["box"], 1, "a fail resets to box 1");
    }

    #[tokio::test]
    async fn a_review_on_an_unknown_card_is_404() {
        let (_dir, app) = app();
        let (status, body) = send(
            &app,
            post("/cards/no-such-card/reviews", json!({ "rating": "pass" })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "card not found");
    }

    #[tokio::test]
    async fn a_bad_rating_is_400() {
        let (_dir, app) = app();
        let (_, cards) = seed_term(&app).await;
        let id = cards[0]["id"].as_str().expect("id").to_string();

        let (status, body) = send(
            &app,
            post(
                &format!("/cards/{id}/reviews"),
                json!({ "rating": "maybe" }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].is_string());
    }

    #[tokio::test]
    async fn an_unparseable_due_before_is_400() {
        let (_dir, app) = app();
        let (status, body) = send(&app, get("/cards?due_before=not-a-date")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            body["error"]
                .as_str()
                .expect("error string")
                .contains("due_before")
        );
    }

    #[tokio::test]
    async fn deleting_a_term_removes_its_cards() {
        let (_dir, app) = app();
        let (term_id, _) = seed_term(&app).await;

        let (status, _) = send(&app, delete(&format!("/terms/{term_id}"))).await;
        assert_eq!(status, StatusCode::OK);

        let (status, cards) = send(&app, get("/cards")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(cards.as_array().expect("array").is_empty());
    }
}
