//! What an HTTP tracker client reads when the database fails, and what the log holds.
//!
//! An announce reads the persisted download count of a torrent that is not in memory when
//! `persistent_torrent_completed_stat` is on. The test drops the `torrents` table of the running
//! tracker, so that read fails with a real database error. The client must learn which operation
//! failed and the request id, and nothing from the driver; the log must hold the driver text and the
//! same request id (rules 2 and 7 of the client error text ADR,
//! `docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`).
//! The test binary installs a subscriber that captures every level, which only one binary can do.
use std::sync::Arc;
use std::time::Duration;

use sqlx::{Connection, SqliteConnection};
use torrust_tracker_axum_http_server::testing::environment::Started;
use torrust_tracker_client::http::client::Client;
use torrust_tracker_test_helpers::{configuration, logging};

const INFO_HASH: &str = "info_hash=%3B%24U%04%CF%5F%11%BB%DB%E1%20%1C%EAjk%F4Z%EE%1B%C0";
const ANNOUNCE_QUERY: &str =
    "info_hash=%3B%24U%04%CF%5F%11%BB%DB%E1%20%1C%EAjk%F4Z%EE%1B%C0&peer_id=-qB00000000000000001&port=17548&left=0";

async fn drop_the_torrents_table(database_path: &std::path::Path) {
    let mut connection = SqliteConnection::connect(&format!("sqlite://{}", database_path.display()))
        .await
        .expect("the test database should open");
    sqlx::query("DROP TABLE torrents")
        .execute(&mut connection)
        .await
        .expect("the torrents table should drop");
}

/// Sends `path_and_query` to a tracker whose `torrents` table was dropped after start, and checks
/// that the reply names `operation` and the request id and holds no driver text, and that the log
/// holds the driver text and the same request id.
async fn assert_a_database_failure_is_reported_as_an_internal_failure(path_and_query: &str, operation: &str) {
    // Arrange
    logging::setup_capturing_every_level();
    let (mut cfg, database) = configuration::ephemeral_with_sqlite_database();
    cfg.core.tracker_policy.persistent_torrent_completed_stat = true;
    let env = Started::new(
        &Arc::new(cfg.core.clone()),
        &Arc::new(cfg.http_trackers.clone().unwrap()[0].clone()),
    )
    .await;
    drop_the_torrents_table(&database.path()).await;
    let client = Client::new(env.base_url(), Duration::from_secs(5)).unwrap();

    // Act
    let response = client.get(path_and_query).await.unwrap();
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
    assert!(
        body.contains(&format!(
            "Internal tracker error: the {operation} could not be completed, try again later (request id: {request_id})"
        )),
        "the client should read the operation and its request id, got: {body}"
    );
    for driver_text in [
        "no such table",
        "Sqlite3",
        "SQLite",
        "sqlx",
        "error returned from database",
        "malformed",
    ] {
        assert!(
            !body.contains(driver_text),
            "the reply holds driver text `{driver_text}`: {body}"
        );
    }
    let logs = logging::captured_logs();
    let error_line = logs
        .lines()
        .find(|line| line.contains(" ERROR ") && line.contains("no such table") && line.contains(&request_id))
        .unwrap_or_else(|| {
            panic!("the log should hold an ERROR event with the driver text and the request id {request_id}, got: {logs}")
        });
    assert!(error_line.contains("the request failed inside the tracker"), "{error_line}");
}

#[tokio::test]
async fn it_should_tell_an_announcing_client_the_failed_operation_and_the_request_id_and_log_the_driver_text() {
    assert_a_database_failure_is_reported_as_an_internal_failure(&format!("announce?{ANNOUNCE_QUERY}"), "announce").await;
}

#[tokio::test]
async fn it_should_tell_a_scraping_client_the_failed_operation_and_the_request_id_and_log_the_driver_text() {
    assert_a_database_failure_is_reported_as_an_internal_failure(&format!("scrape?{INFO_HASH}"), "scrape").await;
}
