# Distributed Tracing Setup

How to wire the CyberFabric framework to an OpenTelemetry collector (Jaeger,
Uptrace, Tempo, etc.) and what spans you get out of the box.

## What the framework gives you

Without writing any instrumentation code:

- **W3C Trace Context propagation** — incoming `traceparent` header is honored
  as parent; outgoing HTTP calls made through `modkit_http::HttpClientBuilder`
  with `.with_otel()` inject it automatically.
- **Span hierarchy for every REST request:**
  `http_request` (TraceLayer) → `handler` (route template from `MatchedPath`)
  → `auth` (jwt validation, principal) → module handler code → `db.insert` /
  `db.update` / `db.delete` / `db.txn` (SecureConn) and
  `outgoing_http` → `http.retry` (per attempt) for downstream calls.
- **Startup and shutdown spans** — `HostRuntime`'s `run_*_phase` methods each
  get a method-level span (`phase="init" | "db" | "rest" | ...`) with per-module
  child spans (`module.init`, `module.start`, `db.migrate`, `rest.register`, …).
- **Real graceful shutdown** — `SdkTracerProvider` and `SdkMeterProvider` are
  force-flushed and shut down on exit, so the final batch of buffered spans
  actually reaches the collector.
- **`tracing_error::ErrorLayer` installed** — error types that opt in via
  `tracing_error::SpanTrace::capture()` get the current span context for free.
- **Unified trace-ID extraction** — `modkit::telemetry::current_trace_id()`
  returns the real W3C trace ID of the active OTEL span (or `None`); use it in
  `From<DomainError> for Problem` impls and any other place that needs a
  `trace_id` string.

## Quick start with Jaeger

### 1. Start Jaeger

```bash
docker run -d --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  -p 4318:4318 \
  -e COLLECTOR_OTLP_ENABLED=true \
  jaegertracing/all-in-one:latest
```

Ports: `16686` = UI, `4317` = OTLP gRPC, `4318` = OTLP HTTP.

### 2. Configure tracing

The real schema nests everything under `opentelemetry`. Match the shape in
`libs/modkit/src/telemetry/config.rs` — top-level `tracing:` is not recognized.

```yaml
opentelemetry:
  resource:
    service_name: "hyperspot-api"
    attributes:
      service.version: "1.0.0"
      deployment.environment: "dev"

  # Shared exporter for both tracing and metrics.
  # Per-signal `exporter` overrides this when present.
  exporter:
    kind: "otlp_grpc"                # or "otlp_http"
    endpoint: "http://127.0.0.1:4317"
    timeout_ms: 5000

  tracing:
    enabled: true
    sampler:
      parent_based_ratio:
        ratio: 0.1                   # 10% sampling
    propagation:
      w3c_trace_context: true

  metrics:
    enabled: true
```

### 3. Run

```bash
cargo run --bin hyperspot-server -- --config config/with-tracing.yaml
```

### 4. View traces

