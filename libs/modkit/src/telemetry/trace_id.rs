//! Unified trace-ID extraction.
//!
//! When the `otel` feature is enabled, `current_trace_id()` returns the real
//! W3C trace ID of the active OpenTelemetry span. Without the feature it
//! returns `None`. Never use `tracing::Span::current().id()` for trace IDs —
//! that is a subscriber-local numeric span id, not a trace id.

use http::HeaderMap;

/// Real W3C trace ID of the active OTEL span, if any.
#[cfg(feature = "otel")]
#[must_use]
pub fn current_trace_id() -> Option<String> {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let ctx = tracing::Span::current().context();
    let span = ctx.span();
    let sc = span.span_context();
    sc.is_valid().then(|| sc.trace_id().to_string())
}

#[cfg(not(feature = "otel"))]
#[must_use]
pub fn current_trace_id() -> Option<String> {
    None
}

/// Parse the trace-id portion of a W3C `traceparent` header
/// (format: `00-<trace_id>-<span_id>-<flags>`). Per the W3C spec, `trace_id`
/// must be 32 lowercase hex characters and not all zeros; we reject anything
/// else so callers never propagate a malformed id into `Problem.trace_id` or
/// log-correlation fields.
fn parse_traceparent_trace_id(traceparent: &str) -> Option<String> {
    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() < 4 || parts[0] != "00" {
        return None;
    }
    let tid = parts[1];
    if tid.len() != 32
        || !tid
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        || tid.bytes().all(|b| b == b'0')
    {
        return None;
    }
    Some(tid.to_owned())
}

/// Best-effort trace ID: OTEL context first, then `traceparent`, then `x-request-id`.
#[must_use]
pub fn trace_id_from_request(headers: &HeaderMap) -> Option<String> {
    if let Some(id) = current_trace_id() {
        return Some(id);
    }
    if let Some(tp) = headers.get("traceparent").and_then(|v| v.to_str().ok())
        && let Some(id) = parse_traceparent_trace_id(tp)
    {
        return Some(id);
    }
    headers
        .get("x-request-id")
        .or_else(|| headers.get("x-trace-id"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_traceparent() {
        let tp = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        assert_eq!(
            parse_traceparent_trace_id(tp),
            Some("4bf92f3577b34da6a3ce929d0e0e4736".to_owned())
        );
    }

    #[test]
    fn rejects_non_v00_traceparent() {
        let tp = "ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        assert_eq!(parse_traceparent_trace_id(tp), None);
    }

    #[test]
    fn rejects_short_traceparent() {
        assert_eq!(parse_traceparent_trace_id("00-abc-def"), None);
    }

    #[test]
    fn rejects_empty_trace_id() {
        assert_eq!(parse_traceparent_trace_id("00--00f067aa0ba902b7-01"), None);
    }

    #[test]
    fn rejects_short_trace_id() {
        assert_eq!(
            parse_traceparent_trace_id("00-abc123-00f067aa0ba902b7-01"),
            None
        );
    }

    #[test]
    fn rejects_uppercase_trace_id() {
        assert_eq!(
            parse_traceparent_trace_id("00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-01"),
            None
        );
    }

    #[test]
    fn rejects_all_zero_trace_id() {
        assert_eq!(
            parse_traceparent_trace_id("00-00000000000000000000000000000000-00f067aa0ba902b7-01"),
            None
        );
    }

    #[test]
    fn trace_id_from_headers_prefers_traceparent_over_x_request_id() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
                .parse()
                .unwrap(),
        );
        headers.insert("x-request-id", "req-123".parse().unwrap());

        // Without an active OTEL span, falls back to traceparent.
        let id = trace_id_from_request(&headers);
        assert_eq!(id, Some("4bf92f3577b34da6a3ce929d0e0e4736".to_owned()));
    }

    #[test]
    fn trace_id_from_headers_uses_x_request_id_when_no_traceparent() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "req-abc".parse().unwrap());
        let id = trace_id_from_request(&headers);
        assert_eq!(id, Some("req-abc".to_owned()));
    }
}
