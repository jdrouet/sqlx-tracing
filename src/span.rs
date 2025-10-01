#[macro_export]
macro_rules! instrument {
    ($name:expr, $statement:expr, $attributes:expr) => {
        tracing::info_span!(
            $name,
            "db.name" = $attributes.database,
            "db.operation" = ::tracing::field::Empty,
            "db.query.text" = $statement,
            "db.response.affected_rows" = ::tracing::field::Empty,
            "db.response.returned_rows" = ::tracing::field::Empty,
            "db.response.status_code" = ::tracing::field::Empty,
            "db.sql.table" = ::tracing::field::Empty,
            "db.system.name" = DB::SYSTEM,
            "error.type" = ::tracing::field::Empty,
            "error.message" = ::tracing::field::Empty,
            "error.stacktrace" = ::tracing::field::Empty,
            "net.peer.name" = $attributes.host,
            "net.peer.port" = $attributes.port,
            "otel.kind" = "client",
            "otel.status_code" = ::tracing::field::Empty,
            "otel.status_description" = ::tracing::field::Empty,
            "peer.service" = $attributes.name,
        )
    };
}

pub fn record_one<T>(_value: &T) {
    let span = tracing::Span::current();
    span.record("db.response.returned_rows", 1);
}

pub fn record_optional<T>(value: &Option<T>) {
    let span = tracing::Span::current();
    span.record(
        "db.response.returned_rows",
        if value.is_some() { 1 } else { 0 },
    );
}

pub fn record_error(err: &sqlx::Error) {
    let span = tracing::Span::current();
    span.record("otel.status_code", "error");
    span.record("otel.status_description", err.to_string());
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
