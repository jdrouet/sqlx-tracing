use sqlx::{Database, Execute, SqlStr};

/// Captures a query before `Execute::sql` consumes it, so tracing can borrow
/// its SQL without losing arguments, statement metadata, or cache preferences.
pub(crate) struct Query<DB: Database> {
    sql: SqlStr,
    statement: Option<DB::Statement>,
    arguments: Result<Option<DB::Arguments>, sqlx::error::BoxDynError>,
    persistent: bool,
}

impl<DB: Database> Query<DB> {
    pub(crate) fn new<'q>(mut query: impl Execute<'q, DB>) -> Self {
        let statement = query.statement().cloned();
        let arguments = query.take_arguments();
        let persistent = query.persistent();
        Self {
            sql: query.sql(),
            statement,
            arguments,
            persistent,
        }
    }

    pub(crate) fn sql(&self) -> &str {
        self.sql.as_str()
    }
}

impl<DB: Database> Execute<'_, DB> for Query<DB> {
    fn sql(self) -> SqlStr {
        self.sql
    }

    fn statement(&self) -> Option<&DB::Statement> {
        self.statement.as_ref()
    }

    fn take_arguments(&mut self) -> Result<Option<DB::Arguments>, sqlx::error::BoxDynError> {
        std::mem::replace(&mut self.arguments, Ok(None))
    }

    fn persistent(&self) -> bool {
        self.persistent
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;

    #[test]
    fn preserves_arguments_and_persistence() {
        let original = sqlx::query::<sqlx::Sqlite>("SELECT ?")
            .bind(42_i32)
            .persistent(false);
        let mut query = Query::new(original);
        assert_eq!(Query::sql(&query), "SELECT ?");
        assert!(!query.persistent());
        assert!(query.take_arguments().unwrap().is_some());
        assert!(query.take_arguments().unwrap().is_none());
        assert_eq!(Execute::sql(query).as_str(), "SELECT ?");
    }

    struct EncodingError;

    impl Execute<'_, sqlx::Sqlite> for EncodingError {
        fn sql(self) -> SqlStr {
            SqlStr::from_static("SELECT ?")
        }

        fn statement(&self) -> Option<&sqlx::sqlite::SqliteStatement> {
            None
        }

        fn take_arguments(
            &mut self,
        ) -> Result<Option<sqlx::sqlite::SqliteArguments>, sqlx::error::BoxDynError> {
            Err(std::io::Error::other("argument encoding failed").into())
        }

        fn persistent(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn preserves_encoding_errors() {
        let inner = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        let pool = crate::Pool::from(inner);
        let error = sqlx::Executor::fetch_one(&pool, EncodingError)
            .await
            .unwrap_err();
        match error {
            sqlx::Error::Encode(error) => {
                assert_eq!(error.to_string(), "argument encoding failed");
            }
            error => panic!("expected encoding error, got {error:?}"),
        }
    }
}
