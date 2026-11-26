//! The reply to an HTTP tracker client for a failure of the tracker core.
//!
//! Client text is chosen here from the class and the variant of the error, not taken from the
//! `Display` chain of the outer error (rule 3 of the client error text ADR,
//! `docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`):
//! a refusal says what was refused in one sentence, and a failure inside the tracker (the database)
//! sends a fixed sentence that holds nothing from its source.
use torrust_tracker_core::authentication::key::Error as AuthenticationKeyError;
use torrust_tracker_core::error::{AnnounceError, ScrapeError, TrackerCoreError, WhitelistError};
use torrust_tracker_http_protocol::v1::responses::error::Error as HttpProtocolErrorResponse;

pub fn protocol_error_from_tracker_core_error(error: TrackerCoreError) -> HttpProtocolErrorResponse {
    match error {
        TrackerCoreError::AnnounceError { source } => match source {
            AnnounceError::Whitelist(whitelist_error) => not_whitelisted(&whitelist_error, "announce"),
            _ => HttpProtocolErrorResponse::internal("announce"),
        },
        TrackerCoreError::ScrapeError { source } => match source {
            ScrapeError::Whitelist(whitelist_error) => not_whitelisted(&whitelist_error, "scrape"),
            _ => HttpProtocolErrorResponse::internal("scrape"),
        },
        TrackerCoreError::WhitelistError { source } => not_whitelisted(&source, "request"),
        TrackerCoreError::AuthenticationError { source } => match source {
            AuthenticationKeyError::KeyVerificationError { .. } => HttpProtocolErrorResponse::internal("authentication"),
            refusal => HttpProtocolErrorResponse {
                failure_reason: refusal.to_string(),
            },
        },
        _ => HttpProtocolErrorResponse::internal("request"),
    }
}

/// The refusal for a torrent that is not whitelisted: one sentence that names the torrent.
fn not_whitelisted(error: &WhitelistError, operation: &str) -> HttpProtocolErrorResponse {
    match error {
        WhitelistError::TorrentNotWhitelisted { info_hash } => HttpProtocolErrorResponse {
            failure_reason: format!("The torrent {info_hash} is not whitelisted on this tracker"),
        },
        _ => HttpProtocolErrorResponse::internal(operation),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rstest::rstest;
    use torrust_tracker_core::authentication::key::Error as AuthenticationKeyError;
    use torrust_tracker_core::databases::error::Error as DatabaseError;
    use torrust_tracker_core::error::{AnnounceError, ScrapeError, TrackerCoreError, WhitelistError};
    use torrust_tracker_primitives::Driver;

    use super::protocol_error_from_tracker_core_error;
    use crate::tests::sample_info_hash;

    fn not_whitelisted() -> WhitelistError {
        WhitelistError::TorrentNotWhitelisted {
            info_hash: sample_info_hash(),
        }
    }

    /// A database failure whose source holds a text no client may read.
    fn a_database_failure(marker: &str) -> DatabaseError {
        DatabaseError::ServerError {
            source: Arc::new(std::io::Error::other(marker.to_string())),
            driver: Driver::MySQL,
        }
    }

    fn the_whitelist_sentence() -> String {
        format!("The torrent {} is not whitelisted on this tracker", sample_info_hash())
    }

    #[rstest]
    #[case::announce(TrackerCoreError::from(AnnounceError::from(not_whitelisted())))]
    #[case::scrape(TrackerCoreError::from(ScrapeError::from(not_whitelisted())))]
    #[case::whitelist(TrackerCoreError::from(not_whitelisted()))]
    fn it_should_refuse_a_torrent_that_is_not_whitelisted_in_one_sentence_whatever_layer_reports_it(
        #[case] error: TrackerCoreError,
    ) {
        let response = protocol_error_from_tracker_core_error(error);

        assert_eq!(response.failure_reason, the_whitelist_sentence());
        assert!(!response.is_internal());
    }

    #[rstest]
    #[case::missing(
        AuthenticationKeyError::MissingAuthKey,
        "Missing authentication key: this tracker is private, use /announce/<key>"
    )]
    #[case::not_registered(AuthenticationKeyError::UnableToReadKey, "Authentication key not registered")]
    #[case::expired(AuthenticationKeyError::KeyExpired, "Authentication key expired")]
    fn it_should_tell_the_client_what_the_key_check_refused_without_a_prefix(
        #[case] error: AuthenticationKeyError,
        #[case] expected: &str,
    ) {
        let response = protocol_error_from_tracker_core_error(TrackerCoreError::from(error));

        assert_eq!(response.failure_reason, expected);
        assert!(!response.is_internal());
    }

    #[test]
    fn it_should_treat_a_key_check_that_could_not_run_as_an_internal_failure() {
        let marker = "TestMarkerOfTheKeyStore0001";
        let error = AuthenticationKeyError::KeyVerificationError {
            source: Arc::new(std::io::Error::other(marker.to_string())),
        };

        let response = protocol_error_from_tracker_core_error(TrackerCoreError::from(error));

        assert_eq!(
            response.failure_reason,
            "Internal tracker error: the authentication could not be completed, try again later"
        );
        assert!(!response.failure_reason.contains(marker));
    }

    #[rstest]
    #[case::announce(
        TrackerCoreError::from(AnnounceError::from(a_database_failure("TestDriverMarker0001"))),
        "announce"
    )]
    #[case::scrape(
        TrackerCoreError::from(ScrapeError::from(a_database_failure("TestDriverMarker0001"))),
        "scrape"
    )]
    fn it_should_treat_a_database_failure_as_an_internal_failure_that_names_the_operation_only(
        #[case] error: TrackerCoreError,
        #[case] operation: &str,
    ) {
        let response = protocol_error_from_tracker_core_error(error);

        assert_eq!(
            response.failure_reason,
            format!("Internal tracker error: the {operation} could not be completed, try again later")
        );
        assert!(response.is_internal());
        assert!(!response.failure_reason.contains("TestDriverMarker0001"));
        assert!(!response.failure_reason.contains("MySQL"));
    }
}
