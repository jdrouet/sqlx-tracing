use futures::{StreamExt, TryStreamExt};
use tracing::Instrument;

fn record_error(err: &sqlx::Error) {
    let span = tracing::Span::current();
    span.record("otel.status_code", "error");
    match err {
        sqlx::Error::ColumnIndexOutOfBounds { .. }
        | sqlx::Error::ColumnDecode { .. }
        | sqlx::Error::ColumnNotFound(_)
        | sqlx::Error::Decode { .. }
        | sqlx::Error::Encode { .. }
        | sqlx::Error::RowNotFound
        | sqlx::Error::TypeNotFound { .. } => {
            span.record("error.type", "client");
        }
        _ => {
            span.record("error.type", "server");
        }
    }
    span.record("error.message", err.to_string());
    span.record("error.stacktrace", format!("{err:?}"));
}

impl<'p, DB> sqlx::Executor<'p> for &'_ crate::Pool<DB>
where
    DB: sqlx::Database + crate::prelude::Database,
    for<'c> &'c mut DB::Connection: sqlx::Executor<'c, Database = DB>,
{
    type Database = DB;

    #[doc(hidden)]
    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> futures::future::BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>> {
        let span = crate::query_span!("sqlx.describe", sql);
        let fut = self.inner.describe(sql).instrument(span);
        Box::pin(async move { fut.await.inspect_err(record_error) })
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
        let fut = self.inner.execute(query).instrument(span);
        Box::pin(async move { fut.await.inspect_err(record_error) })
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
        let stream = self.inner.execute_many(query);
        use futures::StreamExt;
        Box::pin(
            stream
                .inspect(move |_| {
                    let _enter = span.enter();
                })
                .inspect_err(record_error),
        )
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
        let stream = self.inner.fetch(query);
        use futures::StreamExt;
        Box::pin(
            stream
                .inspect(move |_| {
                    let _enter = span.enter();
                })
                .inspect_err(record_error),
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
    {
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_all", sql);
        let fut = self.inner.fetch_all(query).instrument(span);
        Box::pin(async move {
            fut.await
                .inspect(|res| {
                    let span = tracing::Span::current();
                    span.record("db.response.returned_rows", res.len());
                })
                .inspect_err(record_error)
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
    {
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_all", sql);
        let stream = self.inner.fetch_many(query);
        Box::pin(
            stream
                .inspect(move |_| {
                    let _enter = span.enter();
                })
                .inspect_err(record_error),
        )
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
        let fut = self.inner.fetch_one(query).instrument(span);
        Box::pin(async move {
            fut.await
                .inspect(|_| {
                    tracing::Span::current().record("db.response.returned_rows", 1);
                })
                .inspect_err(record_error)
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
    {
        let sql = query.sql();
        let span = crate::query_span!("sqlx.fetch_optional", sql);
        let fut = self.inner.fetch_optional(query).instrument(span);
        Box::pin(async move {
            fut.await
                .inspect(|res| {
                    tracing::Span::current().record(
                        "db.response.returned_rows",
                        if res.is_some() { 1 } else { 0 },
                    );
                })
                .inspect_err(record_error)
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
        let fut = self.inner.prepare(query).instrument(span);
        Box::pin(async move { fut.await.inspect_err(record_error) })
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
        let fut = self.inner.prepare_with(sql, parameters).instrument(span);
        Box::pin(async move { fut.await.inspect_err(record_error) })
    }
}
