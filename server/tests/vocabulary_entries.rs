mod common;

use common::spawn_app;
use serde_json::json;

async fn create_category(base_url: &str, client: &reqwest::Client, name: &str) -> i64 {
    let response = client
        .post(format!("{base_url}/api/categories"))
        .json(&json!({ "name": name }))
        .send()
        .await
        .expect("request failed");
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    body["data"]["id"].as_i64().unwrap()
}

// `POST /api/vocabulary-entries` creates one entry from source/target text,
// languages, and one or more Category Memberships (spec.md story 13, 16, 20;
// ticket 05).
#[tokio::test]
async fn creating_a_vocabulary_entry_returns_created() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let category_id = create_category(&base_url, &client, "Fruit").await;

    let response = client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "ES",
            "source_text": "manzana",
            "target_language": "EN",
            "target_text": "apple",
            "category_ids": [category_id],
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["source_language"], "es");
    assert_eq!(body["data"]["source_text"], "manzana");
    assert_eq!(body["data"]["target_language"], "en");
    assert_eq!(body["data"]["target_text"], "apple");
    assert_eq!(body["data"]["category_ids"], json!([category_id]));
    assert!(body["data"]["id"].is_number());
}

// A Vocabulary Entry without at least one Category is rejected with `400`
// (spec.md story 16; ticket 05).
#[tokio::test]
async fn creating_a_vocabulary_entry_without_a_category_is_rejected() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es",
            "source_text": "manzana",
            "target_language": "en",
            "target_text": "apple",
            "category_ids": [],
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(body["error"]["details"][0]["field"], "category_ids");
}

// Unsupported language codes are rejected with `400` (spec.md story 20;
// ticket 05).
#[tokio::test]
async fn creating_a_vocabulary_entry_with_an_unsupported_language_is_rejected() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let category_id = create_category(&base_url, &client, "Fruit").await;

    let response = client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "fr",
            "source_text": "pomme",
            "target_language": "en",
            "target_text": "apple",
            "category_ids": [category_id],
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(body["error"]["details"][0]["field"], "source_language");
}

// Duplicate language-and-text pairs are rejected globally with `409`
// (spec.md story 15; ticket 05).
#[tokio::test]
async fn duplicate_vocabulary_entry_pairs_conflict() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let category_id = create_category(&base_url, &client, "Fruit").await;

    client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es",
            "source_text": "manzana",
            "target_language": "en",
            "target_text": "apple",
            "category_ids": [category_id],
        }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es",
            "source_text": "  manzana  ",
            "target_language": "en",
            "target_text": "apple",
            "category_ids": [category_id],
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 409);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "VOCABULARY_ENTRY_CONFLICT");
}

// `GET /api/vocabulary-entries` lists entries, optionally filtered by
// Category (spec.md route table; ticket 05).
#[tokio::test]
async fn listing_vocabulary_entries_filters_by_category() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let fruit_id = create_category(&base_url, &client, "Fruit").await;
    let animals_id = create_category(&base_url, &client, "Animals").await;

    client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es", "source_text": "manzana",
            "target_language": "en", "target_text": "apple",
            "category_ids": [fruit_id],
        }))
        .send()
        .await
        .expect("request failed");
    client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es", "source_text": "perro",
            "target_language": "en", "target_text": "dog",
            "category_ids": [animals_id],
        }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .get(format!(
            "{base_url}/api/vocabulary-entries?category_id={fruit_id}"
        ))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    let entries = body["data"].as_array().expect("expected an array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["source_text"], "manzana");
}

// `GET /api/vocabulary-entries/:id` reads a single entry (ticket 05).
#[tokio::test]
async fn reading_a_vocabulary_entry_returns_it() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let category_id = create_category(&base_url, &client, "Fruit").await;

    let create_response = client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es", "source_text": "manzana",
            "target_language": "en", "target_text": "apple",
            "category_ids": [category_id],
        }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");
    let entry_id = created["data"]["id"].as_i64().unwrap();

    let response = client
        .get(format!("{base_url}/api/vocabulary-entries/{entry_id}"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["data"]["id"], entry_id);
    assert_eq!(body["data"]["source_text"], "manzana");
}

// Reading an unknown Vocabulary Entry returns 404 (ticket 05).
#[tokio::test]
async fn reading_an_unknown_vocabulary_entry_returns_not_found() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base_url}/api/vocabulary-entries/999"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 404);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "VOCABULARY_ENTRY_NOT_FOUND");
}

