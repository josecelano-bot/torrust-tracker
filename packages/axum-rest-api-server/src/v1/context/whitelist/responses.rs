//! API responses for the [`whitelist`](crate::v1::context::whitelist)
//! API context.
use axum::response::Response;
use torrust_tracker_rest_api_protocol::v1::context::whitelist::resources::whitelist::WhitelistError;

use crate::v1::responses::{internal_failure_response, unhandled_rejection_response};

/// `500` error response when a torrent cannot be removed from the whitelist.
#[must_use]
pub fn failed_to_remove_torrent_from_whitelist_response(e: &WhitelistError) -> Response {
    failed("remove torrent from whitelist", e)
}

/// `500` error response when a torrent cannot be added to the whitelist.
#[must_use]
pub fn failed_to_whitelist_torrent_response(e: &WhitelistError) -> Response {
    failed("whitelist torrent", e)
}

/// `500` error response when the whitelist cannot be reloaded from the database.
#[must_use]
pub fn failed_to_reload_whitelist_response(e: &WhitelistError) -> Response {
    failed("reload whitelist", e)
}

/// A capability that the configuration disables is a refusal and says so, as before. Any other
/// failure is inside the tracker: the reply names the operation and the request id only.
fn failed(operation: &str, error: &WhitelistError) -> Response {
    match error {
        WhitelistError::DisabledByConfiguration { .. } => unhandled_rejection_response(format!("failed to {operation}: {error}")),
        _ => internal_failure_response(operation, error),
    }
}
