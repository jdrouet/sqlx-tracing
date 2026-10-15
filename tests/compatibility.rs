#![cfg(feature = "sqlite")]

use futures::TryStreamExt;
use sqlx::{Executor, SqlStr, Statement};

// No containers or telemetry collector are needed for these API regressions.
// Run the same SQLx 0.9 operations through every instrumented executor.
macro_rules! exercise_executor {
    ($executor:expr) => {{
        let value: i32 = sqlx::query_scalar("SELECT ?")
            .bind(42_i32)
            .persistent(false)
            .fetch_one($executor)
            .await?;
        assert_eq!(value, 42);

        let sql = String::from("SELECT ?");
        let value: Option<i32> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .bind(43_i32)
            .fetch_optional($executor)
            .await?;
        assert_eq!(value, Some(43));

        let mut builder = sqlx::QueryBuilder::<sqlx::Sqlite>::new("SELECT ");
        builder.push_bind(44_i32);
        let values: Vec<(i32,)> = builder.build_query_as().fetch_all($executor).await?;
        assert_eq!(values, vec![(44,)]);

        let values: Vec<(i32,)> = sqlx::query_as::<_, (i32,)>("SELECT ? UNION ALL SELECT ?")
            .bind(45_i32)
            .bind(46_i32)
            .fetch($executor)
            .try_collect()
            .await?;
        assert_eq!(values, vec![(45,), (46,)]);

        let statement = $executor.prepare(SqlStr::from_static("SELECT ?")).await?;
        let value: i32 = statement
            .query_scalar()
            .bind(47_i32)
            .fetch_one($executor)
            .await?;
        assert_eq!(value, 47);

        let statement = $executor
            .prepare_with(SqlStr::from_static("SELECT ?"), &[])
            .await?;
        let value: i32 = statement
            .query_scalar()
            .bind(48_i32)
            .fetch_one($executor)
            .await?;
        assert_eq!(value, 48);

        let description = $executor.describe(SqlStr::from_static("SELECT 1")).await?;
        assert_eq!(description.columns().len(), 1);

        let results: Vec<_> = $executor
            .execute_many(sqlx::raw_sql("SELECT 1; SELECT 2"))
            .try_collect()
            .await?;
        assert_eq!(results.len(), 2);

        let results: Vec<_> = $executor
            .fetch_many(sqlx::raw_sql("SELECT 1; SELECT 2"))
            .try_collect()
            .await?;
        assert_eq!(results.iter().filter(|res| res.is_right()).count(), 2);

        $executor
            .execute(sqlx::query(
                "CREATE TABLE IF NOT EXISTS compatibility (value INTEGER)",
            ))
            .await?;
        let result = $executor
            .execute(sqlx::query("INSERT INTO compatibility VALUES (?)").bind(49_i32))
            .await?;
        assert_eq!(result.rows_affected(), 1);

        assert!(
            sqlx::query("SELECT missing_column")
                .fetch_one($executor)
                .await
                .is_err()
        );
    }};
}

#[tokio::test]
async fn sqlx_09_executors() -> Result<(), sqlx::Error> {
    let inner = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(":memory:")
        .await?;
    let pool = sqlx_tracing::Pool::from(inner);

    exercise_executor!(&pool);
    {
        let mut connection = pool.acquire().await?;
        exercise_executor!(&mut connection);
    }
    {
        let mut transaction = pool.begin().await?;
        exercise_executor!(&mut transaction);
        transaction.commit().await?;
    }
    {
        let mut transaction = pool.begin().await?;
        {
            let mut connection = transaction.executor();
            exercise_executor!(&mut connection);
        }
        transaction.rollback().await?;
    }
    // Returning a dropped connection to the pool may still be in progress.
    if let Some(mut transaction) = pool.try_begin().await? {
        exercise_executor!(&mut transaction);
        transaction.rollback().await?;
    }
    Ok(())
}
