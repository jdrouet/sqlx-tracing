use futures::FutureExt;
use tracing::Instrument;

impl<'p, DB> sqlx::Executor<'p> for &'_ crate::Pool<DB>
where
    DB: sqlx::Database,
    DB: crate::prelude::Database,
    for<'c> &'c mut DB::Connection: sqlx::Executor<'c, Database = DB>,
{
    type Database = DB;

    #[doc(hidden)]
    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> futures::future::BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>> {
        let span = crate::query_span!("sqlx.describe", sql);
        self.0.describe(sql).instrument(span).boxed()
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
    {
        let sql = query.sql();
        let span = crate::query_span!("sqlx.execute", sql);
        self.0.execute(query).instrument(span).boxed()
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
    {
        let sql = query.sql();
        let span = crate::query_span!("sqlx.execute_many", sql);
        let stream = self.0.execute_many(query);
        use futures::StreamExt;
        Box::pin(stream.inspect(move |_| {
            let _enter = span.enter();
        }))
    }

    fn fetch<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::stream::BoxStream<'e, Result<<Self::Database as sqlx::Database>::Row, sqlx::Error>>
    where
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch", sql);
        let stream = self.0.fetch(query);
        use futures::StreamExt;
        Box::pin(stream.inspect(move |_| {
            let _enter = span.enter();
        }))
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
    {
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_all", sql);
        let fut = self.0.fetch_all(query).instrument(span);
        Box::pin(async move {
            fut.await.inspect(|res| {
                let span = tracing::Span::current();
                span.record("db.response.returned_rows", res.len());
            })
        })
    }

    fn fetch_many<'e, 'q: 'e, E>(
        self,
        _query: E,
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
    {
        todo!()
    }

    fn fetch_one<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::future::BoxFuture<'e, Result<<Self::Database as sqlx::Database>::Row, sqlx::Error>>
    where
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_one", sql);
        self.0.fetch_one(query).instrument(span).boxed()
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
    {
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_optional", sql);
        let fut = self.0.fetch_optional(query).instrument(span);
        Box::pin(async move {
            fut.await.inspect(|res| {
                tracing::Span::current().record(
                    "db.response.returned_rows",
                    if res.is_some() { 1 } else { 0 },
                );
            })
        })
    }

    fn prepare<'e, 'q: 'e>(
        self,
        query: &'q str,
    ) -> futures::future::BoxFuture<
        'e,
        Result<<Self::Database as sqlx::Database>::Statement<'q>, sqlx::Error>,
    > {
        let span = crate::query_span!("sqlx.prepare", query);
        self.0.prepare(query).instrument(span).boxed()
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as sqlx::Database>::TypeInfo],
    ) -> futures::future::BoxFuture<
        'e,
        Result<<Self::Database as sqlx::Database>::Statement<'q>, sqlx::Error>,
    > {
        let span = crate::query_span!("sqlx.prepare_with", sql);
        self.0
            .prepare_with(sql, parameters)
            .instrument(span)
            .boxed()
    }
}
