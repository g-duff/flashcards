mod common;

use common::spawn_app;
use serde_json::json;

// `POST /api/categories` creates a shared Category (spec.md story 8;
// ticket 04).
#[tokio::test]
async fn creating_a_category_returns_created() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Animals" }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["name"], "Animals");
    assert!(body["data"]["id"].is_number());
    assert!(body["meta"]["timestamp"].is_string());
}

// Empty (post-trim) names are rejected with a field-level validation error
// (ticket 04).
#[tokio::test]
async fn creating_a_category_with_an_empty_name_is_rejected() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/api/categories"))
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

// Category names are compared case-insensitively after trimming
// whitespace; duplicates return 409 (spec.md story 10, 11, 12; ticket 04).
#[tokio::test]
async fn duplicate_category_names_conflict_case_and_whitespace_insensitively() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Animals" }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "  animals  " }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 409);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["status"], "error");
    assert_eq!(body["error"]["code"], "CATEGORY_NAME_CONFLICT");
}

// `GET /api/categories` lists Categories, defaulting to creation-date order
// (spec.md story 9; ticket 04).
#[tokio::test]
async fn listing_categories_defaults_to_creation_order() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Zebra" }))
        .send()
        .await
        .expect("request failed");
    client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Apple" }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .get(format!("{base_url}/api/categories"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    let names: Vec<&str> = body["data"]
        .as_array()
        .expect("expected an array")
        .iter()
        .map(|category| category["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Zebra", "Apple"]);
}

// `GET /api/categories?sort=alphabetical` sorts case-insensitively by name
// (spec.md story 9; ticket 04).
#[tokio::test]
async fn listing_categories_supports_alphabetical_sort() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Zebra" }))
        .send()
        .await
        .expect("request failed");
    client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Apple" }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .get(format!("{base_url}/api/categories?sort=alphabetical"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    let names: Vec<&str> = body["data"]
        .as_array()
        .expect("expected an array")
        .iter()
        .map(|category| category["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Apple", "Zebra"]);
}

// `GET /api/categories/:id` reads a single Category (ticket 04).
#[tokio::test]
async fn reading_a_category_returns_it() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let create_response = client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Animals" }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");
    let category_id = created["data"]["id"].as_i64().unwrap();

    let response = client
        .get(format!("{base_url}/api/categories/{category_id}"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["data"]["id"], category_id);
    assert_eq!(body["data"]["name"], "Animals");
}

// Reading an unknown Category returns 404 (ticket 04).
#[tokio::test]
async fn reading_an_unknown_category_returns_not_found() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base_url}/api/categories/999"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 404);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "CATEGORY_NOT_FOUND");
}

// `PATCH /api/categories/:id` renames a Category while preserving its
// durable ID (spec.md story 10; ticket 04).
#[tokio::test]
async fn renaming_a_category_preserves_identity() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let create_response = client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Animals" }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");
    let category_id = created["data"]["id"].as_i64().unwrap();

    let response = client
        .patch(format!("{base_url}/api/categories/{category_id}"))
        .json(&json!({ "name": "  Wild Animals  " }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["data"]["id"], category_id);
    assert_eq!(body["data"]["name"], "Wild Animals");
}

// Renaming into a name already used by another Category conflicts (spec.md
// story 10; ticket 04).
#[tokio::test]
async fn renaming_a_category_to_an_existing_name_conflicts() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Animals" }))
        .send()
        .await
        .expect("request failed");

    let plants_response = client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Plants" }))
        .send()
        .await
        .expect("request failed");
    let plants: serde_json::Value = plants_response.json().await.expect("invalid json body");
    let plants_id = plants["data"]["id"].as_i64().unwrap();

    let response = client
        .patch(format!("{base_url}/api/categories/{plants_id}"))
        .json(&json!({ "name": "  animals  " }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 409);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "CATEGORY_NAME_CONFLICT");
}

// Renaming an unknown Category returns 404 (ticket 04).
#[tokio::test]
async fn renaming_an_unknown_category_returns_not_found() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .patch(format!("{base_url}/api/categories/999"))
        .json(&json!({ "name": "Ghost" }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 404);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "CATEGORY_NOT_FOUND");
}

// `DELETE /api/categories/:id` removes a Category with no Vocabulary
// Entries depending on it (spec.md story 11; ticket 04). Rejection of
// unsafe deletion is covered once Vocabulary Entries exist (ticket 05).
#[tokio::test]
async fn deleting_a_category_with_no_dependents_succeeds() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let create_response = client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": "Animals" }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");
    let category_id = created["data"]["id"].as_i64().unwrap();

    let response = client
        .delete(format!("{base_url}/api/categories/{category_id}"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["status"], "success");
    assert!(body["data"].is_null());

    let list_response = client
        .get(format!("{base_url}/api/categories"))
        .send()
        .await
        .expect("request failed");
    let list_body: serde_json::Value = list_response.json().await.expect("invalid json body");
    assert_eq!(list_body["data"].as_array().unwrap().len(), 0);
}

// Deleting an unknown Category returns 404 (ticket 04).
#[tokio::test]
async fn deleting_an_unknown_category_returns_not_found() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .delete(format!("{base_url}/api/categories/999"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 404);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "CATEGORY_NOT_FOUND");
}
