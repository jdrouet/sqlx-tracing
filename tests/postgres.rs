#![cfg(feature = "postgres")]

use std::time::Duration;

use opentelemetry::trace::{FutureExt, TraceContextExt, Tracer};
use sqlx::Postgres;
use testcontainers::{
    GenericImage, ImageExt,
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
};

mod common;

#[derive(Debug)]
struct PostgresContainer {
    container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
}

impl PostgresContainer {
    async fn create() -> Self {
        let container = GenericImage::new("postgres", "15-alpine")
            .with_wait_for(WaitFor::message_on_stderr(
                "database system is ready to accept connections",
            ))
            .with_exposed_port(ContainerPort::Tcp(5432))
            .with_env_var("POSTGRES_USER", "postgres")
            .with_env_var("POSTGRES_DB", "postgres")
            .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
            .with_startup_timeout(Duration::from_secs(60))
            .start()
            .await
            .expect("starting a postgres database");

        Self { container }
    }

    async fn client(&self) -> sqlx_tracing::Pool<Postgres> {
        let port = self.container.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres@localhost:{port}/postgres");
        sqlx::PgPool::connect(&url)
            .await
            .map(sqlx_tracing::Pool::from)
            .unwrap()
    }
}

#[tokio::test]
async fn should_trace_pool() {
    let container = common::ObservabilityContainer::create().await;
    let provider = container.install().await;

    let db_container = PostgresContainer::create().await;

    let tracer = opentelemetry::global::tracer("testing");
    let span = tracer.span_builder("trace pool").start(&tracer);
    let ctx = opentelemetry::Context::new().with_span(span);
    async move {
        let pool = db_container.client().await;
        let result: Option<i32> = sqlx::query_scalar("select 1")
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
