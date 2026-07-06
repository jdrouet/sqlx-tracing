#![cfg(feature = "sqlite")]

use sqlx::Sqlite;
use sqlx_tracing::Pool;

mod common;

#[tokio::test]
async fn execute() {
    let observability = opentelemetry_testing::ObservabilityContainer::create().await;
    let provider = observability.install().await;

    let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    let pool = sqlx_tracing::Pool::from(pool);

    common::should_trace("trace_pool", "sqlite", &observability, &provider, &pool).await;

    {
        let mut conn = pool.acquire().await.unwrap();
        common::should_trace("trace_conn", "sqlite", &observability, &provider, &mut conn).await;
    }

    {
        let mut tx: sqlx_tracing::Transaction<'_, Sqlite> = pool.begin().await.unwrap();
        common::should_trace(
            "trace_tx",
            "sqlite",
            &observability,
            &provider,
            &mut tx.executor(),
        )
        .await;
    }
}

#[test]
fn pool_sqlite_is_clone() {
    fn assert_clone<T: Clone>() {}
    assert_clone::<Pool<Sqlite>>();
}

#[cfg(feature = "offline")]
#[tokio::test]
async fn query_macro_executes_through_tracing_executors() {
    let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    let pool = sqlx_tracing::Pool::from(pool);

    let pool_row = sqlx::query!("select 1 as col1")
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert_eq!(pool_row.map(|row| row.col1), Some(1));

    let mut conn = pool.acquire().await.unwrap();
    let conn_row = sqlx::query!("select 1 as col1")
        .fetch_optional(&mut conn)
        .await
        .unwrap();
    assert_eq!(conn_row.map(|row| row.col1), Some(1));

    let mut tx: sqlx_tracing::Transaction<'_, Sqlite> = pool.begin().await.unwrap();
    let tx_row = sqlx::query!("select 1 as col1")
        .fetch_optional(&mut tx.executor())
        .await
        .unwrap();
    assert_eq!(tx_row.map(|row| row.col1), Some(1));
}