// `PATCH /api/vocabulary-entries/:id` edits text and Category Memberships
// (spec.md story 19; ticket 05).
#[tokio::test]
async fn editing_a_vocabulary_entry_updates_text_and_memberships() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let fruit_id = create_category(&base_url, &client, "Fruit").await;
    let snacks_id = create_category(&base_url, &client, "Snacks").await;

    let create_response = client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es", "source_text": "manzana",
            "target_language": "en", "target_text": "apple",
            "category_ids": [fruit_id],
        }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");
    let entry_id = created["data"]["id"].as_i64().unwrap();

    let response = client
        .patch(format!("{base_url}/api/vocabulary-entries/{entry_id}"))
        .json(&json!({
            "source_text": "manzana roja",
            "target_text": "red apple",
            "category_ids": [fruit_id, snacks_id],
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["data"]["id"], entry_id);
    assert_eq!(body["data"]["source_text"], "manzana roja");
    assert_eq!(body["data"]["target_text"], "red apple");
    let mut category_ids: Vec<i64> = body["data"]["category_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_i64().unwrap())
        .collect();
    category_ids.sort();
    assert_eq!(category_ids, vec![fruit_id, snacks_id]);
}

// Source and target languages are immutable after creation and rejected if
// a change is attempted (spec.md story 21; ticket 05).
#[tokio::test]
async fn editing_a_vocabulary_entry_language_is_rejected() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let category_id = create_category(&base_url, &client, "Fruit").await;

    let create_response = client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es", "source_text": "manzana",
            "target_language": "en", "target_text": "apple",
            "category_ids": [category_id],
        }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");
    let entry_id = created["data"]["id"].as_i64().unwrap();

    let response = client
        .patch(format!("{base_url}/api/vocabulary-entries/{entry_id}"))
        .json(&json!({
            "source_text": "manzana",
            "target_text": "apple",
            "category_ids": [category_id],
            "source_language": "fr",
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(body["error"]["details"][0]["field"], "source_language");
}

// `DELETE /api/vocabulary-entries/:id` removes the entry and its Category
// Memberships in one transaction (spec.md story 22, 23; ticket 05).
#[tokio::test]
async fn deleting_a_vocabulary_entry_removes_it_and_its_memberships() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let category_id = create_category(&base_url, &client, "Fruit").await;

    let create_response = client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es", "source_text": "manzana",
            "target_language": "en", "target_text": "apple",
            "category_ids": [category_id],
        }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");
    let entry_id = created["data"]["id"].as_i64().unwrap();

    let response = client
        .delete(format!("{base_url}/api/vocabulary-entries/{entry_id}"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert!(body["data"].is_null());

    let get_response = client
        .get(format!("{base_url}/api/vocabulary-entries/{entry_id}"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(get_response.status(), 404);

    // The Category itself is safe to delete now that no entry depends on it.
    let delete_category_response = client
        .delete(format!("{base_url}/api/categories/{category_id}"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(delete_category_response.status(), 200);
}

// Deleting an unknown Vocabulary Entry returns 404 (ticket 05).
#[tokio::test]
async fn deleting_an_unknown_vocabulary_entry_returns_not_found() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .delete(format!("{base_url}/api/vocabulary-entries/999"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 404);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "VOCABULARY_ENTRY_NOT_FOUND");
}

// `DELETE /api/categories/:id` is rejected with `409` when it would remove
// the final Category Membership of a Vocabulary Entry (spec.md story 11,
// 12; deferred from ticket 04, picked up in ticket 05).
#[tokio::test]
async fn deleting_a_category_that_would_orphan_an_entry_conflicts() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let category_id = create_category(&base_url, &client, "Fruit").await;

    client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es", "source_text": "manzana",
            "target_language": "en", "target_text": "apple",
            "category_ids": [category_id],
        }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .delete(format!("{base_url}/api/categories/{category_id}"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 409);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "CATEGORY_WOULD_ORPHAN_ENTRY");
}

// Deleting a Category is still safe when the dependent entry belongs to
// another Category too (spec.md story 11; ticket 05).
#[tokio::test]
async fn deleting_a_category_with_another_membership_present_succeeds() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let fruit_id = create_category(&base_url, &client, "Fruit").await;
    let snacks_id = create_category(&base_url, &client, "Snacks").await;

    client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es", "source_text": "manzana",
            "target_language": "en", "target_text": "apple",
            "category_ids": [fruit_id, snacks_id],
        }))
        .send()
        .await
        .expect("request failed");

    let response = client
        .delete(format!("{base_url}/api/categories/{fruit_id}"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
}
