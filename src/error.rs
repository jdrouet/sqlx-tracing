pub(crate) fn record_error(err: &sqlx::Error) {
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
