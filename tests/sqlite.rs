#![cfg(feature = "sqlite")]

use std::time::Duration;

use opentelemetry::trace::{FutureExt, TraceContextExt, Tracer};

mod common;

#[tokio::test]
async fn should_trace_pool() {
    let container = common::ObservabilityContainer::create().await;
    let provider = container.install().await;

    let tracer = opentelemetry::global::tracer("testing");
    let span = tracer.span_builder("trace pool").start(&tracer);
    let ctx = opentelemetry::Context::new().with_span(span);
    async move {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        let pool = sqlx_tracing::Pool::from(pool);

        let result: Option<i64> = sqlx::query_scalar("select 1")
            .fetch_optional(&pool)
            .await
            .unwrap();
        assert_eq!(result, Some(1));
    }
    .with_context(ctx)
    .await;

    provider.shutdown();

    tokio::time::sleep(Duration::from_secs(1)).await;

    let traces = container.traces();
    assert!(traces.contains("sqlx.fetch_optional"));
}
