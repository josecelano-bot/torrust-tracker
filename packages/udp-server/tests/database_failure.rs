//! What a UDP tracker client reads when the database fails, and what the log holds.
//!
//! An announce reads the persisted download count of a torrent that is not in memory when
//! `persistent_torrent_completed_stat` is on. The test drops the `torrents` table of the running
//! tracker, so that read fails with a real database error. The client must read the operation and
//! the request id and nothing from the driver; the log line of the failure must hold the driver text
//! and the same request id (rules 2 and 7 of the client error text ADR,
//! `docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`).
//! The test binary installs a subscriber that captures every level, which only one binary can do.
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use sqlx::{Connection, SqliteConnection};
use torrust_peer_id::PeerId;
use torrust_tracker_client::udp::client::UdpTrackerClient;
use torrust_tracker_test_helpers::{configuration, logging};
use torrust_tracker_udp_protocol::{
    AnnounceActionPlaceholder, AnnounceEvent, AnnounceRequest, ConnectRequest, InfoHash, NumberOfBytes, NumberOfPeers, PeerKey,
    Port, Response, TransactionId,
};

const TIMEOUT: Duration = Duration::from_secs(5);

async fn drop_the_torrents_table(database_path: &std::path::Path) {
    let mut connection = SqliteConnection::connect(&format!("sqlite://{}", database_path.display()))
        .await
        .expect("the test database should open");
    sqlx::query("DROP TABLE torrents")
        .execute(&mut connection)
        .await
        .expect("the torrents table should drop");
}

#[tokio::test]
async fn it_should_tell_the_client_the_failed_operation_and_the_request_id_and_log_the_driver_text() {
    // Arrange
    logging::setup_capturing_every_level();
    let (mut cfg, database) = configuration::ephemeral_with_sqlite_database();
    cfg.core.tracker_policy.persistent_torrent_completed_stat = true;
    let core_config = Arc::new(cfg.core.clone());
    let udp_tracker_config = Arc::new(cfg.udp_trackers.clone().unwrap()[0].clone());
    let env = torrust_tracker_udp_server::testing::environment::Started::new(&core_config, &udp_tracker_config).await;
    drop_the_torrents_table(&database.path()).await;
    let client = UdpTrackerClient::new(env.bind_address(), TIMEOUT)
        .await
        .expect("the UDP client should connect");
    let transaction_id = TransactionId::new(123);
    client
        .send(ConnectRequest { transaction_id }.into())
        .await
        .expect("the connect request should be sent");
    let Response::Connect(connect) = client.receive().await.expect("a connect response") else {
        panic!("the tracker should answer the connect request with a connect response");
    };
    let announce = AnnounceRequest {
        connection_id: connect.connection_id,
        action_placeholder: AnnounceActionPlaceholder::default(),
        transaction_id,
        info_hash: InfoHash([7u8; 20]),
        peer_id: PeerId([255u8; 20]),
        bytes_downloaded: NumberOfBytes(0i64.into()),
        bytes_uploaded: NumberOfBytes(0i64.into()),
        bytes_left: NumberOfBytes(0i64.into()),
        event: AnnounceEvent::Started.into(),
        ip_address: Ipv4Addr::UNSPECIFIED.into(),
        key: PeerKey::new(0i32),
        peers_wanted: NumberOfPeers(1i32.into()),
        port: Port(client.client.socket.local_addr().unwrap().port().into()),
    };

    // Act
    client.send(announce.into()).await.expect("the announce should be sent");
    let response = client.receive().await.expect("an announce reply");
    env.stop().await;

    // Assert
    let Response::Error(error) = response else {
        panic!("the tracker should answer with an error packet, got {response:?}");
    };
    let message = error.message.to_string();
    let request_id = message
        .strip_prefix("Internal tracker error: the announce could not be completed, try again later (request id: ")
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or_else(|| panic!("the client should read the operation and a request id, got: {message}"));
    for driver_text in [
        "no such table",
        "Sqlite3",
        "SQLite",
        "sqlx",
        "error returned from database",
        "malformed",
    ] {
        assert!(
            !message.contains(driver_text),
            "the reply holds driver text `{driver_text}`: {message}"
        );
    }
    let logs = logging::captured_logs();
    assert!(
        logs.lines()
            .any(|line| line.contains(" ERROR ") && line.contains("no such table") && line.contains(request_id)),
        "the log should hold an ERROR line with the driver text and the request id {request_id}, got: {logs}"
    );
}
