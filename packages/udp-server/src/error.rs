//! Error types for the UDP server.
use std::fmt::Display;

use derive_more::derive::Display;
use thiserror::Error;
use torrust_tracker_core::error::{AnnounceError, ScrapeError, WhitelistError};
use torrust_tracker_udp_core::services::announce::UdpAnnounceError;
use torrust_tracker_udp_core::services::scrape::UdpScrapeError;
use torrust_tracker_udp_protocol::{ConnectionId, RequestParseError, TransactionId};
use uuid::Uuid;

#[derive(Display, Debug)]
#[display(":?")]
pub struct ConnectionCookie(pub ConnectionId);

/// Error returned by the UDP server.
///
/// This internal type carries implementation details and must not be used as a
/// new event payload without the stable reason classification required by the
/// [general error-events EPIC](../../../docs/issues/drafts/generalize-error-events/EPIC.md).
#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    /// Error returned when the request is invalid.
    #[error("error parsing request: {request_parse_error:?}")]
    InvalidRequest { request_parse_error: SendableRequestParseError },

    /// Error returned when the domain tracker returns an announce error.
    #[error("tracker announce error: {source}")]
    AnnounceFailed { source: UdpAnnounceError },

    /// Error returned when the domain tracker returns an scrape error.
    #[error("tracker scrape error: {source}")]
    ScrapeFailed { source: UdpScrapeError },

    /// Error returned from the wire-protocol crate (`torrust_tracker_udp_protocol`).
    #[error("internal server error: {message}")]
    Internal { message: String },

    /// Error returned when tracker requires authentication.
    #[error("domain tracker requires authentication but is not supported in current UDP implementation")]
    AuthRequired,
}

/// What every connection cookie error tells the client: it needs a new connection id (BEP 15).
/// The decoded cookie and the server's time bounds stay in the log and the error event.
const CONNECTION_ID_TEXT: &str = "Invalid or expired connection id: send a new connect request";

impl Error {
    /// The message of the error packet the client receives.
    ///
    /// It is chosen from the class and the variant of the error, not from its `Display` (rule 3 of
    /// the client error text ADR,
    /// `docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`):
    /// a refusal or a bad request is one sentence, and a failure inside the tracker names the
    /// operation and `request_id`, which the log line of the failure carries too, and holds nothing
    /// from its source. `Display` stays the full text for the log and the event.
    #[must_use]
    pub fn client_message(&self, request_id: Uuid) -> String {
        match self {
            Self::InvalidRequest { request_parse_error } => format!("Invalid request: {}", request_parse_error.reply_message),
            Self::AnnounceFailed { source } => match source {
                UdpAnnounceError::ConnectionCookieError { .. } => CONNECTION_ID_TEXT.to_string(),
                UdpAnnounceError::InvalidAnnounceRequest { source } => format!("Invalid announce request: {source}"),
                UdpAnnounceError::TrackerCoreAnnounceError {
                    source: AnnounceError::Whitelist(whitelist_error),
                }
                | UdpAnnounceError::TrackerCoreWhitelistError { source: whitelist_error } => {
                    not_whitelisted(whitelist_error, "announce", request_id)
                }
                _ => internal_failure("announce", request_id),
            },
            Self::ScrapeFailed { source } => match source {
                UdpScrapeError::ConnectionCookieError { .. } => CONNECTION_ID_TEXT.to_string(),
                UdpScrapeError::TrackerCoreScrapeError {
                    source: ScrapeError::Whitelist(whitelist_error),
                }
                | UdpScrapeError::TrackerCoreWhitelistError { source: whitelist_error } => {
                    not_whitelisted(whitelist_error, "scrape", request_id)
                }
                _ => internal_failure("scrape", request_id),
            },
            Self::Internal { .. } => internal_failure("request", request_id),
            Self::AuthRequired => self.to_string(),
        }
    }
}

fn not_whitelisted(error: &WhitelistError, operation: &str, request_id: Uuid) -> String {
    match error {
        WhitelistError::TorrentNotWhitelisted { info_hash } => {
            format!("The torrent {info_hash} is not whitelisted on this tracker")
        }
        _ => internal_failure(operation, request_id),
    }
}

