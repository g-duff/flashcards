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

async fn create_vocabulary_entry(
    base_url: &str,
    client: &reqwest::Client,
    source_text: &str,
    target_text: &str,
    category_id: i64,
) -> i64 {
    let response = client
        .post(format!("{base_url}/api/vocabulary-entries"))
        .json(&json!({
            "source_language": "es",
            "source_text": source_text,
            "target_language": "en",
            "target_text": target_text,
            "category_ids": [category_id],
        }))
        .send()
        .await
        .expect("request failed");
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    body["data"]["id"].as_i64().unwrap()
}

/// A `reqwest::Client` with an in-memory cookie jar, plus a freshly created
/// current Learner, so requests carry the `learner_id` cookie the practice
/// session endpoints require.
async fn client_with_current_learner(base_url: &str) -> reqwest::Client {
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

    client
}

/// Seeds a Category with five distinct fruit entries, each with enough
/// Language Pair siblings to produce four distractors.
async fn seed_five_eligible_entries(base_url: &str, client: &reqwest::Client) -> i64 {
    let category_id = create_category(base_url, client, "Fruit").await;
    create_vocabulary_entry(base_url, client, "manzana", "apple", category_id).await;
    create_vocabulary_entry(base_url, client, "naranja", "orange", category_id).await;
    create_vocabulary_entry(base_url, client, "platano", "banana", category_id).await;
    create_vocabulary_entry(base_url, client, "uva", "grape", category_id).await;
    create_vocabulary_entry(base_url, client, "pera", "pear", category_id).await;
    category_id
}

// `POST /api/practice-sessions` creates an active session with generated,
// snapshotted questions (grilled-spec.md sec. 2, 4, 5; ticket 07).
#[tokio::test]
async fn creating_a_practice_session_returns_created_with_questions() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = client_with_current_learner(&base_url).await;
    let category_id = seed_five_eligible_entries(&base_url, &client).await;

    let response = client
        .post(format!("{base_url}/api/practice-sessions"))
        .json(&json!({
            "category_id": category_id,
            "direction": "source_to_target",
            "question_count": 10,
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["status"], "active");
    assert_eq!(body["data"]["category_id"], category_id);
    assert_eq!(body["data"]["direction"], "source_to_target");
    assert_eq!(body["data"]["requested_question_count"], 10);
    // Only five entries are eligible, so the session contains the
    // available count rather than the requested count (grilled-spec.md
    // sec. 4; ticket 07).
    assert_eq!(body["data"]["actual_question_count"], 5);
    let questions = body["data"]["questions"].as_array().unwrap();
    assert_eq!(questions.len(), 5);
}

// At creation, every question snapshots prompt text, options, and
// ordering, and the public response never reveals `is_correct`
// (grilled-spec.md sec. 5; ticket 07).
#[tokio::test]
async fn created_session_questions_hide_correctness_and_have_six_options() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = client_with_current_learner(&base_url).await;
    let category_id = seed_five_eligible_entries(&base_url, &client).await;

    let response = client
        .post(format!("{base_url}/api/practice-sessions"))
        .json(&json!({
            "category_id": category_id,
            "direction": "source_to_target",
            "question_count": 10,
        }))
        .send()
        .await
        .expect("request failed");

    let body: serde_json::Value = response.json().await.expect("invalid json body");
    let questions = body["data"]["questions"].as_array().unwrap();

    for (index, question) in questions.iter().enumerate() {
        assert_eq!(question["ordinal"], (index + 1) as i64);
        assert!(question["prompt_text"].is_string());
        assert_eq!(question["direction"], "source_to_target");

        let options = question["options"].as_array().unwrap();
        // Four translation options (one correct, three... four incorrect)
        // plus "Don't know" (ticket 07).
        assert_eq!(options.len(), 6);
        for option in options {
            assert!(
                option.get("is_correct").is_none(),
                "is_correct must not be present pre-submission"
            );
            assert!(option["id"].is_number());
            assert!(option["text"].is_string());
        }
        assert_eq!(
            options.iter().filter(|o| o["is_dont_know"] == true).count(),
            1
        );
        assert_eq!(options.last().unwrap()["text"], "Don't know");
    }
}

