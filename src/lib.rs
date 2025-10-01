#![doc = include_str!("../README.md")]

mod connection;
pub(crate) mod error;
pub mod macros;
mod pool;
pub mod prelude;
mod transaction;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub mod sqlite;

/// An asynchronous pool of SQLx database connections.
#[derive(Clone, Debug)]
pub struct Pool<DB>
where
    DB: sqlx::Database,
{
    inner: sqlx::Pool<DB>,
}

impl<DB> From<sqlx::Pool<DB>> for Pool<DB>
where
    DB: sqlx::Database,
{
    fn from(inner: sqlx::Pool<DB>) -> Self {
        Self { inner }
    }
}

impl<DB> Pool<DB>
where
    DB: sqlx::Database,
{
    /// Retrieves a connection and immediately begins a new transaction.
    pub async fn begin<'c>(&'c self) -> Result<Transaction<'c, DB>, sqlx::Error> {
        self.inner.begin().await.map(Transaction::from)
    }

    /// Retrieves a connection and immediately begins a new transaction.
    pub async fn acquire(&self) -> Result<PoolConnection<DB>, sqlx::Error> {
        self.inner.acquire().await.map(PoolConnection::from)
    }
}

pub struct Connection<'c, DB>
where
    DB: sqlx::Database,
{
    inner: &'c mut DB::Connection,
}

impl<'c, DB: sqlx::Database> std::fmt::Debug for Connection<'c, DB> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct PoolConnection<DB>
where
    DB: sqlx::Database,
{
    inner: sqlx::pool::PoolConnection<DB>,
}

impl<DB> From<sqlx::pool::PoolConnection<DB>> for PoolConnection<DB>
where
    DB: sqlx::Database,
{
    fn from(inner: sqlx::pool::PoolConnection<DB>) -> Self {
        Self { inner }
    }
}

/// An in-progress database transaction or savepoint.
#[derive(Debug)]
pub struct Transaction<'c, DB>
where
    DB: sqlx::Database,
{
    inner: sqlx::Transaction<'c, DB>,
}

impl<'c, DB> From<sqlx::Transaction<'c, DB>> for Transaction<'c, DB>
where
    DB: sqlx::Database,
{
    fn from(inner: sqlx::Transaction<'c, DB>) -> Self {
        Self { inner }
    }
}
