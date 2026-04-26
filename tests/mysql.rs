#![cfg(feature = "mysql")]

use std::time::Duration;

use sqlx::MySql;
use sqlx_tracing::Pool;
use testcontainers::{
    GenericImage, ImageExt,
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
};

mod common;

#[derive(Debug)]
struct MySqlContainer {
    container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
}

impl MySqlContainer {
    async fn create() -> Self {
        let container = GenericImage::new("mysql", "8")
            .with_wait_for(WaitFor::message_on_stderr(
                "ready for connections. Bind-address: '::' port: 3306",
            ))
            .with_exposed_port(ContainerPort::Tcp(3306))
            .with_env_var("MYSQL_ALLOW_EMPTY_PASSWORD", "yes")
            .with_env_var("MYSQL_DATABASE", "test")
            .with_startup_timeout(Duration::from_secs(120))
            .start()
            .await
            .expect("starting a mysql database");

        Self { container }
    }

    async fn client(&self) -> sqlx_tracing::Pool<MySql> {
        let port = self.container.get_host_port_ipv4(3306).await.unwrap();
        let url = format!("mysql://root@localhost:{port}/test");
        sqlx::MySqlPool::connect(&url)
            .await
            .map(sqlx_tracing::Pool::from)
            .unwrap()
    }
}

#[tokio::test]
async fn execute() {
    let observability = opentelemetry_testing::ObservabilityContainer::create().await;
    let provider = observability.install().await;

    let container = MySqlContainer::create().await;
    let pool = container.client().await;

    common::should_trace("trace_pool", "mysql", &observability, &provider, &pool).await;

    {
        let mut conn = pool.acquire().await.unwrap();
        common::should_trace("trace_conn", "mysql", &observability, &provider, &mut conn).await;
    }

    {
        let mut tx: sqlx_tracing::Transaction<'_, MySql> = pool.begin().await.unwrap();
        common::should_trace(
            "trace_tx",
            "mysql",
            &observability,
            &provider,
            &mut tx.executor(),
        )
        .await;
    }
}

#[test]
fn pool_mysql_is_clone() {
    fn assert_clone<T: Clone>() {}
    assert_clone::<Pool<MySql>>();
}
