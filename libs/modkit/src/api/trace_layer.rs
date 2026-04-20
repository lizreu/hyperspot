//! Trace propagation utilities for Problem responses
//!
//! This module provides helper traits and functions to automatically enrich
//! `Problem` with trace context:
//! - `trace_id`: real W3C trace ID of the active OpenTelemetry span context,
//!   via `modkit::telemetry::current_trace_id()`. May be `None` when OTEL is
//!   disabled or no span context is active — in that case the field is left
//!   unset on the response rather than populated with a placeholder.
//! - `instance`: extracted from the request URI.
//!
//! This eliminates per-callsite boilerplate and ensures consistent error reporting.

use crate::api::problem::Problem;

/// Helper trait for enriching Problem with trace context
pub trait WithTraceContext {
    /// Enrich this Problem with `trace_id` and instance from the current request context
    #[must_use]
    fn with_trace_context(self, instance: impl Into<String>) -> Self;
}

impl WithTraceContext for Problem {
    fn with_trace_context(mut self, instance: impl Into<String>) -> Self {
        self = self.with_instance(instance);
        if let Some(tid) = crate::telemetry::current_trace_id() {
            self = self.with_trace_id(tid);
        }
        self
    }
}

/// Middleware-friendly: enrich errors from Axum extractors
///
/// Use this in handlers to automatically add trace context:
///
/// ```ignore
/// async fn handler(uri: Uri) -> Result<Json<Data>, Problem> {
///     let data = fetch_data()
///         .await
///         .map_err(Problem::from)
///         .map_err(|p| p.with_request_context(&uri))?;
///     Ok(Json(data))
/// }
/// ```
pub trait WithRequestContext {
    /// Add `trace_id` and instance from the current request
    #[must_use]
    fn with_request_context(self, uri: &axum::http::Uri) -> Self;
}

impl WithRequestContext for Problem {
    fn with_request_context(self, uri: &axum::http::Uri) -> Self {
        self.with_trace_context(uri.path())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_with_trace_context() {
        use http::StatusCode;

        let problem = Problem::new(StatusCode::NOT_FOUND, "Not Found", "Resource not found")
            .with_trace_context("/tests/v1/users/123");

        assert_eq!(problem.instance, "/tests/v1/users/123");
        // trace_id may or may not be set depending on tracing context
    }

    #[test]
    fn test_with_request_context() {
        use axum::http::Uri;
        use http::StatusCode;

        let uri: Uri = "/tests/v1/users/123".parse().unwrap();
        let problem = Problem::new(StatusCode::NOT_FOUND, "Not Found", "Resource not found")
            .with_request_context(&uri);

        assert_eq!(problem.instance, "/tests/v1/users/123");
    }
}
