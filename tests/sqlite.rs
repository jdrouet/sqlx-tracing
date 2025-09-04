#![cfg(feature = "sqlite")]

#[tokio::test]
async fn should_trace_pool() {
    let _ = tracing_subscriber::fmt::try_init();
    let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    let pool = sqlx_tracing::Pool::from(pool);
    let result: i64 = sqlx::query_scalar("select 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(result, 1);
}

#[tokio::test]
async fn should_trace_other_pool() {
    let _ = tracing_subscriber::fmt::try_init();
    let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    let pool = sqlx_tracing::Pool::from(pool);
    let result: i64 = sqlx::query_scalar("select 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(result, 1);
}