fn internal_failure(operation: &str, request_id: Uuid) -> String {
    format!("Internal tracker error: the {operation} could not be completed, try again later (request id: {request_id})")
}

impl From<RequestParseError> for Error {
    fn from(request_parse_error: RequestParseError) -> Self {
        Self::InvalidRequest {
            request_parse_error: request_parse_error.into(),
        }
    }
}

impl From<UdpAnnounceError> for Error {
    fn from(udp_announce_error: UdpAnnounceError) -> Self {
        Self::AnnounceFailed {
            source: udp_announce_error,
        }
    }
}

impl From<UdpScrapeError> for Error {
    fn from(udp_scrape_error: UdpScrapeError) -> Self {
        Self::ScrapeFailed {
            source: udp_scrape_error,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SendableRequestParseError {
    pub message: String,
    /// What the client is told: the fixed message of the protocol parser, or that the packet is
    /// too short when reading it failed. `message` keeps the parser's own text for the event.
    pub reply_message: String,
    pub opt_connection_id: Option<ConnectionId>,
    pub opt_transaction_id: Option<TransactionId>,
}

impl Display for SendableRequestParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SendableRequestParseError: message: {}, connection_id: {:?}, transaction_id: {:?}",
            self.message, self.opt_connection_id, self.opt_transaction_id
        )
    }
}

