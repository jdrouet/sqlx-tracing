#![doc = include_str!("../README.md")]

pub mod macros;
mod pool;
pub mod prelude;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub mod sqlite;

/// An asynchronous pool of SQLx database connections.
#[derive(Clone, Debug)]
pub struct Pool<DB>(sqlx::Pool<DB>)
where
    DB: sqlx::Database;

impl<DB> From<sqlx::Pool<DB>> for Pool<DB>
where
    DB: sqlx::Database,
{
    fn from(value: sqlx::Pool<DB>) -> Self {
        Self(value)
    }
}

impl<DB> Pool<DB>
where
    DB: sqlx::Database,
{
    /// Retrieves a connection and immediately begins a new transaction.
    pub async fn begin<'c>(&'c self) -> Result<Transaction<'c, DB>, sqlx::Error> {
        self.0.begin().await.map(Transaction::from)
    }
}

/// An in-progress database transaction or savepoint.
#[derive(Debug)]
pub struct Transaction<'c, DB>(sqlx::Transaction<'c, DB>)
where
    DB: sqlx::Database;

impl<'c, DB> From<sqlx::Transaction<'c, DB>> for Transaction<'c, DB>
where
    DB: sqlx::Database,
{
    fn from(value: sqlx::Transaction<'c, DB>) -> Self {
        Self(value)
    }
}
