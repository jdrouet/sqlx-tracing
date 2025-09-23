#![cfg(feature = "sqlite")]

use opentelemetry::trace::{TraceContextExt, Tracer};

mod common;

async fn should_trace_pool(
    observability: &opentelemetry_testing::ObservabilityContainer,
    provider: &opentelemetry_testing::OpenTelemetryProvider,
) {
    let tracer = opentelemetry::global::tracer("should_trace_pool");
    let span = tracer.span_builder("trace pool").start(&tracer);
    let ctx = opentelemetry::Context::new().with_span(span);

    let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    let pool = sqlx_tracing::Pool::from(pool);
    common::should_trace_pool(observability, provider, pool).await;
}

#[tokio::test]
async fn execute() {
    let observability = opentelemetry_testing::ObservabilityContainer::create().await;
    let provider = observability.install().await;

    should_trace_pool(&observability, &provider).await;
}
