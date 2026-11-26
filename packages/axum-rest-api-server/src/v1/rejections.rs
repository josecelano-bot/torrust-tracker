//! What a client reads when the request does not match what an endpoint expects: a body that is
//! not the JSON of the endpoint, a body without the JSON content type, a query parameter that does
//! not parse.
//!
//! Axum answers these with the text of its own extractors, which wrap the text of `serde`. That text
//! is a library's, not the tracker's (rule 4 of the client error text ADR,
//! `docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`),
//! so [`ApiRejection`] replaces it: one sentence that names the field or the parameter and what is
//! expected. API v1 keeps the statuses axum chose (`400`, `415`, `422`) and the plain text body.
//!
//! The name of the field comes from the path prefix that axum puts before the `serde` text
//! (`key: invalid type...`); when the text has none, the reply names the whole body.
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::v1::responses::echo;

/// The body of `POST /api/v1/keys`, the only endpoint that reads a JSON body.
const KEY_BODY_TEXT: &str =
    "Invalid request body: expected a JSON object with the fields key (string or null) and seconds_valid (number or null)";

/// The rejection of an extractor, as the client reads it. Use it with
/// `axum_extra::extract::WithRejection`.
#[derive(Debug)]
pub struct ApiRejection {
    status: StatusCode,
    text: String,
}

impl ApiRejection {
    fn new(status: StatusCode, text: impl Into<String>) -> Self {
        Self {
            status,
            text: text.into(),
        }
    }
}

impl IntoResponse for ApiRejection {
    fn into_response(self) -> Response {
        (self.status, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], self.text).into_response()
    }
}

impl From<JsonRejection> for ApiRejection {
    fn from(rejection: JsonRejection) -> Self {
        match &rejection {
            JsonRejection::JsonDataError(_) => field_path(&rejection.body_text(), "the target type: ").map_or_else(
                || Self::new(rejection.status(), KEY_BODY_TEXT),
                |field| {
                    Self::new(
                        rejection.status(),
                        format!("Invalid request body: invalid value for the field {}", echo(&field)),
                    )
                },
            ),
            JsonRejection::JsonSyntaxError(_) => Self::new(rejection.status(), KEY_BODY_TEXT),
            JsonRejection::MissingJsonContentType(_) => Self::new(
                rejection.status(),
                "Invalid request: the body must be sent with Content-Type: application/json",
            ),
            _ => Self::new(rejection.status(), "Invalid request body"),
        }
    }
}

impl From<QueryRejection> for ApiRejection {
    fn from(rejection: QueryRejection) -> Self {
        invalid_query(rejection.status(), &rejection.body_text())
    }
}

impl From<axum_extra::extract::QueryRejection> for ApiRejection {
    fn from(rejection: axum_extra::extract::QueryRejection) -> Self {
        invalid_query(rejection.status(), &rejection.body_text())
    }
}

fn invalid_query(status: StatusCode, library_text: &str) -> ApiRejection {
    field_path(library_text, "query string: ").map_or_else(
        || ApiRejection::new(status, "Invalid query string"),
        |parameter| {
            ApiRejection::new(
                status,
                format!(
                    "Invalid query parameter {}: expected {}",
                    echo(&parameter),
                    expected_form(&parameter)
                ),
            )
        },
    )
}

/// What the endpoints of the API expect for a query parameter, by its name.
fn expected_form(parameter: &str) -> &'static str {
    match parameter {
        "offset" | "limit" => "a non-negative integer",
        "format" => "json or prometheus",
        "info_hash" => "a 40 character long string",
        _ => "a valid value",
    }
}

/// The path that axum prints after `marker` in its rejection text, for example `key` in
/// `... the target type: key: invalid type: ...`; `None` when the text has no path there.
fn field_path(library_text: &str, marker: &str) -> Option<String> {
    let after_marker = library_text.split_once(marker)?.1;
    let (path, _) = after_marker.split_once(": ")?;

    let is_a_path = !path.is_empty()
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '[' | ']'));

    is_a_path.then(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::extract::Query;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use axum_extra::extract::WithRejection;
    use rstest::rstest;
    use serde::Deserialize;
    use tower::ServiceExt as _;

    use super::ApiRejection;

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct Form {
        key: String,
        seconds_valid: Option<u64>,
    }

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct Params {
        offset: Option<u32>,
    }

    async fn answer(request: Request<Body>) -> (StatusCode, String) {
        let router = Router::new()
            .route(
                "/json",
                post(|WithRejection(_, _): WithRejection<axum::Json<Form>, ApiRejection>| async {}),
            )
            .route(
                "/query",
                get(|WithRejection(_, _): WithRejection<Query<Params>, ApiRejection>| async {}),
            );

        let response = router.oneshot(request).await.expect("the router should answer");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("a readable body");

        (status, String::from_utf8(bytes.to_vec()).expect("a text body"))
    }

    fn json_post(body: &'static str, content_type: Option<&'static str>) -> Request<Body> {
        let mut builder = Request::post("/json");
        if let Some(content_type) = content_type {
            builder = builder.header("content-type", content_type);
        }
        builder.body(Body::from(body)).expect("a valid request")
    }

    const KEY_BODY: &str =
        "Invalid request body: expected a JSON object with the fields key (string or null) and seconds_valid (number or null)";

    #[rstest]
    #[case::malformed_json(json_post("{bad", Some("application/json")), StatusCode::BAD_REQUEST, KEY_BODY)]
    #[case::a_value_of_the_wrong_type(
        json_post(r#"{"key":1}"#, Some("application/json")),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Invalid request body: invalid value for the field key"
    )]
    #[case::a_second_value_of_the_wrong_type(
        json_post(r#"{"key":"k","seconds_valid":"x"}"#, Some("application/json")),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Invalid request body: invalid value for the field seconds_valid"
    )]
    #[case::a_missing_field(
        json_post(r#"{"seconds_valid":5}"#, Some("application/json")),
        StatusCode::UNPROCESSABLE_ENTITY,
        KEY_BODY
    )]
    #[case::no_content_type(
        json_post(r#"{"key":null}"#, None),
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Invalid request: the body must be sent with Content-Type: application/json"
    )]
    #[tokio::test]
    async fn it_should_replace_the_text_of_a_json_rejection_with_one_sentence(
        #[case] request: Request<Body>,
        #[case] status: StatusCode,
        #[case] expected: &str,
    ) {
        assert_eq!(answer(request).await, (status, expected.to_string()));
    }

    #[tokio::test]
    async fn it_should_name_the_query_parameter_and_what_it_expects() {
        let request = Request::get("/query?offset=abc")
            .body(Body::empty())
            .expect("a valid request");

        assert_eq!(
            answer(request).await,
            (
                StatusCode::BAD_REQUEST,
                "Invalid query parameter offset: expected a non-negative integer".to_string()
            )
        );
    }
}
