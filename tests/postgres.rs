#![cfg(feature = "postgres")]

use std::time::Duration;

use opentelemetry::trace::Tracer;
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

async fn should_trace_pool(
    observability: &opentelemetry_testing::ObservabilityContainer,
    provider: &opentelemetry_testing::OpenTelemetryProvider,
) {
    let container = PostgresContainer::create().await;
    let pool = container.client().await;

    common::should_trace_pool(observability, provider, pool).await;
}

#[tokio::test]
async fn execute() {
    let observability = opentelemetry_testing::ObservabilityContainer::create().await;
    let provider = observability.install().await;

    should_trace_pool(&observability, &provider).await;
}
