//! What a REST API client reads when the database fails or the body is malformed, and what the log
//! holds.
//!
//! The test drops the `keys` table of the running tracker, so that adding a key fails with a real
//! database error. The reply must name the operation and the request id and hold nothing from the
//! driver; the log must hold what the tracker keeps of the error (for a statement that binds a key
//! only its kind and code, never the server's message, which can hold the key) and the same request id (rules 2 and 7 of the client
//! error text ADR,
//! `docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`).
//! The test binary installs a subscriber that captures every level, which only one binary can do.
use secrecy::ExposeSecret;
use sqlx::{Connection, SqliteConnection};
use torrust_tracker_axum_rest_api_server::testing::environment::Started;
use torrust_tracker_test_helpers::{configuration, logging};

async fn drop_the_keys_table(database_path: &std::path::Path) {
    let mut connection = SqliteConnection::connect(&format!("sqlite://{}", database_path.display()))
        .await
        .expect("the test database should open");
    sqlx::query("DROP TABLE keys")
        .execute(&mut connection)
        .await
        .expect("the keys table should drop");
}

#[tokio::test]
async fn it_should_tell_the_client_the_failed_operation_and_the_request_id_and_log_the_database_error() {
    // Arrange
    logging::setup_capturing_every_level();
    let (mut cfg, database) = configuration::ephemeral_with_sqlite_database();
    cfg.core.private = true;
    let env = Started::new(&cfg.into()).await;
    drop_the_keys_table(&database.path()).await;
    let connection_info = env.get_connection_info();
    let admin_token = connection_info.api_token.as_ref().unwrap().expose_secret().to_string();
    let url = format!("{}api/v1/keys", connection_info.origin);

    // Act
    let response = reqwest::Client::new()
        .post(url)
        .bearer_auth(&admin_token)
        .json(&serde_json::json!({"key": "TestPeerKeyInDroppedTable0000001", "seconds_valid": 60}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .expect("the reply should carry the request id header")
        .to_str()
        .unwrap()
        .to_string();
    let body = response.text().await.unwrap();
    env.stop().await;

    // Assert
    assert_eq!(status, 500);
    assert_eq!(
        body,
        format!(
            "Unhandled rejection: Err {{ reason: \"failed to add key: internal tracker error (request id: {request_id})\" }}"
        )
    );
    let logs = logging::captured_logs();
    let error_line = logs
        .lines()
        .find(|line| {
            line.contains(" ERROR ") && line.contains("the database rejected the statement") && line.contains(&request_id)
        })
        .unwrap_or_else(|| {
            panic!("the log should hold an ERROR event with the database error and the request id {request_id}, got: {logs}")
        });
    assert!(error_line.contains("the request failed inside the tracker"), "{error_line}");
}

#[tokio::test]
async fn it_should_answer_a_malformed_json_body_with_one_sentence_and_no_serde_text() {
    // Arrange
    logging::setup_capturing_every_level();
    let env = Started::new(&configuration::ephemeral().into()).await;
    let connection_info = env.get_connection_info();
    let admin_token = connection_info.api_token.as_ref().unwrap().expose_secret().to_string();
    let url = format!("{}api/v1/keys", connection_info.origin);

    // Act
    let response = reqwest::Client::new()
        .post(url)
        .bearer_auth(&admin_token)
        .header("content-type", "application/json")
        .body("{bad")
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.text().await.unwrap();
    env.stop().await;

    // Assert
    assert_eq!(status, 400);
    assert_eq!(
        body,
        "Invalid request body: expected a JSON object with the fields key (string or null) and seconds_valid (number or null)"
    );
}
