use server::config::{AlgorithmDefaults, Config};
use server::state::AppState;
use server::{db, routes};

/// Boots the real axum app against a freshly migrated, isolated SQLite
/// database and returns its base URL. The database file lives in a
/// tempdir that is deleted when the returned guard is dropped.
pub async fn spawn_app() -> (String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("failed to create tempdir for test database");
    let db_path = dir.path().join("test.db");
    let database_url = format!("sqlite://{}", db_path.display());

    let pool = db::connect_and_migrate(&database_url, "migrations")
        .await
        .expect("failed to migrate test database");

    let config = Config {
        database_url,
        migrations_path: "migrations".to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        session_inactivity_timeout_minutes: 30,
        cookie_lifetime_days: 365,
        question_count_min: 10,
        question_count_max: 20,
        supported_languages: vec!["en".to_string(), "es".to_string()],
        algorithm_defaults: AlgorithmDefaults {
            correct_streak_threshold: 5,
            incorrect_boost_threshold: 2,
            deprioritize_duration_days: 3.0,
            prioritize_duration_days: 2.0,
            min_interval_before_retest_days: 1.0,
            incorrect_weight: 3.0,
            time_decay_factor: 0.5,
            base_priority: 0.0,
        },
    };

    let state = AppState::new(pool, config);
    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind test listener");
    let addr = listener
        .local_addr()
        .expect("failed to read test listener address");

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("test server failed");
    });

    (format!("http://{addr}"), dir)
}
