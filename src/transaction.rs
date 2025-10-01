use futures::{StreamExt, TryStreamExt};
use tracing::Instrument;

impl<'c, DB> crate::Transaction<'c, DB>
where
    DB: crate::prelude::Database + sqlx::Database,
    for<'a> &'a mut DB::Connection: sqlx::Executor<'a, Database = DB>,
{
    pub fn executor(&mut self) -> crate::Connection<'_, DB> {
        crate::Connection {
            inner: &mut *self.inner,
        }
    }
}

impl<'c, DB> sqlx::Executor<'c> for &'c mut crate::Transaction<'c, DB>
where
    DB: crate::prelude::Database + sqlx::Database,
    for<'a> &'a mut DB::Connection: sqlx::Executor<'a, Database = DB>,
{
    type Database = DB;

    #[doc(hidden)]
    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> futures::future::BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
    {
        let span = crate::query_span!("sqlx.describe", sql);
        Box::pin(async move {
            let fut = (&mut self.inner).describe(sql).instrument(span);
            fut.await.inspect_err(crate::error::record_error)
        })
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
        let sql = query.sql();
        let span = crate::query_span!("sqlx.execute", sql);
        let fut = (&mut self.inner).execute(query).instrument(span);
        Box::pin(async move { fut.await.inspect_err(crate::error::record_error) })
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
        let sql = query.sql();
        let span = crate::query_span!("sqlx.execute_many", sql);
        let stream = (&mut self.inner).execute_many(query);
        use futures::StreamExt;
        Box::pin(
            stream
                .inspect(move |_| {
                    let _enter = span.enter();
                })
                .inspect_err(crate::error::record_error),
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
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch", sql);
        let stream = (&mut self.inner).fetch(query);
        use futures::StreamExt;
        Box::pin(
            stream
                .inspect(move |_| {
                    let _enter = span.enter();
                })
                .inspect_err(crate::error::record_error),
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
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_all", sql);
        let fut = (&mut self.inner).fetch_all(query).instrument(span);
        Box::pin(async move {
            fut.await
                .inspect(|res| {
                    let span = tracing::Span::current();
                    span.record("db.response.returned_rows", res.len());
                })
                .inspect_err(crate::error::record_error)
        })
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
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_all", sql);
        let stream = (&mut self.inner).fetch_many(query);
        Box::pin(
            stream
                .inspect(move |_| {
                    let _enter = span.enter();
                })
                .inspect_err(crate::error::record_error),
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
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_one", sql);
        let fut = (&mut self.inner).fetch_one(query).instrument(span);
        Box::pin(async move {
            fut.await
                .inspect(|_| {
                    tracing::Span::current().record("db.response.returned_rows", 1);
                })
                .inspect_err(crate::error::record_error)
        })
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
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_optional", sql);
        let fut = (&mut self.inner).fetch_optional(query).instrument(span);
        Box::pin(async move {
            fut.await
                .inspect(|res| {
                    tracing::Span::current().record(
                        "db.response.returned_rows",
                        if res.is_some() { 1 } else { 0 },
                    );
                })
                .inspect_err(crate::error::record_error)
        })
    }

    fn prepare<'e, 'q: 'e>(
        self,
        query: &'q str,
    ) -> futures::future::BoxFuture<
        'e,
        Result<<Self::Database as sqlx::Database>::Statement<'q>, sqlx::Error>,
    >
    where
        'c: 'e,
    {
        let span = crate::query_span!("sqlx.prepare", query);
        let fut = (&mut self.inner).prepare(query).instrument(span);
        Box::pin(async move { fut.await.inspect_err(crate::error::record_error) })
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as sqlx::Database>::TypeInfo],
    ) -> futures::future::BoxFuture<
        'e,
        Result<<Self::Database as sqlx::Database>::Statement<'q>, sqlx::Error>,
    >
    where
        'c: 'e,
    {
        let span = crate::query_span!("sqlx.prepare_with", sql);
        let fut = (&mut self.inner)
            .prepare_with(sql, parameters)
            .instrument(span);
        Box::pin(async move { fut.await.inspect_err(crate::error::record_error) })
    }
}
