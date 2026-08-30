//! Imperative shell for the HTTP layer: the axum router, the shared
//! [`AppState`], the request handlers ([`handlers`]), and the
//! error-to-status mapping ([`error`]). The pure validation logic the
//! handlers lean on lives in [`crate::core`].

pub mod error;
pub mod handlers;

use axum::routing::{get, patch};

use crate::store::Db;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
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
        .route(
            "/terms/:id",
            patch(handlers::patch_term).delete(handlers::delete_term),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use tempfile::TempDir;
    use tower::ServiceExt;

    /// A router over a throwaway on-disk database. The returned `TempDir`
    /// must be kept alive for the duration of the test.
    fn app() -> (TempDir, axum::Router) {
        let dir = TempDir::new().expect("tempdir");
        let db = Db::open(&dir.path().join("flashcards.db")).expect("open db");
        (dir, router(AppState { db }))
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
}
