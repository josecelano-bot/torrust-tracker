//! `Error` response for the HTTP tracker.
//!
//! Data structures and logic to build the error responses.
//!
//! From the [BEP 03. The `BitTorrent` Protocol Specification](https://www.bittorrent.org/beps/bep_0003.html):
//!
//! _"Tracker responses are bencoded dictionaries. If a tracker response has a
//! key failure reason, then that maps to a human readable string which explains
//! why the query failed, and no other keys are required."_
//!
//! > **NOTICE**: error responses are bencoded and always have a `200 OK` status
//! > code. The official `BitTorrent` specification does not specify the status
//! > code.
use serde::{Deserialize, Serialize};

use crate::v1::auth;
use crate::v1::services::peer_ip_resolver::PeerIpResolutionError;

/// `Error` response for the HTTP tracker.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Error {
    /// Human readable string which explains why the request failed.
    #[serde(rename = "failure reason")]
    pub failure_reason: String,
}

/// How the `failure reason` of a failure inside the tracker begins.
///
/// A failure inside the tracker (a database that is down, a state the code did not expect) sends
/// one fixed sentence that names the operation and nothing from the error, and the request id the
/// operator finds the detail by (rule 2 of the client error text ADR,
/// `docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`).
pub const INTERNAL_FAILURE_PREFIX: &str = "Internal tracker error: ";

impl Error {
    /// The reply for a failure inside the tracker while doing `operation` (`announce`, `scrape`
    /// or `request`). It holds nothing from the error that caused it.
    #[must_use]
    pub fn internal(operation: &str) -> Self {
        Self {
            failure_reason: format!("{INTERNAL_FAILURE_PREFIX}the {operation} could not be completed, try again later"),
        }
    }

    /// Whether this is a failure inside the tracker, as [`Error::internal`] builds it.
    #[must_use]
    pub fn is_internal(&self) -> bool {
        self.failure_reason.starts_with(INTERNAL_FAILURE_PREFIX)
    }

    /// Adds the request id to a failure inside the tracker, so the client can quote it. Any other
    /// failure is returned as it is: it tells the client what to fix, and an id would not help.
    #[must_use]
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        if self.is_internal() {
            self.failure_reason.push_str(" (request id: ");
            self.failure_reason.push_str(request_id);
            self.failure_reason.push(')');
        }
        self
    }

    /// Returns the bencoded representation of the `Error` struct.
    ///
    /// ```rust
    /// use torrust_tracker_http_protocol::v1::responses::error::Error;
    ///
    /// let err = Error {
    ///    failure_reason: "error message".to_owned(),
    /// };
    ///
    /// // cspell:disable-next-line
    /// assert_eq!(err.write(), "d14:failure reason13:error messagee");
    /// ```
    ///
    /// # Panics
    ///
    /// It would panic if the `Error` struct contained an inappropriate field
    /// type.
    #[must_use]
    pub fn write(&self) -> String {
        serde_bencode::to_string(&self).unwrap()
    }
}

impl From<auth::Error> for Error {
    fn from(err: auth::Error) -> Self {
        Self {
            failure_reason: err.to_string(),
        }
    }
}

impl From<PeerIpResolutionError> for Error {
    fn from(err: PeerIpResolutionError) -> Self {
        match err {
            // The client's request lacks the header a reverse proxy must add: the client can act.
            PeerIpResolutionError::MissingRightMostXForwardedForIp => Self {
                failure_reason: format!("Error resolving peer IP: {err}"),
            },
            // The server did not give the handler the connection address: nothing the client sent.
            PeerIpResolutionError::MissingClientIp => Self::internal("request"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::v1::responses;
    use crate::v1::services::peer_ip_resolver::PeerIpResolutionError;

    #[test]
    fn http_tracker_errors_can_be_bencoded() {
        let err = Error {
            failure_reason: "error message".to_owned(),
        };

        assert_eq!(err.write(), "d14:failure reason13:error messagee"); // cspell:disable-line
    }

    fn assert_error_response(error: &responses::error::Error, error_message: &str) {
        assert!(
            error.failure_reason.contains(error_message),
            "Error response does not contain message: '{error_message}'. Error: {error:?}"
        );
    }

    #[test]
    fn it_should_map_a_peer_ip_resolution_error_into_an_error_response() {
        let response = responses::error::Error::from(PeerIpResolutionError::MissingRightMostXForwardedForIp);

        assert_error_response(&response, "Error resolving peer IP");
    }

    #[test]
    fn it_should_treat_a_missing_client_socket_address_as_an_internal_failure_of_the_tracker() {
        // The client sent nothing wrong: the server did not give the handler the connection address.
        let response = responses::error::Error::from(PeerIpResolutionError::MissingClientIp);

        assert_eq!(
            response.failure_reason,
            "Internal tracker error: the request could not be completed, try again later"
        );
    }

    #[test]
    fn it_should_name_only_the_operation_in_an_internal_failure() {
        let response = Error::internal("announce");

        assert_eq!(
            response.failure_reason,
            "Internal tracker error: the announce could not be completed, try again later"
        );
        assert!(response.is_internal());
    }

    #[test]
    fn it_should_add_the_request_id_to_an_internal_failure() {
        let response = Error::internal("scrape").with_request_id("an-id");

        assert_eq!(
            response.failure_reason,
            "Internal tracker error: the scrape could not be completed, try again later (request id: an-id)"
        );
    }

    #[test]
    fn it_should_leave_a_failure_that_is_not_internal_without_a_request_id() {
        let response = Error {
            failure_reason: "Authentication key expired".to_owned(),
        }
        .with_request_id("an-id");

        assert_eq!(response.failure_reason, "Authentication key expired");
        assert!(!response.is_internal());
    }
}
