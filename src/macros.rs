#[macro_export]
macro_rules! query_span {
    ($name:expr, $statement:expr) => {
        tracing::info_span!(
            $name,
            "db.name" = ::tracing::field::Empty,
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
            "net.peer.name" = ::tracing::field::Empty,
            "net.peer.port" = ::tracing::field::Empty,
            "otel.kind" = "client",
            "otel.status_code" = ::tracing::field::Empty,
            "otel.status_description" = ::tracing::field::Empty,
            "peer.service" = ::tracing::field::Empty,
        )
    };
}
