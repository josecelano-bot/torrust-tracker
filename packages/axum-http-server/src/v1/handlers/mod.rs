//! Axum [`handlers`](axum#handlers) for the HTTP server.
pub mod announce;
pub mod health_check;
pub mod scrape;

use std::error::Error;

use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use torrust_tracker_axum_server::request_id::current_request_id;
use torrust_tracker_http_protocol::v1::responses;

use crate::HTTP_TRACKER_LOG_TARGET;

/// The reply to a request that failed, and the event that tells the operator why.
///
/// A failure inside the tracker sends the fixed sentence of `reply` with the request id added, and
/// leaves one `ERROR` event with the full text of `error` (its sources included) and the same
/// request id, so the operator finds the detail by the id a client quotes (rules 2 and 7 of the
/// client error text ADR,
/// `docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`).
/// Any other failure is the client's to fix and is logged as before, by the request layers.
pub(crate) fn failure_response(error: &dyn Error, reply: responses::error::Error) -> Response {
    let reply = if reply.is_internal() {
        let request_id = current_request_id();
        tracing::error!(target: HTTP_TRACKER_LOG_TARGET, %request_id, error = %error, "the request failed inside the tracker");
        reply.with_request_id(&request_id)
    } else {
        reply
    };

    (StatusCode::OK, reply.write()).into_response()
}