[http://localhost:16686](http://localhost:16686) → service `hyperspot-api`.

---

## Quick start with Uptrace

[Uptrace](https://uptrace.dev) is a ClickHouse-backed OTEL UI. The Rust side
is identical — only the exporter endpoint + DSN header differ.

```yaml
opentelemetry:
  resource:
    service_name: "hyperspot-api"
    attributes:
      service.version: "1.3.7"
      deployment.environment: "dev"
      service.namespace: "hyperspot"

  exporter:
    kind: "otlp_grpc"
    endpoint: "http://127.0.0.1:14317"
    timeout_ms: 5000
    headers:
      uptrace-dsn: "http://project1_secret@localhost:14318?grpc=14317"

  tracing:
    enabled: true
    sampler:
      always_on: {}
```

Minimal Docker Compose for the backend is in the previous revision of this doc
and the Uptrace README; it hasn't changed.

---

## Configuration reference

### Exporter

| Key | Type | Notes |
|---|---|---|
| `opentelemetry.exporter.kind` | `otlp_grpc` \| `otlp_http` | Default `otlp_grpc`. |
| `opentelemetry.exporter.endpoint` | string | Defaults: `http://127.0.0.1:4317` (gRPC), `http://127.0.0.1:4318` (HTTP). |
| `opentelemetry.exporter.headers` | map<string,string> | Auth / routing headers (e.g. `authorization: Bearer …`, `uptrace-dsn: …`). Merged with `OTEL_EXPORTER_OTLP_HEADERS` env. |
| `opentelemetry.exporter.timeout_ms` | u64 | Per-export timeout. |
| `opentelemetry.tracing.exporter` | same shape | Per-signal override; fully replaces the shared exporter when set. |
| `opentelemetry.metrics.exporter` | same shape | Same, for metrics. |

### Sampler

Pick exactly one variant under `sampler`:

```yaml
opentelemetry:
  tracing:
    sampler:
      parent_based_ratio: { ratio: 0.1 }
      # or: parent_based_always_on: {}   (honor parent decision, else on)
      # or: always_on: {}
      # or: always_off: {}
```

### Propagation and HTTP options

```yaml
opentelemetry:
  tracing:
    propagation:
      w3c_trace_context: true
    http:
      inject_request_id_header: "x-request-id"
      record_headers:
        - "user-agent"
        - "x-forwarded-for"
```

`record_headers` values with sensitive content (`authorization`, cookies) are
not redacted at export time — add them at your own risk.

### Environment overrides

Any config value has an equivalent env var using the `APP__` prefix and double
underscores as separators:

```bash
export APP__OPENTELEMETRY__TRACING__ENABLED=true
export APP__OPENTELEMETRY__RESOURCE__SERVICE_NAME=hyperspot-prod
export APP__OPENTELEMETRY__EXPORTER__ENDPOINT=http://jaeger:4317
export APP__OPENTELEMETRY__TRACING__SAMPLER__PARENT_BASED_RATIO__RATIO=0.01
```

---

## Span reference

What the framework emits, by subsystem. Field names follow the
[OpenTelemetry semantic conventions](https://opentelemetry.io/docs/specs/semconv/).

| Span | Parent | Level | Key fields |
|---|---|---|---|
| `http_request` | (root) | info | `method`, `uri`, `status`, `http.method`, `http.target`, `http.status_code`, `latency_ms`, `trace_id`, `request_id` |
| `handler` | `http_request` | info | `otel.name`, `http.method`, `http.route`, `http.status_code` |
| `auth` | `handler` | info | `otel.kind="internal"`, `auth.method`, `auth.requirement`, `auth.principal`, `auth.tenant`, `auth.result` |
| `db.insert` / `db.update` / `db.delete` | caller | debug | `db.system`, `db.operation`, `entity`, `err` on error |
| `db.txn` | caller | debug | `db.system`, `otel.kind="client"` |
| `outgoing_http` | caller | info | `http.method`, `http.url`, `http.status_code`, `otel.kind="client"`, `error` on failure |
| `http.retry` | `outgoing_http` | debug | `attempt`, `http.method`, `http.url`, `http.status_code`, `error` |
| `grpc_call` | caller | debug | `op`, `attempt` (gRPC client retry) |
| `phase` (`run_*_phase`) | (root at startup) | info | `phase="init"/"db"/"rest"/"start"/...`, `err` on failure |
| `module.init` / `module.start` / `module.stop` / `db.migrate` / `rest.register` / `grpc.register` / `module.post_init` / `oop.spawn` | `phase` | info | `module` |

`handler` is produced by `modkit::api::handler_span_middleware`, wired between
the api-gateway's `TraceLayer` and the business middlewares. It requires routes
to be registered before the layer is attached (which is the case in
`rest_finalize` — the production path).

---

## Using `HttpClient` with OpenTelemetry

The `.with_otel()` builder method requires the `otel` feature:

```toml
[dependencies]
modkit-http = { workspace = true, features = ["otel"] }
```

```rust,ignore
use modkit_http::HttpClientBuilder;

let client = HttpClientBuilder::new()
    .with_otel()                                   // injects traceparent on every request
    .timeout(std::time::Duration::from_secs(30))
    .user_agent("my-service/1.0")
    .build()?;

let bytes = client
    .get("https://api.example.com/data")
    .send()
    .await?
    .checked_bytes()
    .await?;
```

`with_otel()` adds the `OtelLayer` (produces the `outgoing_http` span) and the
retry middleware layer (produces per-attempt `http.retry` spans). Both sit
between your code and the underlying `reqwest` call.

## Manual spans

For business-logic spans around work the framework doesn't see:

```rust,ignore
use tracing::{info_span, Instrument};

async fn process_user(user_id: u64) -> anyhow::Result<()> {
    async {
        // work here — any instrumented call (DB, HTTP) nests under this span.
        Ok(())
    }
    .instrument(info_span!("process_user", user.id = user_id))
    .await
}
```

Use `#[tracing::instrument(skip_all, fields(...))]` when the whole function
body is the span. Avoid `.entered()` across `.await` — it pins a thread-local
and can interleave spans under a multithreaded runtime.

---

## Field naming convention

We standardize on the OpenTelemetry semantic conventions so that spans exported
to Jaeger / Tempo / Uptrace light up the upstream UI features (service maps,
DB latency panels, error filtering) without per-service configuration.

| Concept | Legacy (still emitted) | Canonical |
|---|---|---|
| HTTP method | `method` | `http.method` |
| HTTP route (templated) | — (was `endpoint`, removed) | `http.route` |
| HTTP target (actual path) | `uri` | `http.target` |
| HTTP status | `status` | `http.status_code` |
| DB system | — | `db.system` |
| DB operation | — | `db.operation` |
| Error flag | `err` | `error` (bool) + `exception.*` on errors |
| Authenticated principal | `user_id` (ad-hoc) | `enduser.id` or `auth.principal` |
| Trace id for log correlation | `trace_id` | `trace_id` (kept — matches log pipelines) |

**New code** uses the canonical names. **Existing code** keeps legacy fields
because log pipelines, dashboards, and the `access_log` integration tests key
on those names; migrate opportunistically when a file is touched for another
reason, and record the canonical field alongside the legacy one when you do.

Two surfaces are exempt from migration in both directions:

- `modules/system/api-gateway/src/middleware/access_log.rs` — the access-log
  event is a stable pipeline contract. Do not rename its fields.
- `endpoint = …` fields on gRPC-hub / directory spans refer to a gRPC service
  address, not an HTTP route — OTEL has no semantic convention for this. Leave
  them as-is.

---

## Production

### Sampling

Default to `parent_based_ratio` in production (`ratio: 0.01` for 1% on a busy
service). Never `always_on` in prod — a chatty service will saturate the
collector ingress.

### Docker Compose with Jaeger

```yaml
services:
  jaeger:
    image: ${REGISTRY:-}jaegertracing/jaeger:${JAEGER_VERSION:-latest}
    ports:
      - "16686:16686"
      - "4317:4317"
      - "4318:4318"
    environment:
      - COLLECTOR_OTLP_ENABLED=true
```

## Troubleshooting

### No traces appearing

1. Confirm Jaeger/Uptrace is up and reachable on the configured endpoint.
2. `opentelemetry.tracing.enabled: true` — disabled by default.
3. For smoke tests use `sampler: { always_on: {} }` so nothing gets sampled out.
4. Look for `OpenTelemetry tracing initialized` at startup. If you don't see
   it, the config is being parsed into a different shape than you expect —
   check that your YAML keys actually nest under `opentelemetry`, not
   top-level `tracing`.

### Trace context not propagating

- Incoming: client must send `traceparent`; `propagation.w3c_trace_context`
  must be `true` (default).
- Outgoing: HTTP client must be built with `.with_otel()`. Plain `reqwest`
  calls won't inject.

### Final spans missing at shutdown

Since the tracer provider shutdown rewrite in #817, `shutdown_tracing()` does
force-flush + shutdown, and `bootstrap::run_server` / `bootstrap::run_migrate`
both call it on exit. If you still see truncated traces, the process is
probably not going through those entry points — a panic or
`std::process::exit()` skips destructors and drops the batch.
