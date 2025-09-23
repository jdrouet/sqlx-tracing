use std::time::Duration;

use opentelemetry::trace::{FutureExt, TraceContextExt, Tracer};

pub async fn should_trace_pool<DB>(
    observability: &opentelemetry_testing::ObservabilityContainer,
    provider: &opentelemetry_testing::OpenTelemetryProvider,
    pool: sqlx_tracing::Pool<DB>,
) where
    DB: sqlx::Database + sqlx_tracing::prelude::Database,
    for<'r> i32: sqlx::Type<DB> + sqlx::Decode<'r, DB>,
    for<'c> &'c mut DB::Connection: sqlx::Executor<'c, Database = DB>,
    usize: sqlx::ColumnIndex<<DB as sqlx::Database>::Row>,
    for<'c> <DB as sqlx::Database>::Arguments<'c>: sqlx::IntoArguments<'c, DB>,
{
    let scope = format!("should_trace_pool_{}", DB::SYSTEM);
    let tracer = opentelemetry::global::tracer(scope.clone());
    let span = tracer.span_builder("trace pool").start(&tracer);
    let ctx = opentelemetry::Context::new().with_span(span);
    let result: Option<i32> = sqlx::query_scalar("select 1")
        .fetch_optional(&pool)
        .with_context(ctx)
        .await
        .unwrap();
    assert_eq!(result, Some(1));

    provider.flush();

    tokio::time::sleep(Duration::from_secs(1)).await;

    let traces = observability.json_traces();
    let scope_span = traces.find_scope_span(&scope).unwrap();
    let entry = scope_span.first_span().unwrap();
    assert_eq!(entry.name, "trace pool");
    let next = traces
        .find_child(&entry.span_id, "sqlx.fetch_optional")
        .unwrap();
    assert_eq!(next.string_attribute("db.system").unwrap(), DB::SYSTEM);
    assert_eq!(next.string_attribute("db.query.text").unwrap(), "select 1");
}