// Requested question counts outside the server-configured bounds are
// rejected with `400` (spec.md story 34; ticket 07).
#[tokio::test]
async fn creating_a_session_with_an_out_of_range_question_count_is_rejected() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = client_with_current_learner(&base_url).await;
    let category_id = seed_five_eligible_entries(&base_url, &client).await;

    let response = client
        .post(format!("{base_url}/api/practice-sessions"))
        .json(&json!({
            "category_id": category_id,
            "direction": "source_to_target",
            "question_count": 999,
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(body["error"]["details"][0]["field"], "question_count");
}

// Distractors are sourced from the selected Category first (grilled-
// spec.md sec. 4; ticket 07): a Category with five entries of its own is
// entirely self-sufficient and needs no fallback.
#[tokio::test]
async fn distractors_prefer_the_selected_category() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = client_with_current_learner(&base_url).await;
    let category_id = seed_five_eligible_entries(&base_url, &client).await;
    // Another Category, same Language Pair, that would also be able to
    // supply distractors if fallback were needed.
    let other_category_id = create_category(&base_url, &client, "Animals").await;
    create_vocabulary_entry(&base_url, &client, "perro", "dog", other_category_id).await;

    let response = client
        .post(format!("{base_url}/api/practice-sessions"))
        .json(&json!({
            "category_id": category_id,
            "direction": "source_to_target",
            "question_count": 10,
        }))
        .send()
        .await
        .expect("request failed");

    let body: serde_json::Value = response.json().await.expect("invalid json body");
    let questions = body["data"]["questions"].as_array().unwrap();
    for question in questions {
        let texts: Vec<String> = question["options"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["is_dont_know"] == false)
            .map(|o| o["text"].as_str().unwrap().to_string())
            .collect();
        assert!(!texts.contains(&"dog".to_string()));
    }
}

// When the selected Category alone can't supply four distinct distractors,
// the server falls back to other Categories in the same Language Pair
// (grilled-spec.md sec. 4; ticket 07).
#[tokio::test]
async fn distractors_fall_back_to_other_categories_in_the_same_language_pair() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = client_with_current_learner(&base_url).await;
    let category_id = create_category(&base_url, &client, "Fruit").await;
    create_vocabulary_entry(&base_url, &client, "manzana", "apple", category_id).await;

    let other_category_id = create_category(&base_url, &client, "Animals").await;
    create_vocabulary_entry(&base_url, &client, "perro", "dog", other_category_id).await;
    create_vocabulary_entry(&base_url, &client, "gato", "cat", other_category_id).await;
    create_vocabulary_entry(&base_url, &client, "pajaro", "bird", other_category_id).await;
    create_vocabulary_entry(&base_url, &client, "pez", "fish", other_category_id).await;

    let response = client
        .post(format!("{base_url}/api/practice-sessions"))
        .json(&json!({
            "category_id": category_id,
            "direction": "source_to_target",
            "question_count": 10,
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["data"]["actual_question_count"], 1);
}

// An entry whose Language Pair can't supply four distinct incorrect
// options is omitted from the session (grilled-spec.md sec. 4; ticket 07).
#[tokio::test]
async fn entries_without_enough_distractors_are_omitted() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = client_with_current_learner(&base_url).await;
    let category_id = create_category(&base_url, &client, "Fruit").await;
    // Only two entries share this Language Pair: not enough for four
    // distinct incorrect distractors.
    create_vocabulary_entry(&base_url, &client, "manzana", "apple", category_id).await;
    create_vocabulary_entry(&base_url, &client, "naranja", "orange", category_id).await;

    let response = client
        .post(format!("{base_url}/api/practice-sessions"))
        .json(&json!({
            "category_id": category_id,
            "direction": "source_to_target",
            "question_count": 10,
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 409);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "NO_ELIGIBLE_QUESTIONS");
}

// Zero Eligible Entries rejects session creation with a clear message
// rather than a `500` (grilled-spec.md sec. 9; ticket 07).
#[tokio::test]
async fn zero_eligible_entries_rejects_session_creation() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = client_with_current_learner(&base_url).await;
    let category_id = create_category(&base_url, &client, "Empty").await;

    let response = client
        .post(format!("{base_url}/api/practice-sessions"))
        .json(&json!({
            "category_id": category_id,
            "direction": "source_to_target",
            "question_count": 10,
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 409);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "NO_ELIGIBLE_QUESTIONS");
}

// `GET /api/practice-sessions/:id` reads the session snapshot and status
// (grilled-spec.md sec. 5; ticket 07).
#[tokio::test]
async fn reading_a_practice_session_returns_its_snapshot() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = client_with_current_learner(&base_url).await;
    let category_id = seed_five_eligible_entries(&base_url, &client).await;

    let create_response = client
        .post(format!("{base_url}/api/practice-sessions"))
        .json(&json!({
            "category_id": category_id,
            "direction": "source_to_target",
            "question_count": 10,
        }))
        .send()
        .await
        .expect("request failed");
    let created: serde_json::Value = create_response.json().await.expect("invalid json body");
    let session_id = created["data"]["id"].as_i64().unwrap();

    let response = client
        .get(format!("{base_url}/api/practice-sessions/{session_id}"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["data"]["id"], session_id);
    assert_eq!(body["data"]["status"], "active");
    assert_eq!(body["data"]["questions"].as_array().unwrap().len(), 5);
}

// Reading an unknown Practice Session returns 404 (ticket 07).
#[tokio::test]
async fn reading_an_unknown_practice_session_returns_not_found() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base_url}/api/practice-sessions/999"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 404);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "PRACTICE_SESSION_NOT_FOUND");
}

// With no current-learner cookie, creating a session is rejected rather
// than silently attributed to some other Learner (grilled-spec.md sec. 5;
// ticket 07).
#[tokio::test]
async fn creating_a_session_without_a_current_learner_is_rejected() {
    let (base_url, _db_guard) = spawn_app().await;
    let client = reqwest::Client::new();
    let category_id = seed_five_eligible_entries(&base_url, &client).await;

    let response = client
        .post(format!("{base_url}/api/practice-sessions"))
        .json(&json!({
            "category_id": category_id,
            "direction": "source_to_target",
            "question_count": 10,
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.expect("invalid json body");
    assert_eq!(body["error"]["code"], "LEARNER_NOT_SELECTED");
}
