//! The id of the request that is being served, for the replies and events built far from the
//! request layers.
//!
//! The servers give every request an `x-request-id` header (`tower_http`'s `SetRequestIdLayer`)
//! and record it in the request span. An internal failure tells the client that id (rule 2 of the
//! client error text ADR, `docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`),
//! and the code that builds that text sits several calls below the handler and has no request in
//! hand. [`scope_from_header`] keeps the id for the duration of the request and
//! [`current_request_id`] reads it, so no handler or service signature has to carry it.
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

tokio::task_local! {
    static REQUEST_ID: String;
}

/// What [`current_request_id`] answers when no request id is in scope.
pub const UNKNOWN_REQUEST_ID: &str = "unknown";

/// The header the servers set on every request and reply.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// The id of the request being served, or [`UNKNOWN_REQUEST_ID`] outside a request.
#[must_use]
pub fn current_request_id() -> String {
    REQUEST_ID
        .try_with(Clone::clone)
        .unwrap_or_else(|_| UNKNOWN_REQUEST_ID.to_string())
}

/// Runs `future` with `request_id` as the id of the request being served.
pub async fn scope_request_id<F: std::future::Future>(request_id: String, future: F) -> F::Output {
    REQUEST_ID.scope(request_id, future).await
}

/// A middleware that scopes the `x-request-id` header of the request around the rest of the stack.
///
/// Add it inside the layer that sets the header (a layer added earlier runs later), so the header
/// exists when it runs. A request without the header is served with [`UNKNOWN_REQUEST_ID`].
pub async fn scope_from_header(request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or(UNKNOWN_REQUEST_ID)
        .to_string();

    scope_request_id(request_id, next.run(request)).await
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use tower::ServiceExt as _;

    use super::{UNKNOWN_REQUEST_ID, current_request_id, scope_from_header, scope_request_id};

    #[tokio::test]
    async fn it_should_answer_unknown_outside_a_request() {
        assert_eq!(current_request_id(), UNKNOWN_REQUEST_ID);
    }

    #[tokio::test]
    async fn it_should_answer_the_scoped_id_inside_the_scope_and_unknown_after_it() {
        let inside = scope_request_id("a-test-request-id".to_string(), async { current_request_id() }).await;

        assert_eq!(inside, "a-test-request-id");
        assert_eq!(current_request_id(), UNKNOWN_REQUEST_ID);
    }

    async fn serve(request: Request<Body>) -> String {
        let router = Router::new()
            .route("/", get(|| async { current_request_id() }))
            .layer(from_fn(scope_from_header));

        let response = router.oneshot(request).await.expect("the router should answer");

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body should be readable");
        String::from_utf8(bytes.to_vec()).expect("the body should be text")
    }

    #[tokio::test]
    async fn it_should_make_the_id_of_the_request_header_available_to_the_handler() {
        let request = Request::get("/")
            .header("x-request-id", "an-id-from-the-header")
            .body(Body::empty())
            .expect("the request should be valid");

        assert_eq!(serve(request).await, "an-id-from-the-header");
    }

    #[tokio::test]
    async fn it_should_answer_unknown_to_the_handler_when_the_request_has_no_id_header() {
        let request = Request::get("/").body(Body::empty()).expect("the request should be valid");

        assert_eq!(serve(request).await, UNKNOWN_REQUEST_ID);
    }
}
