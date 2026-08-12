mod common;

use common::spawn_app;

// Server exposes a health/status endpoint returning the project's standard
// success envelope (01-project-scaffolding.md).
#[tokio::test]
async fn health_endpoint_returns_success_envelope() {
    let (base_url, _db_guard) = spawn_app().await;

    let response = reqwest::get(format!("{base_url}/api/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("invalid json body");

    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["status"], "ok");
    assert!(body["meta"]["timestamp"].is_string());
    assert!(body.get("error").is_none());
}
