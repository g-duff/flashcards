mod common;

use common::spawn_app;
use serde_json::json;

// `POST /api/learners` creates a Learner with a unique display name and
// sets the current-learner cookie (spec.md story 1; ticket 02).
#[tokio::test]
async fn creating_a_learner_returns_created_and_sets_cookie() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/api/learners"))
        .json(&json!({ "name": "Alice" }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 201);

    let set_cookie = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .expect("expected a Set-Cookie header")
        .to_str()
        .expect("set-cookie header was not valid utf-8")
        .to_string();

    assert!(set_cookie.starts_with("learner_id="));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    assert!(set_cookie.contains("Path=/"));
    assert!(set_cookie.contains("Max-Age="));
    assert!(!set_cookie.contains("Domain="));

    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["name"], "Alice");
    assert!(body["data"]["id"].is_number());
    assert!(body["meta"]["timestamp"].is_string());
}

// `GET /api/learners` lists Learners for profile selection (spec.md
// story 2; ticket 02).
#[tokio::test]
async fn listing_learners_returns_created_learners() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base_url}/api/learners"))
        .json(&json!({ "name": "Alice" }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .get(format!("{base_url}/api/learners"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    let learners = body["data"].as_array().expect("expected an array");
    assert_eq!(learners.len(), 1);
    assert_eq!(learners[0]["name"], "Alice");
}

// Empty (post-trim) names are rejected with a field-level validation error.
#[tokio::test]
async fn creating_a_learner_with_an_empty_name_is_rejected() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/api/learners"))
        .json(&json!({ "name": "   " }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["status"], "error");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(body["error"]["details"][0]["field"], "name");
}

// Learner names are compared case-insensitively after trimming whitespace;
// duplicates return 409 (spec.md story 6; ticket 02).
#[tokio::test]
async fn duplicate_learner_names_conflict_case_and_whitespace_insensitively() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base_url}/api/learners"))
        .json(&json!({ "name": "Alice" }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .post(format!("{base_url}/api/learners"))
        .json(&json!({ "name": "  alice  " }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 409);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["status"], "error");
    assert_eq!(body["error"]["code"], "LEARNER_NAME_CONFLICT");
}

// `POST /api/session/learner` selects an existing Learner and sets the
// cookie (spec.md story 2; ticket 02).
#[tokio::test]
async fn selecting_an_existing_learner_sets_the_cookie() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let create_response = client
        .post(format!("{base_url}/api/learners"))
        .json(&json!({ "name": "Alice" }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");
    let learner_id = created["data"]["id"].as_i64().unwrap();

    let response = client
        .post(format!("{base_url}/api/session/learner"))
        .json(&json!({ "learner_id": learner_id }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let set_cookie = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .expect("expected a Set-Cookie header");
    assert!(set_cookie.to_str().unwrap().starts_with("learner_id="));

    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["data"]["id"], learner_id);
}

// Selecting an unknown Learner ID does not let a caller act as another
// Learner; it simply fails to resolve (spec.md story 32; ticket 02).
#[tokio::test]
async fn selecting_an_unknown_learner_returns_not_found() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/api/session/learner"))
        .json(&json!({ "learner_id": 999 }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 404);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "LEARNER_NOT_FOUND");
}

// `DELETE /api/session/learner` clears the cookie (ticket 02).
#[tokio::test]
async fn clearing_the_session_removes_the_cookie() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .expect("failed to build client");

    client
        .post(format!("{base_url}/api/learners"))
        .json(&json!({ "name": "Alice" }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .delete(format!("{base_url}/api/session/learner"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["status"], "success");
    assert!(body["data"].is_null());

    let current = client
        .get(format!("{base_url}/api/session/learner"))
        .send()
        .await
        .expect("request failed");
    let current_body: serde_json::Value = current.json().await.expect("invalid json body");
    assert!(current_body["data"].is_null());
}

// `GET /api/session/learner` reflects the Learner identified by the cookie
// (spec.md story 3; ticket 02).
#[tokio::test]
async fn current_learner_reflects_the_session_cookie() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .expect("failed to build client");

    let create_response = client
        .post(format!("{base_url}/api/learners"))
        .json(&json!({ "name": "Alice" }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");

    let response = client
        .get(format!("{base_url}/api/session/learner"))
        .send()
        .await
        .expect("request failed");

    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["data"]["id"], created["data"]["id"]);
}

// An invalid (deleted-profile or malformed) cookie is cleared by the
// server; the client uses the resulting null identity to redirect to Home
// (spec.md story 4; ticket 02).
#[tokio::test]
async fn an_invalid_cookie_is_cleared_by_the_server() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base_url}/api/session/learner"))
        .header(reqwest::header::COOKIE, "learner_id=999999")
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);

    let set_cookie = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .expect("expected the invalid cookie to be cleared")
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.starts_with("learner_id="));
    assert!(set_cookie.contains("Max-Age=0") || set_cookie.to_lowercase().contains("expires="));

    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert!(body["data"].is_null());
}

// With no cookie at all, there is simply no current Learner (not an error).
#[tokio::test]
async fn no_cookie_means_no_current_learner() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base_url}/api/session/learner"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    assert!(
        response
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .is_none()
    );

    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert!(body["data"].is_null());
}
