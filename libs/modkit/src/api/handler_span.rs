//! Automatic handler-level span for every REST request.
//!
//! Sits inside the api-gateway `TraceLayer` (so `http_request` is the parent)
//! and records `http.route` from Axum's [`MatchedPath`] extension. Handlers can
//! still stack their own `#[tracing::instrument]` on top for more granularity.
//!
//! Field naming follows OpenTelemetry semantic conventions (`http.method`,
//! `http.route`, `http.status_code`, plus the `otel.name` override so the span
//! shows up in collectors as e.g. `GET /users/{id}`).
//!
//! ## Placement requirement
//!
//! `MatchedPath` is populated by Axum's routing layer. For this middleware to
//! observe a templated route, it must run **after** routes have been matched.
//! In practice that means it should be installed via `Router::layer(...)` on a
//! router that already has its routes registered (for example in
//! `rest_finalize` rather than `rest_prepare`). If `MatchedPath` is absent the
//! middleware falls back to the raw request path, which is still useful for
//! debugging but has unbounded cardinality.
//!
//! [`MatchedPath`]: axum::extract::MatchedPath

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use tracing::{Instrument, field::Empty};

/// Middleware that wraps the downstream handler in an `info`-level `handler`
/// span.
///
/// Records `http.route` (templated path when available) plus `http.method`,
/// and fills in `http.status_code` from the outgoing response. The `otel.name`
/// override gives collectors a compact `"{METHOD} {route}"` name.
pub async fn handler_span_middleware(request: Request, next: Next) -> Response {
    let route = request.extensions().get::<MatchedPath>().map_or_else(
        || request.uri().path().to_owned(),
        |mp| mp.as_str().to_owned(),
    );

    let method = request.method().clone();

    let span = tracing::info_span!(
        "handler",
        otel.name = %format!("{method} {route}"),
        http.method = %method,
        http.route = %route,
        http.status_code = Empty,
    );

    let response = next.run(request).instrument(span.clone()).await;
    span.record("http.status_code", response.status().as_u16());
    response
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        middleware::from_fn,
        routing::get,
    };
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "hello"
    }

    async fn error_handler() -> (StatusCode, &'static str) {
        (StatusCode::IM_A_TEAPOT, "nope")
    }

    #[tokio::test]
    async fn forwards_successful_response() {
        let app = Router::new()
            .route("/items/{id}", get(ok_handler))
            .layer(from_fn(handler_span_middleware));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/items/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn preserves_downstream_status_codes() {
        let app = Router::new()
            .route("/teapot", get(error_handler))
            .layer(from_fn(handler_span_middleware));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/teapot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::IM_A_TEAPOT);
    }

    #[tokio::test]
    async fn handles_unmatched_path_without_panicking() {
        // Exercises the `MatchedPath`-absent fallback: when no route matches,
        // axum returns 404 and the middleware uses the raw request path.
        let app = Router::new()
            .route("/known", get(ok_handler))
            .layer(from_fn(handler_span_middleware));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/unknown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
