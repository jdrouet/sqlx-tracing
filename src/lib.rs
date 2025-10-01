#![doc = include_str!("../README.md")]

use std::sync::Arc;

use sqlx::ConnectOptions;

mod connection;
mod pool;
pub mod prelude;
pub(crate) mod span;
mod transaction;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[derive(Debug, Default)]
struct Attributes {
    name: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
}

#[derive(Debug)]
pub struct PoolBuilder<DB: sqlx::Database> {
    pool: sqlx::Pool<DB>,
    attributes: Attributes,
}

impl<DB: sqlx::Database> PoolBuilder<DB> {
    pub fn new(pool: sqlx::Pool<DB>) -> Self {
        let url = pool.connect_options().to_url_lossy();
        let attributes = Attributes {
            name: None,
            host: url.host_str().map(String::from),
            port: url.port(),
            database: url
                .path_segments()
                .and_then(|mut segments| segments.next().map(String::from)),
        };
        Self { pool, attributes }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.attributes.name = Some(name.into());
        self
    }

    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.attributes.database = Some(database.into());
        self
    }

    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.attributes.host = Some(host.into());
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.attributes.port = Some(port);
        self
    }

    pub fn build(self) -> Pool<DB> {
        Pool {
            inner: self.pool,
            attributes: Arc::new(self.attributes),
        }
    }
}

/// An asynchronous pool of SQLx database connections.
#[derive(Clone, Debug)]
pub struct Pool<DB>
where
    DB: sqlx::Database,
{
    inner: sqlx::Pool<DB>,
    attributes: Arc<Attributes>,
}

impl<DB> From<sqlx::Pool<DB>> for Pool<DB>
where
    DB: sqlx::Database,
{
    fn from(inner: sqlx::Pool<DB>) -> Self {
        PoolBuilder::new(inner).build()
    }
}

impl<DB> Pool<DB>
where
    DB: sqlx::Database,
{
    /// Retrieves a connection and immediately begins a new transaction.
    pub async fn begin<'c>(&'c self) -> Result<Transaction<'c, DB>, sqlx::Error> {
        self.inner.begin().await.map(|inner| Transaction {
            inner,
            attributes: self.attributes.clone(),
        })
    }

    /// Retrieves a connection and immediately begins a new transaction.
    pub async fn acquire(&self) -> Result<PoolConnection<DB>, sqlx::Error> {
        self.inner.acquire().await.map(|inner| PoolConnection {
            attributes: self.attributes.clone(),
            inner,
        })
    }
}

pub struct Connection<'c, DB>
where
    DB: sqlx::Database,
{
    inner: &'c mut DB::Connection,
    attributes: Arc<Attributes>,
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
    attributes: Arc<Attributes>,
}

/// An in-progress database transaction or savepoint.
#[derive(Debug)]
pub struct Transaction<'c, DB>
where
    DB: sqlx::Database,
{
    inner: sqlx::Transaction<'c, DB>,
    attributes: Arc<Attributes>,
}