impl From<RequestParseError> for SendableRequestParseError {
    fn from(request_parse_error: RequestParseError) -> Self {
        let (message, reply_message, opt_connection_id, opt_transaction_id) = match request_parse_error {
            RequestParseError::Sendable {
                connection_id,
                transaction_id,
                err,
            } => (
                (*err).to_string(),
                (*err).to_string(),
                Some(connection_id),
                Some(transaction_id),
            ),
            RequestParseError::Unsendable { err } => {
                let reply_message = err.as_ref().right().map_or_else(
                    || "the packet is too short".to_string(),
                    |fixed_message| (*fixed_message).to_string(),
                );
                (err.to_string(), reply_message, None, None)
            }
        };

        Self {
            message,
            reply_message,
            opt_connection_id,
            opt_transaction_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use torrust_tracker_udp_core::connection_cookie::ConnectionCookieError;
    use torrust_tracker_udp_core::services::announce::UdpAnnounceError;
    use torrust_tracker_udp_core::services::scrape::UdpScrapeError;
    use torrust_tracker_udp_protocol::{ConnectionId, RequestParseError, TransactionId};
    use zerocopy::byteorder::network_endian::{I32, I64};

    use super::{Error, SendableRequestParseError};

    #[test]
    fn it_should_preserve_response_routing_identifiers_for_a_sendable_parse_error() {
        // Arrange
        let connection_id = ConnectionId(I64::new(12));
        let transaction_id = TransactionId(I32::new(34));
        let parse_error = RequestParseError::sendable_text("invalid announce request", connection_id, transaction_id);
        let expected = SendableRequestParseError {
            message: "invalid announce request".to_string(),
            reply_message: "invalid announce request".to_string(),
            opt_connection_id: Some(connection_id),
            opt_transaction_id: Some(transaction_id),
        };

        // Act
        let actual = SendableRequestParseError::from(parse_error);

        // Assert
        assert_eq!(actual, expected);
    }

    #[test]
    fn it_should_clear_response_routing_identifiers_for_an_unsendable_parse_error() {
        // Arrange
        let parse_error = RequestParseError::unsendable_text("invalid request action");
        let expected = SendableRequestParseError {
            message: "invalid request action".to_string(),
            reply_message: "invalid request action".to_string(),
            opt_connection_id: None,
            opt_transaction_id: None,
        };

        // Act
        let actual = SendableRequestParseError::from(parse_error);

        // Assert
        assert_eq!(actual, expected);
    }

    #[test]
    fn it_should_wrap_a_sendable_parse_error_as_an_invalid_request() {
        // Arrange
        let connection_id = ConnectionId(I64::new(12));
        let transaction_id = TransactionId(I32::new(34));
        let parse_error = RequestParseError::sendable_text("invalid scrape request", connection_id, transaction_id);

        // Act
        let actual = Error::from(parse_error);

        // Assert
        assert!(matches!(
            actual,
            Error::InvalidRequest {
                request_parse_error: SendableRequestParseError {
                    message,
                    opt_connection_id: Some(actual_connection_id),
                    opt_transaction_id: Some(actual_transaction_id),
                    ..
                },
            } if message == "invalid scrape request"
                && actual_connection_id == connection_id
                && actual_transaction_id == transaction_id
        ));
    }

    #[test]
    fn it_should_wrap_an_announce_service_error_as_an_announce_failure() {
        // Arrange
        let source = ConnectionCookieError::ValueExpired {
            expired_value: 12.0,
            min_value: 13.0,
        };
        let announce_error = UdpAnnounceError::ConnectionCookieError { source: source.clone() };

        // Act
        let actual = Error::from(announce_error);

        // Assert
        assert!(matches!(
            actual,
            Error::AnnounceFailed {
                source: UdpAnnounceError::ConnectionCookieError { source: actual_source },
            } if actual_source == source
        ));
    }

    #[test]
    fn it_should_wrap_a_scrape_service_error_as_a_scrape_failure() {
        // Arrange
        let source = ConnectionCookieError::ValueFromFuture {
            future_value: 14.0,
            max_value: 13.0,
        };
        let scrape_error = UdpScrapeError::ConnectionCookieError { source: source.clone() };

        // Act
        let actual = Error::from(scrape_error);

        // Assert
        assert!(matches!(
            actual,
            Error::ScrapeFailed {
                source: UdpScrapeError::ConnectionCookieError { source: actual_source },
            } if actual_source == source
        ));
    }

    #[test]
    fn it_should_not_hold_a_source_path_when_an_internal_error_is_displayed() {
        // Arrange
        let error = Error::Internal {
            message: "failure".to_string(),
        };

        // Act
        let message = error.to_string();

        // Assert
        assert_eq!(message, "internal server error: failure");
    }

    #[test]
    fn it_should_not_hold_a_source_path_when_an_authentication_required_error_is_displayed() {
        // Arrange
        let error = Error::AuthRequired;

        // Act
        let message = error.to_string();

        // Assert
        assert_eq!(
            message,
            "domain tracker requires authentication but is not supported in current UDP implementation"
        );
    }

    mod what_the_client_is_told {
        use std::sync::Arc;

        use rstest::rstest;
        use torrust_tracker_core::databases::error::Error as DatabaseError;
        use torrust_tracker_core::error::{AnnounceError, ScrapeError, WhitelistError};
        use torrust_tracker_primitives::Driver;
        use torrust_tracker_udp_core::connection_cookie::ConnectionCookieError;
        use torrust_tracker_udp_core::peer_builder::{ByteCountField, FromRequestError};
        use torrust_tracker_udp_core::services::announce::UdpAnnounceError;
        use torrust_tracker_udp_core::services::scrape::UdpScrapeError;
        use torrust_tracker_udp_protocol::RequestParseError;
        use uuid::Uuid;

        use crate::error::Error;

        const COOKIE_TEXT: &str = "Invalid or expired connection id: send a new connect request";

        fn request_id() -> Uuid {
            Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap()
        }

        fn not_whitelisted() -> WhitelistError {
            WhitelistError::TorrentNotWhitelisted {
                info_hash: "3b245504cf5f11bbdbe1201cea6a6bf45aee1bc0".parse().unwrap(),
            }
        }

        const WHITELIST_TEXT: &str = "The torrent 3b245504cf5f11bbdbe1201cea6a6bf45aee1bc0 is not whitelisted on this tracker";

        fn a_database_failure(marker: &str) -> DatabaseError {
            DatabaseError::ServerError {
                source: Arc::new(std::io::Error::other(marker.to_string())),
                driver: Driver::MySQL,
            }
        }

        #[rstest]
        #[case::expired(ConnectionCookieError::ValueExpired { expired_value: 1.5, min_value: 1_795_353_220.657_289_3 })]
        #[case::from_the_future(ConnectionCookieError::ValueFromFuture { future_value: 9.5, max_value: 3.5 })]
        #[case::not_normal(ConnectionCookieError::ValueNotNormal { not_normal_value: f64::NAN })]
        fn it_should_tell_the_same_thing_for_every_connection_cookie_error_and_no_clock_value(
            #[case] cookie: ConnectionCookieError,
        ) {
            let announce = Error::from(UdpAnnounceError::ConnectionCookieError { source: cookie.clone() });
            let scrape = Error::from(UdpScrapeError::ConnectionCookieError { source: cookie });

            assert_eq!(announce.client_message(request_id()), COOKIE_TEXT);
            assert_eq!(scrape.client_message(request_id()), COOKIE_TEXT);
        }

        #[test]
        fn it_should_name_the_parser_message_and_no_type_name_for_a_request_that_cannot_be_parsed() {
            let error = Error::from(RequestParseError::unsendable_text("Couldn't parse action"));

            assert_eq!(error.client_message(request_id()), "Invalid request: Couldn't parse action");
        }

        #[test]
        fn it_should_say_that_the_packet_is_too_short_when_reading_it_failed() {
            let short = std::io::Error::from(std::io::ErrorKind::UnexpectedEof);
            let error = Error::from(RequestParseError::unsendable_io(short));

            assert_eq!(error.client_message(request_id()), "Invalid request: the packet is too short");
        }

        #[test]
        fn it_should_keep_the_negative_byte_count_text() {
            let error = Error::from(UdpAnnounceError::InvalidAnnounceRequest {
                source: FromRequestError::NegativeNumberOfBytes {
                    field: ByteCountField::Left,
                    value: -5,
                },
            });

            assert_eq!(
                error.client_message(request_id()),
                "Invalid announce request: `left` must not be negative, got -5"
            );
        }

        #[test]
        fn it_should_refuse_a_torrent_that_is_not_whitelisted_in_one_sentence_for_announce_and_scrape() {
            let announce = Error::from(UdpAnnounceError::TrackerCoreWhitelistError {
                source: not_whitelisted(),
            });
            let announce_in_core = Error::from(UdpAnnounceError::TrackerCoreAnnounceError {
                source: AnnounceError::from(not_whitelisted()),
            });
            let scrape = Error::from(UdpScrapeError::TrackerCoreWhitelistError {
                source: not_whitelisted(),
            });
            let scrape_in_core = Error::from(UdpScrapeError::TrackerCoreScrapeError {
                source: ScrapeError::from(not_whitelisted()),
            });

            for error in [announce, announce_in_core, scrape, scrape_in_core] {
                assert_eq!(error.client_message(request_id()), WHITELIST_TEXT);
            }
        }

        #[test]
        fn it_should_send_a_database_failure_as_an_internal_failure_with_the_operation_and_the_request_id_only() {
            let marker = "TestDriverMarker0001";
            let announce = Error::from(UdpAnnounceError::TrackerCoreAnnounceError {
                source: AnnounceError::from(a_database_failure(marker)),
            });
            let scrape = Error::from(UdpScrapeError::TrackerCoreScrapeError {
                source: ScrapeError::from(a_database_failure(marker)),
            });

            assert_eq!(
                announce.client_message(request_id()),
                "Internal tracker error: the announce could not be completed, try again later (request id: 11111111-2222-3333-4444-555555555555)"
            );
            assert_eq!(
                scrape.client_message(request_id()),
                "Internal tracker error: the scrape could not be completed, try again later (request id: 11111111-2222-3333-4444-555555555555)"
            );
            assert!(!announce.client_message(request_id()).contains(marker));
        }

        #[test]
        fn it_should_hide_the_message_of_an_internal_error_and_keep_the_authentication_text() {
            let internal = Error::Internal {
                message: "TestMarkerInternal0001".to_string(),
            };

            assert_eq!(
                internal.client_message(request_id()),
                "Internal tracker error: the request could not be completed, try again later (request id: 11111111-2222-3333-4444-555555555555)"
            );
            assert_eq!(
                Error::AuthRequired.client_message(request_id()),
                "domain tracker requires authentication but is not supported in current UDP implementation"
            );
        }
    }
}
