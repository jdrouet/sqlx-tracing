use futures::{StreamExt, TryStreamExt};
use sqlx::Error;
use tracing::Instrument;

impl<'c, DB> crate::Transaction<'c, DB>
where
    DB: crate::prelude::Database + sqlx::Database,
    for<'a> &'a mut DB::Connection: sqlx::Executor<'a, Database = DB>,
{
    /// Returns a tracing-instrumented executor for this transaction.
    ///
    /// This allows running queries with full span context and attributes.
    pub fn executor(&mut self) -> crate::Connection<'_, DB> {
        crate::Connection {
            inner: &mut *self.inner,
            attributes: self.attributes.clone(),
        }
    }

    /// Commits this transaction or savepoint.
    pub async fn commit(self) -> Result<(), Error> {
        self.inner.commit().await
    }

    /// Aborts this transaction or savepoint.
    pub async fn rollback(self) -> Result<(), Error> {
        self.inner.rollback().await
    }
}

/// Implements `sqlx::Executor` for a mutable reference to a tracing-instrumented transaction.
///
/// Each method creates a tracing span for the SQL operation, attaches relevant attributes,
/// and records errors or row counts as appropriate for observability.
impl<'c, DB> sqlx::Executor<'c> for &'c mut crate::Transaction<'c, DB>
where
    DB: crate::prelude::Database + sqlx::Database,
    for<'a> &'a mut DB::Connection: sqlx::Executor<'a, Database = DB>,
{
    type Database = DB;

    #[doc(hidden)]
    #[cfg(feature = "offline")]
    fn describe<'e>(
        self,
        sql: sqlx::SqlStr,
    ) -> futures::future::BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
    {
        let attrs = &self.attributes;
        let span = crate::span::span_for_sql::<DB>("sqlx.describe", sql.as_ref(), attrs);
        Box::pin(
            async move {
                let fut = (&mut *self.inner).describe(sql);
                fut.await.inspect_err(crate::span::record_error)
            }
            .instrument(span),
        )
    }

    fn execute<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::future::BoxFuture<
        'e,
        Result<<Self::Database as sqlx::Database>::QueryResult, sqlx::Error>,
    >
    where
        E: 'q + sqlx::Execute<'q, Self::Database>,
        'c: 'e,
    {
        let attrs = &self.attributes;
        let (query, span) = match crate::instrument!("sqlx.execute", query, attrs) {
            Ok(v) => v,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let fut = (&mut self.inner).execute(query);
        Box::pin(async move { fut.await.inspect_err(crate::span::record_error) }.instrument(span))
    }

    fn execute_many<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::stream::BoxStream<
        'e,
        Result<<Self::Database as sqlx::Database>::QueryResult, sqlx::Error>,
    >
    where
        E: 'q + sqlx::Execute<'q, Self::Database>,
        'c: 'e,
    {
        let attrs = &self.attributes;
        let (query, span) = match crate::instrument!("sqlx.execute_many", query, attrs) {
            Ok(v) => v,
            Err(e) => return Box::pin(futures::stream::once(futures::future::ready(Err(e)))),
        };
        let stream = (&mut self.inner).execute_many(query);
        Box::pin(
            stream
                .inspect(move |_| {
                    let _enter = span.enter();
                })
                .inspect_err(crate::span::record_error),
        )
    }

    fn fetch<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::stream::BoxStream<'e, Result<<Self::Database as sqlx::Database>::Row, sqlx::Error>>
    where
        E: 'q + sqlx::Execute<'q, Self::Database>,
        'c: 'e,
    {
        let attrs = &self.attributes;
        let (query, span) = match crate::instrument!("sqlx.fetch", query, attrs) {
            Ok(v) => v,
            Err(e) => return Box::pin(futures::stream::once(futures::future::ready(Err(e)))),
        };
        let stream = (&mut self.inner).fetch(query);
        Box::pin(
            stream
                .inspect(move |_| {
                    let _enter = span.enter();
                })
                .inspect_err(crate::span::record_error),
        )
    }

    fn fetch_all<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::future::BoxFuture<
        'e,
        Result<Vec<<Self::Database as sqlx::Database>::Row>, sqlx::Error>,
    >
    where
        E: 'q + sqlx::Execute<'q, Self::Database>,
        'c: 'e,
    {
        let attrs = &self.attributes;
        let (query, span) = match crate::instrument!("sqlx.fetch_all", query, attrs) {
            Ok(v) => v,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let fut = (&mut self.inner).fetch_all(query);
        Box::pin(
            async move {
                fut.await
                    .inspect(|res| {
                        let span = tracing::Span::current();
                        span.record("db.response.returned_rows", res.len());
                    })
                    .inspect_err(crate::span::record_error)
            }
            .instrument(span),
        )
    }

    fn fetch_many<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::stream::BoxStream<
        'e,
        Result<
            sqlx::Either<
                <Self::Database as sqlx::Database>::QueryResult,
                <Self::Database as sqlx::Database>::Row,
            >,
            sqlx::Error,
        >,
    >
    where
        E: 'q + sqlx::Execute<'q, Self::Database>,
        'c: 'e,
    {
        let attrs = &self.attributes;
        let (query, span) = match crate::instrument!("sqlx.fetch_all", query, attrs) {
            Ok(v) => v,
            Err(e) => return Box::pin(futures::stream::once(futures::future::ready(Err(e)))),
        };
        let stream = (&mut self.inner).fetch_many(query);
        Box::pin(
            stream
                .inspect(move |_| {
                    let _enter = span.enter();
                })
                .inspect_err(crate::span::record_error),
        )
    }

    fn fetch_one<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::future::BoxFuture<'e, Result<<Self::Database as sqlx::Database>::Row, sqlx::Error>>
    where
        E: 'q + sqlx::Execute<'q, Self::Database>,
        'c: 'e,
    {
        let attrs = &self.attributes;
        let (query, span) = match crate::instrument!("sqlx.fetch_one", query, attrs) {
            Ok(v) => v,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let fut = (&mut self.inner).fetch_one(query);
        Box::pin(
            async move {
                fut.await
                    .inspect(crate::span::record_one)
                    .inspect_err(crate::span::record_error)
            }
            .instrument(span),
        )
    }

    fn fetch_optional<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::future::BoxFuture<
        'e,
        Result<Option<<Self::Database as sqlx::Database>::Row>, sqlx::Error>,
    >
    where
        E: 'q + sqlx::Execute<'q, Self::Database>,
        'c: 'e,
    {
        let attrs = &self.attributes;
        let (query, span) = match crate::instrument!("sqlx.fetch_optional", query, attrs) {
            Ok(v) => v,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let fut = (&mut self.inner).fetch_optional(query);
        Box::pin(
            async move {
                fut.await
                    .inspect(crate::span::record_optional)
                    .inspect_err(crate::span::record_error)
            }
            .instrument(span),
        )
    }

    fn prepare<'e>(
        self,
        query: sqlx::SqlStr,
    ) -> futures::future::BoxFuture<
        'e,
        Result<<Self::Database as sqlx::Database>::Statement, sqlx::Error>,
    >
    where
        'c: 'e,
    {
        let attrs = &self.attributes;
        let span = crate::span::span_for_sql::<DB>("sqlx.prepare", query.as_ref(), attrs);
        let fut = (&mut self.inner).prepare(query);
        Box::pin(async move { fut.await.inspect_err(crate::span::record_error) }.instrument(span))
    }

    fn prepare_with<'e>(
        self,
        sql: sqlx::SqlStr,
        parameters: &'e [<Self::Database as sqlx::Database>::TypeInfo],
    ) -> futures::future::BoxFuture<
        'e,
        Result<<Self::Database as sqlx::Database>::Statement, sqlx::Error>,
    >
    where
        'c: 'e,
    {
        let attrs = &self.attributes;
        let span = crate::span::span_for_sql::<DB>("sqlx.prepare_with", sql.as_ref(), attrs);
        let fut = (&mut self.inner).prepare_with(sql, parameters);
        Box::pin(async move { fut.await.inspect_err(crate::span::record_error) }.instrument(span))
    }
}
